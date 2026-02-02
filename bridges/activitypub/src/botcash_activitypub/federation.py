"""Federation handlers for ActivityPub Inbox/Outbox.

Implements:
- Inbox: Receive activities from remote servers
- Outbox: Serve activities from local actors
- HTTP Signatures for authentication
- Activity delivery to remote inboxes
"""

import asyncio
import base64
import hashlib
import json
import time
from datetime import datetime, timezone
from typing import Any
from urllib.parse import urlparse

import aiohttp
import structlog
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import padding
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from .activitypub_types import (
    AP_ACCEPT_HEADER,
    AP_CONTENT_TYPE,
    AS_PUBLIC,
    Activity,
    ActivityType,
    OrderedCollection,
    OrderedCollectionPage,
)
from .botcash_client import BotcashClient
from .identity import IdentityService
from .models import (
    Follower,
    Following,
    LinkedIdentity,
    LinkStatus,
    RelayedMessage,
    RemoteActor,
    StoredActivity,
)
from .protocol_mapper import MappedMessage, ProtocolMapper

logger = structlog.get_logger()


class FederationError(Exception):
    """Error during federation operations."""
    pass


class SignatureVerificationError(Exception):
    """Error verifying HTTP signature."""
    pass


def compute_digest(body: bytes) -> str:
    """Compute SHA-256 digest of request body.

    Args:
        body: Request body bytes

    Returns:
        Base64-encoded digest with algorithm prefix
    """
    digest = hashlib.sha256(body).digest()
    return f"SHA-256={base64.b64encode(digest).decode()}"


def create_signature_string(
    method: str,
    path: str,
    headers: dict[str, str],
    signed_headers: list[str],
) -> str:
    """Create the string to sign for HTTP signatures.

    Args:
        method: HTTP method (lowercase)
        path: Request path (with query string)
        headers: All request headers
        signed_headers: Headers to include in signature

    Returns:
        Signature string
    """
    lines = []
    for header in signed_headers:
        if header == "(request-target)":
            lines.append(f"(request-target): {method.lower()} {path}")
        else:
            value = headers.get(header, "")
            lines.append(f"{header}: {value}")
    return "\n".join(lines)


def sign_request(
    private_key_pem: str,
    key_id: str,
    method: str,
    url: str,
    headers: dict[str, str],
    body: bytes | None = None,
) -> str:
    """Create HTTP Signature header for request.

    Args:
        private_key_pem: RSA private key in PEM format
        key_id: Public key ID (actor#main-key)
        method: HTTP method
        url: Full URL
        headers: Request headers (will be mutated to add Date, Digest)
        body: Optional request body

    Returns:
        Signature header value
    """
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric import padding

    # Load private key
    private_key = serialization.load_pem_private_key(
        private_key_pem.encode(),
        password=None,
    )

    # Parse URL for path
    parsed = urlparse(url)
    path = parsed.path
    if parsed.query:
        path += f"?{parsed.query}"

    # Add Date header if not present
    if "date" not in headers and "Date" not in headers:
        headers["Date"] = datetime.now(timezone.utc).strftime("%a, %d %b %Y %H:%M:%S GMT")

    # Add Digest if body present
    if body:
        headers["Digest"] = compute_digest(body)

    # Add Host header
    headers["Host"] = parsed.netloc

    # Headers to sign
    signed_headers = ["(request-target)", "host", "date"]
    if body:
        signed_headers.append("digest")

    # Create signature string
    sig_string = create_signature_string(
        method=method,
        path=path,
        headers={k.lower(): v for k, v in headers.items()},
        signed_headers=signed_headers,
    )

    # Sign with RSA-SHA256
    signature = private_key.sign(
        sig_string.encode(),
        padding.PKCS1v15(),
        hashes.SHA256(),
    )

    sig_b64 = base64.b64encode(signature).decode()

    return (
        f'keyId="{key_id}",'
        f'algorithm="rsa-sha256",'
        f'headers="{" ".join(signed_headers)}",'
        f'signature="{sig_b64}"'
    )


class FederationService:
    """Service for handling ActivityPub federation."""

    def __init__(
        self,
        identity_service: IdentityService,
        protocol_mapper: ProtocolMapper,
        botcash_client: BotcashClient,
        base_url: str,
        domain: str,
    ):
        """Initialize federation service.

        Args:
            identity_service: Identity service for actor management
            protocol_mapper: Protocol mapper for message translation
            botcash_client: Botcash RPC client
            base_url: Server base URL
            domain: Server domain
        """
        self.identity = identity_service
        self.mapper = protocol_mapper
        self.botcash = botcash_client
        self.base_url = base_url.rstrip("/")
        self.domain = domain
        self._http_session: aiohttp.ClientSession | None = None

    async def _get_http_session(self) -> aiohttp.ClientSession:
        """Get or create HTTP session."""
        if self._http_session is None or self._http_session.closed:
            self._http_session = aiohttp.ClientSession()
        return self._http_session

    async def close(self) -> None:
        """Close HTTP session."""
        if self._http_session and not self._http_session.closed:
            await self._http_session.close()

    # === Inbox Handler ===

    async def handle_inbox(
        self,
        session: AsyncSession,
        actor_local_part: str,
        activity_data: dict[str, Any],
        signature_verified: bool = False,
    ) -> dict[str, Any]:
        """Handle incoming activity to actor's inbox.

        Args:
            session: Database session
            actor_local_part: Local part of receiving actor
            activity_data: Incoming activity JSON
            signature_verified: Whether HTTP signature was verified

        Returns:
            Response data

        Raises:
            FederationError: If processing fails
        """
        activity_type = activity_data.get("type", "")
        activity_id = activity_data.get("id", "")
        actor_id = activity_data.get("actor", "")

        logger.info(
            "Processing inbox activity",
            type=activity_type,
            activity_id=activity_id,
            from_actor=actor_id,
            to_actor=actor_local_part,
        )

        # Get local actor
        local_identity = await self.identity.get_actor_by_local_part(session, actor_local_part)
        if not local_identity:
            raise FederationError(f"Unknown actor: {actor_local_part}")

        # Store activity for audit
        await self._store_activity(session, activity_data, from_botcash=False)

        # Route by activity type
        if activity_type == ActivityType.FOLLOW.value:
            return await self._handle_follow(session, local_identity, activity_data)
        elif activity_type == ActivityType.UNDO.value:
            return await self._handle_undo(session, local_identity, activity_data)
        elif activity_type == ActivityType.CREATE.value:
            return await self._handle_create(session, local_identity, activity_data)
        elif activity_type == ActivityType.LIKE.value:
            return await self._handle_like(session, local_identity, activity_data)
        elif activity_type == ActivityType.ANNOUNCE.value:
            return await self._handle_announce(session, local_identity, activity_data)
        elif activity_type == ActivityType.ACCEPT.value:
            return await self._handle_accept(session, local_identity, activity_data)
        elif activity_type == ActivityType.REJECT.value:
            return await self._handle_reject(session, local_identity, activity_data)
        else:
            logger.debug("Ignoring unsupported activity type", type=activity_type)
            return {"status": "ignored", "reason": f"unsupported type: {activity_type}"}

    async def _handle_follow(
        self,
        session: AsyncSession,
        local_identity: LinkedIdentity,
        activity_data: dict[str, Any],
    ) -> dict[str, Any]:
        """Handle incoming Follow activity."""
        actor_id = activity_data.get("actor", "")
        activity_id = activity_data.get("id", "")

        # Fetch and cache remote actor
        http_session = await self._get_http_session()
        remote_actor = await self.identity.fetch_remote_actor(
            session, actor_id, http_session
        )

        # Check if already following
        existing = await session.execute(
            select(Follower).where(
                Follower.identity_id == local_identity.id,
                Follower.remote_actor_id == remote_actor.id,
            )
        )
        if existing.scalar_one_or_none():
            return {"status": "already_following"}

        # Create follower record
        follower = Follower(
            identity_id=local_identity.id,
            remote_actor_id=remote_actor.id,
            follow_activity_id=activity_id,
            status="accepted",  # Auto-accept for now
            accepted_at=datetime.now(timezone.utc),
        )
        session.add(follower)
        await session.commit()

        # Send Accept activity
        await self._send_accept(session, local_identity, activity_data, remote_actor)

        # Create follow on Botcash chain (optional - for tracking)
        # Note: This creates a "follower" record in Botcash social layer
        # The Botcash address can choose to follow back

        logger.info(
            "Accepted follow",
            from_actor=actor_id,
            to_actor=local_identity.actor_id,
        )

        return {"status": "accepted"}

    async def _handle_undo(
        self,
        session: AsyncSession,
        local_identity: LinkedIdentity,
        activity_data: dict[str, Any],
    ) -> dict[str, Any]:
        """Handle incoming Undo activity."""
        actor_id = activity_data.get("actor", "")
        obj = activity_data.get("object", {})

        if isinstance(obj, str):
            # Object is just an ID
            return {"status": "ignored", "reason": "undo object is ID only"}

        obj_type = obj.get("type", "")

        if obj_type == ActivityType.FOLLOW.value:
            # Undo Follow = Unfollow
            remote_actor = await self.identity.get_remote_actor(session, actor_id)
            if not remote_actor:
                return {"status": "ignored", "reason": "unknown actor"}

            # Remove follower record
            result = await session.execute(
                select(Follower).where(
                    Follower.identity_id == local_identity.id,
                    Follower.remote_actor_id == remote_actor.id,
                )
            )
            follower = result.scalar_one_or_none()
            if follower:
                await session.delete(follower)
                await session.commit()

            logger.info(
                "Processed unfollow",
                from_actor=actor_id,
                to_actor=local_identity.actor_id,
            )

            return {"status": "unfollowed"}

        return {"status": "ignored", "reason": f"unsupported undo: {obj_type}"}

    async def _handle_create(
        self,
        session: AsyncSession,
        local_identity: LinkedIdentity,
        activity_data: dict[str, Any],
    ) -> dict[str, Any]:
        """Handle incoming Create activity (post/reply).

        If the Create mentions this actor, we may want to relay it.
        """
        # Map to Botcash message
        mapped = self.mapper.activitypub_to_botcash(activity_data)
        if not mapped:
            return {"status": "ignored", "reason": "unmappable content"}

        # Check if this is a reply to one of our posts
        # or a mention of our actor - those could trigger relay

        # For now, just log it
        logger.info(
            "Received Create activity",
            from_actor=activity_data.get("actor"),
            message_type=mapped.message_type,
        )

        # Store as relayed message (inbound)
        relayed = RelayedMessage(
            identity_id=local_identity.id,
            direction="ap_to_bc",
            ap_activity_id=activity_data.get("id"),
            ap_object_id=mapped.metadata.get("ap_object_id"),
            message_type=mapped.message_type,
            content_hash=self.mapper.compute_content_hash(mapped.content),
        )
        session.add(relayed)
        await session.commit()

        return {"status": "accepted"}

    async def _handle_like(
        self,
        session: AsyncSession,
        local_identity: LinkedIdentity,
        activity_data: dict[str, Any],
    ) -> dict[str, Any]:
        """Handle incoming Like activity."""
        # Map to Botcash upvote
        mapped = self.mapper.activitypub_to_botcash(activity_data)
        if not mapped:
            return {"status": "ignored"}

        logger.info(
            "Received Like",
            from_actor=activity_data.get("actor"),
            target=activity_data.get("object"),
        )

        return {"status": "accepted"}

    async def _handle_announce(
        self,
        session: AsyncSession,
        local_identity: LinkedIdentity,
        activity_data: dict[str, Any],
    ) -> dict[str, Any]:
        """Handle incoming Announce (boost) activity."""
        logger.info(
            "Received Announce",
            from_actor=activity_data.get("actor"),
            target=activity_data.get("object"),
        )

        return {"status": "accepted"}

    async def _handle_accept(
        self,
        session: AsyncSession,
        local_identity: LinkedIdentity,
        activity_data: dict[str, Any],
    ) -> dict[str, Any]:
        """Handle Accept activity (e.g., follow accepted)."""
        obj = activity_data.get("object", {})
        if isinstance(obj, str):
            obj = {"id": obj}

        obj_type = obj.get("type", "")

        if obj_type == ActivityType.FOLLOW.value:
            # Our follow was accepted
            target_actor = activity_data.get("actor", "")

            result = await session.execute(
                select(Following).join(RemoteActor).where(
                    Following.identity_id == local_identity.id,
                    RemoteActor.actor_id == target_actor,
                )
            )
            following = result.scalar_one_or_none()
            if following:
                following.status = "accepted"
                following.accepted_at = datetime.now(timezone.utc)
                await session.commit()

            logger.info(
                "Follow accepted",
                by_actor=target_actor,
                for_actor=local_identity.actor_id,
            )

        return {"status": "accepted"}

    async def _handle_reject(
        self,
        session: AsyncSession,
        local_identity: LinkedIdentity,
        activity_data: dict[str, Any],
    ) -> dict[str, Any]:
        """Handle Reject activity (e.g., follow rejected)."""
        obj = activity_data.get("object", {})
        if isinstance(obj, str):
            obj = {"id": obj}

        obj_type = obj.get("type", "")

        if obj_type == ActivityType.FOLLOW.value:
            target_actor = activity_data.get("actor", "")

            result = await session.execute(
                select(Following).join(RemoteActor).where(
                    Following.identity_id == local_identity.id,
                    RemoteActor.actor_id == target_actor,
                )
            )
            following = result.scalar_one_or_none()
            if following:
                following.status = "rejected"
                await session.commit()

            logger.info(
                "Follow rejected",
                by_actor=target_actor,
                for_actor=local_identity.actor_id,
            )

        return {"status": "accepted"}

    # === Outbox Handler ===

    async def get_outbox(
        self,
        session: AsyncSession,
        actor_local_part: str,
        page: int | None = None,
        page_size: int = 20,
    ) -> dict[str, Any]:
        """Get actor's outbox collection.

        Args:
            session: Database session
            actor_local_part: Local part of actor
            page: Page number (None for collection root)
            page_size: Items per page

        Returns:
            OrderedCollection or OrderedCollectionPage as dict
        """
        identity = await self.identity.get_actor_by_local_part(session, actor_local_part)
        if not identity:
            raise FederationError(f"Unknown actor: {actor_local_part}")

        outbox_url = f"{self.base_url}/users/{actor_local_part}/outbox"

        # Count total activities
        total = await session.scalar(
            select(StoredActivity).where(
                StoredActivity.actor_id == identity.actor_id,
                StoredActivity.from_botcash == True,
            ).count()
        )

        if page is None:
            # Return collection root
            collection = OrderedCollection(
                id=outbox_url,
                total_items=total or 0,
                first=f"{outbox_url}?page=1",
            )
            return collection.to_dict()

        # Return page
        offset = (page - 1) * page_size
        result = await session.execute(
            select(StoredActivity).where(
                StoredActivity.actor_id == identity.actor_id,
                StoredActivity.from_botcash == True,
            ).order_by(StoredActivity.received_at.desc())
            .offset(offset)
            .limit(page_size)
        )
        activities = result.scalars().all()

        items = [json.loads(a.activity_json) for a in activities]

        collection_page = OrderedCollectionPage(
            id=f"{outbox_url}?page={page}",
            part_of=outbox_url,
            items=items,
        )

        if len(items) == page_size:
            collection_page.next = f"{outbox_url}?page={page + 1}"
        if page > 1:
            collection_page.prev = f"{outbox_url}?page={page - 1}"

        return collection_page.to_dict()

    # === Followers/Following Collections ===

    async def get_followers(
        self,
        session: AsyncSession,
        actor_local_part: str,
        page: int | None = None,
        page_size: int = 20,
    ) -> dict[str, Any]:
        """Get actor's followers collection."""
        identity = await self.identity.get_actor_by_local_part(session, actor_local_part)
        if not identity:
            raise FederationError(f"Unknown actor: {actor_local_part}")

        followers_url = f"{self.base_url}/users/{actor_local_part}/followers"

        # Count followers
        total = await session.scalar(
            select(Follower).where(
                Follower.identity_id == identity.id,
                Follower.status == "accepted",
            ).count()
        )

        if page is None:
            collection = OrderedCollection(
                id=followers_url,
                total_items=total or 0,
                first=f"{followers_url}?page=1",
            )
            return collection.to_dict()

        # Return page of follower actor IDs
        offset = (page - 1) * page_size
        result = await session.execute(
            select(RemoteActor.actor_id)
            .join(Follower)
            .where(
                Follower.identity_id == identity.id,
                Follower.status == "accepted",
            )
            .offset(offset)
            .limit(page_size)
        )
        actor_ids = [row[0] for row in result.fetchall()]

        collection_page = OrderedCollectionPage(
            id=f"{followers_url}?page={page}",
            part_of=followers_url,
            items=actor_ids,
        )

        if len(actor_ids) == page_size:
            collection_page.next = f"{followers_url}?page={page + 1}"
        if page > 1:
            collection_page.prev = f"{followers_url}?page={page - 1}"

        return collection_page.to_dict()

    async def get_following(
        self,
        session: AsyncSession,
        actor_local_part: str,
        page: int | None = None,
        page_size: int = 20,
    ) -> dict[str, Any]:
        """Get actor's following collection."""
        identity = await self.identity.get_actor_by_local_part(session, actor_local_part)
        if not identity:
            raise FederationError(f"Unknown actor: {actor_local_part}")

        following_url = f"{self.base_url}/users/{actor_local_part}/following"

        total = await session.scalar(
            select(Following).where(
                Following.identity_id == identity.id,
                Following.status == "accepted",
            ).count()
        )

        if page is None:
            collection = OrderedCollection(
                id=following_url,
                total_items=total or 0,
                first=f"{following_url}?page=1",
            )
            return collection.to_dict()

        offset = (page - 1) * page_size
        result = await session.execute(
            select(RemoteActor.actor_id)
            .join(Following)
            .where(
                Following.identity_id == identity.id,
                Following.status == "accepted",
            )
            .offset(offset)
            .limit(page_size)
        )
        actor_ids = [row[0] for row in result.fetchall()]

        collection_page = OrderedCollectionPage(
            id=f"{following_url}?page={page}",
            part_of=following_url,
            items=actor_ids,
        )

        if len(actor_ids) == page_size:
            collection_page.next = f"{following_url}?page={page + 1}"
        if page > 1:
            collection_page.prev = f"{following_url}?page={page - 1}"

        return collection_page.to_dict()

    # === Activity Delivery ===

    async def deliver_activity(
        self,
        session: AsyncSession,
        activity: Activity,
        identity: LinkedIdentity,
        target_inboxes: list[str],
    ) -> list[tuple[str, bool, str | None]]:
        """Deliver activity to remote inboxes.

        Args:
            session: Database session
            activity: Activity to deliver
            identity: Sending actor's identity
            target_inboxes: List of inbox URLs

        Returns:
            List of (inbox_url, success, error_message) tuples
        """
        results: list[tuple[str, bool, str | None]] = []

        if not identity.private_key_pem:
            raise FederationError("Actor has no private key for signing")

        key_id = f"{identity.actor_id}#main-key"
        activity_json = json.dumps(activity.to_dict())
        body = activity_json.encode()

        http_session = await self._get_http_session()

        for inbox_url in target_inboxes:
            try:
                headers = {
                    "Content-Type": AP_CONTENT_TYPE,
                    "Accept": AP_ACCEPT_HEADER,
                    "User-Agent": "BotcashActivityPubBridge/1.0",
                }

                signature = sign_request(
                    private_key_pem=identity.private_key_pem,
                    key_id=key_id,
                    method="POST",
                    url=inbox_url,
                    headers=headers,
                    body=body,
                )
                headers["Signature"] = signature

                async with http_session.post(
                    inbox_url,
                    data=body,
                    headers=headers,
                ) as response:
                    if response.status in (200, 201, 202, 204):
                        results.append((inbox_url, True, None))
                        logger.info("Delivered activity", inbox=inbox_url, status=response.status)
                    else:
                        error = await response.text()
                        results.append((inbox_url, False, f"HTTP {response.status}: {error[:100]}"))
                        logger.warning("Failed to deliver", inbox=inbox_url, status=response.status)

            except Exception as e:
                results.append((inbox_url, False, str(e)))
                logger.error("Delivery error", inbox=inbox_url, error=str(e))

        return results

    async def broadcast_to_followers(
        self,
        session: AsyncSession,
        activity: Activity,
        identity: LinkedIdentity,
    ) -> int:
        """Broadcast activity to all followers.

        Args:
            session: Database session
            activity: Activity to broadcast
            identity: Sending actor's identity

        Returns:
            Number of successful deliveries
        """
        # Get all follower inboxes
        result = await session.execute(
            select(RemoteActor.inbox_url, RemoteActor.shared_inbox_url)
            .join(Follower)
            .where(
                Follower.identity_id == identity.id,
                Follower.status == "accepted",
            )
        )

        # Use shared inboxes where available (more efficient)
        inboxes: set[str] = set()
        for row in result.fetchall():
            inbox, shared_inbox = row
            inboxes.add(shared_inbox or inbox)

        if not inboxes:
            return 0

        results = await self.deliver_activity(session, activity, identity, list(inboxes))
        success_count = sum(1 for _, success, _ in results if success)

        return success_count

    # === Accept/Reject Helpers ===

    async def _send_accept(
        self,
        session: AsyncSession,
        identity: LinkedIdentity,
        follow_activity: dict[str, Any],
        remote_actor: RemoteActor,
    ) -> None:
        """Send Accept activity for Follow."""
        accept_activity = Activity(
            id=f"{identity.actor_id}/activities/{int(time.time() * 1000)}",
            type=ActivityType.ACCEPT,
            actor=identity.actor_id,
            object=follow_activity,
            to=[remote_actor.actor_id],
        )

        await self.deliver_activity(
            session,
            accept_activity,
            identity,
            [remote_actor.inbox_url],
        )

    # === Storage ===

    async def _store_activity(
        self,
        session: AsyncSession,
        activity_data: dict[str, Any],
        from_botcash: bool = False,
        botcash_tx_id: str | None = None,
    ) -> StoredActivity:
        """Store activity for audit and retry."""
        activity_id = activity_data.get("id", f"urn:uuid:{int(time.time() * 1000)}")

        # Check for existing
        result = await session.execute(
            select(StoredActivity).where(StoredActivity.activity_id == activity_id)
        )
        existing = result.scalar_one_or_none()
        if existing:
            return existing

        stored = StoredActivity(
            activity_id=activity_id,
            activity_type=activity_data.get("type", "Unknown"),
            actor_id=activity_data.get("actor", ""),
            activity_json=json.dumps(activity_data),
            object_id=activity_data.get("object", {}).get("id") if isinstance(activity_data.get("object"), dict) else None,
            from_botcash=from_botcash,
            botcash_tx_id=botcash_tx_id,
        )
        session.add(stored)
        await session.commit()

        return stored
