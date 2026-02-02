"""Identity linking and WebFinger service for ActivityPub <-> Botcash bridge.

Implements:
- WebFinger discovery (RFC 7033)
- Actor resolution
- Identity linking between ActivityPub actors and Botcash addresses
"""

import base64
import hashlib
from datetime import datetime, timedelta, timezone
from typing import Any
from urllib.parse import urlparse

import structlog
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import padding, rsa
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from .activitypub_types import (
    AP_ACCEPT_HEADER,
    Actor,
    OrderedCollection,
    create_actor,
    extract_instance_domain,
    parse_actor,
)
from .botcash_client import BotcashClient
from .models import LinkedIdentity, LinkStatus, PrivacyMode, RemoteActor

logger = structlog.get_logger()

# Challenge validity period
CHALLENGE_EXPIRY_MINUTES = 10


class IdentityLinkError(Exception):
    """Error during identity linking process."""
    pass


class ActorNotFoundError(Exception):
    """Remote actor not found."""
    pass


def generate_rsa_keypair() -> tuple[str, str]:
    """Generate RSA key pair for HTTP signatures.

    Returns:
        Tuple of (public_key_pem, private_key_pem)
    """
    private_key = rsa.generate_private_key(
        public_exponent=65537,
        key_size=2048,
    )

    private_pem = private_key.private_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption(),
    ).decode()

    public_pem = private_key.public_key().public_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PublicFormat.SubjectPublicKeyInfo,
    ).decode()

    return public_pem, private_pem


class IdentityService:
    """Service for managing identity links between ActivityPub and Botcash."""

    def __init__(
        self,
        botcash_client: BotcashClient,
        base_url: str,
        domain: str,
    ):
        """Initialize identity service.

        Args:
            botcash_client: Botcash RPC client
            base_url: ActivityPub server base URL (e.g., https://botcash.social)
            domain: Domain for actor handles (e.g., botcash.social)
        """
        self.botcash = botcash_client
        self.base_url = base_url.rstrip("/")
        self.domain = domain

    # === Local Actor Management ===

    async def get_or_create_actor(
        self,
        session: AsyncSession,
        botcash_address: str,
    ) -> LinkedIdentity:
        """Get or create a local actor for a Botcash address.

        This creates an ActivityPub actor representation for any Botcash address,
        allowing it to be discovered and followed by Fediverse users.

        Args:
            session: Database session
            botcash_address: Botcash address

        Returns:
            LinkedIdentity with actor information

        Raises:
            IdentityLinkError: If address is invalid
        """
        # Check if actor already exists
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.botcash_address == botcash_address,
            )
        )
        existing = result.scalar_one_or_none()

        if existing:
            return existing

        # Validate Botcash address
        if not await self.botcash.validate_address(botcash_address):
            raise IdentityLinkError(f"Invalid Botcash address: {botcash_address}")

        # Create local part from address
        local_part = self._address_to_local_part(botcash_address)
        actor_id = f"{self.base_url}/users/{local_part}"

        # Generate RSA key pair for HTTP signatures
        public_key_pem, private_key_pem = generate_rsa_keypair()

        # Create new identity (auto-active for Botcash addresses)
        identity = LinkedIdentity(
            actor_id=actor_id,
            actor_local_part=local_part,
            actor_preferred_username=local_part,
            botcash_address=botcash_address,
            status=LinkStatus.ACTIVE,
            public_key_pem=public_key_pem,
            private_key_pem=private_key_pem,
            linked_at=datetime.now(timezone.utc),
        )
        session.add(identity)
        await session.commit()

        logger.info(
            "Created actor for Botcash address",
            botcash_address=botcash_address,
            actor_id=actor_id,
        )

        return identity

    async def get_actor_by_local_part(
        self,
        session: AsyncSession,
        local_part: str,
    ) -> LinkedIdentity | None:
        """Get actor by local part (username).

        Args:
            session: Database session
            local_part: Local part of actor handle (e.g., "bs1abc123")

        Returns:
            LinkedIdentity if found, None otherwise
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.actor_local_part == local_part,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        return result.scalar_one_or_none()

    async def get_actor_by_address(
        self,
        session: AsyncSession,
        botcash_address: str,
    ) -> LinkedIdentity | None:
        """Get actor by Botcash address.

        Args:
            session: Database session
            botcash_address: Botcash address

        Returns:
            LinkedIdentity if found, None otherwise
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.botcash_address == botcash_address,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        return result.scalar_one_or_none()

    def build_actor_object(self, identity: LinkedIdentity) -> Actor:
        """Build ActivityPub Actor object from identity.

        Args:
            identity: LinkedIdentity record

        Returns:
            Actor object
        """
        return create_actor(
            base_url=self.base_url,
            botcash_address=identity.botcash_address,
            display_name=identity.actor_preferred_username or identity.actor_local_part,
            summary=f"Botcash user {identity.botcash_address}",
            public_key_pem=identity.public_key_pem or "",
        )

    # === WebFinger Support ===

    async def webfinger_lookup(
        self,
        session: AsyncSession,
        resource: str,
    ) -> dict[str, Any] | None:
        """Perform WebFinger lookup for a resource.

        Args:
            session: Database session
            resource: Resource URI (e.g., acct:bs1abc@botcash.social)

        Returns:
            WebFinger JRD document or None if not found
        """
        # Parse resource
        if resource.startswith("acct:"):
            # acct:user@domain format
            acct = resource[5:]
            if "@" not in acct:
                return None
            local_part, domain = acct.rsplit("@", 1)
        elif resource.startswith("https://"):
            # Actor URL format
            if not resource.startswith(self.base_url):
                return None
            # Extract local part from URL
            path = resource[len(self.base_url):]
            if path.startswith("/users/"):
                local_part = path[7:]
            else:
                return None
            domain = self.domain
        else:
            return None

        # Check domain matches
        if domain != self.domain:
            return None

        # Look up actor
        identity = await self.get_actor_by_local_part(session, local_part)
        if not identity:
            # Try to create from Botcash address
            # Assume local_part is a truncated address
            # In practice, we'd need a lookup service
            return None

        actor_url = f"{self.base_url}/users/{local_part}"

        return {
            "subject": f"acct:{local_part}@{self.domain}",
            "aliases": [
                actor_url,
            ],
            "links": [
                {
                    "rel": "self",
                    "type": "application/activity+json",
                    "href": actor_url,
                },
                {
                    "rel": "http://webfinger.net/rel/profile-page",
                    "type": "text/html",
                    "href": actor_url,
                },
            ],
        }

    # === Remote Actor Resolution ===

    async def fetch_remote_actor(
        self,
        session: AsyncSession,
        actor_id: str,
        http_client: Any,
    ) -> RemoteActor:
        """Fetch and cache a remote ActivityPub actor.

        Args:
            session: Database session
            actor_id: Full actor ID URL
            http_client: HTTP client (aiohttp or httpx)

        Returns:
            RemoteActor record

        Raises:
            ActorNotFoundError: If actor cannot be fetched
        """
        # Check cache first
        result = await session.execute(
            select(RemoteActor).where(RemoteActor.actor_id == actor_id)
        )
        existing = result.scalar_one_or_none()

        # Refresh if cached data is older than 24 hours
        if existing:
            age = datetime.now(timezone.utc) - existing.fetched_at.replace(tzinfo=timezone.utc)
            if age < timedelta(hours=24):
                return existing

        # Fetch actor document
        try:
            async with http_client.get(
                actor_id,
                headers={
                    "Accept": AP_ACCEPT_HEADER,
                    "User-Agent": "BotcashActivityPubBridge/1.0",
                },
            ) as response:
                if response.status != 200:
                    raise ActorNotFoundError(f"Failed to fetch actor: HTTP {response.status}")
                data = await response.json()
        except Exception as e:
            raise ActorNotFoundError(f"Failed to fetch actor: {e}")

        # Parse actor data
        actor = parse_actor(data)
        if not actor or not actor.inbox:
            raise ActorNotFoundError("Invalid actor document")

        # Extract public key
        public_key_id = ""
        public_key_pem = ""
        if actor.public_key:
            public_key_id = actor.public_key.id
            public_key_pem = actor.public_key.public_key_pem

        instance_domain = extract_instance_domain(actor_id)
        handle = f"@{actor.preferred_username}@{instance_domain}"

        if existing:
            # Update existing record
            existing.preferred_username = actor.preferred_username
            existing.display_name = actor.name
            existing.summary = actor.summary
            existing.inbox_url = actor.inbox
            existing.outbox_url = actor.outbox
            existing.public_key_id = public_key_id
            existing.public_key_pem = public_key_pem
            existing.fetched_at = datetime.now(timezone.utc)
            await session.commit()
            return existing
        else:
            # Create new record
            remote_actor = RemoteActor(
                actor_id=actor_id,
                instance_domain=instance_domain,
                handle=handle,
                preferred_username=actor.preferred_username,
                display_name=actor.name,
                summary=actor.summary,
                avatar_url=actor.icon.get("url") if actor.icon else None,
                inbox_url=actor.inbox,
                outbox_url=actor.outbox,
                shared_inbox_url=data.get("endpoints", {}).get("sharedInbox"),
                public_key_id=public_key_id,
                public_key_pem=public_key_pem,
            )
            session.add(remote_actor)
            await session.commit()

            logger.info(
                "Cached remote actor",
                actor_id=actor_id,
                handle=handle,
            )

            return remote_actor

    async def get_remote_actor(
        self,
        session: AsyncSession,
        actor_id: str,
    ) -> RemoteActor | None:
        """Get cached remote actor.

        Args:
            session: Database session
            actor_id: Full actor ID URL

        Returns:
            RemoteActor if cached, None otherwise
        """
        result = await session.execute(
            select(RemoteActor).where(RemoteActor.actor_id == actor_id)
        )
        return result.scalar_one_or_none()

    # === Identity Linking for Remote Users ===

    async def initiate_remote_link(
        self,
        session: AsyncSession,
        actor_id: str,
        botcash_address: str,
    ) -> tuple[str, str]:
        """Initiate identity linking for a remote Fediverse user.

        Creates a challenge that the remote user must include in a post
        mentioning the bridge account.

        Args:
            session: Database session
            actor_id: Remote actor ID (e.g., https://mastodon.social/users/alice)
            botcash_address: Botcash address to link

        Returns:
            Tuple of (challenge, verification_message)

        Raises:
            IdentityLinkError: If address is invalid or already linked
        """
        # Validate Botcash address
        if not await self.botcash.validate_address(botcash_address):
            raise IdentityLinkError(f"Invalid Botcash address: {botcash_address}")

        # Check if address is already linked to another actor
        existing_by_address = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.botcash_address == botcash_address,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        if existing_by_address.scalar_one_or_none():
            raise IdentityLinkError("This Botcash address is already linked to another account")

        # Generate challenge
        challenge = self.botcash.generate_challenge()
        expires_at = datetime.now(timezone.utc) + timedelta(minutes=CHALLENGE_EXPIRY_MINUTES)

        # For remote users, we store a pending link record
        # The local_part is derived from the remote actor ID hash
        local_part = self._actor_id_to_local_part(actor_id)

        # Check for existing pending link
        existing_pending = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.actor_id == actor_id,
            )
        )
        existing = existing_pending.scalar_one_or_none()

        if existing:
            if existing.status == LinkStatus.ACTIVE:
                raise IdentityLinkError("This actor is already linked. Unlink first.")
            existing.botcash_address = botcash_address
            existing.challenge = challenge
            existing.challenge_expires_at = expires_at
            existing.status = LinkStatus.PENDING
        else:
            identity = LinkedIdentity(
                actor_id=actor_id,
                actor_local_part=local_part,
                botcash_address=botcash_address,
                status=LinkStatus.PENDING,
                challenge=challenge,
                challenge_expires_at=expires_at,
            )
            session.add(identity)

        await session.commit()

        # Generate verification message
        verification_msg = (
            f"I am linking my Fediverse account to Botcash address {botcash_address}.\n"
            f"Challenge: {challenge}\n"
            f"Mention @bridge@{self.domain} with this message to verify."
        )

        return challenge, verification_msg

    async def complete_remote_link(
        self,
        session: AsyncSession,
        actor_id: str,
        challenge: str,
    ) -> LinkedIdentity:
        """Complete identity linking after challenge verification.

        Called when the bridge receives a mention containing the challenge.

        Args:
            session: Database session
            actor_id: Remote actor ID
            challenge: Challenge string from the mention

        Returns:
            The activated LinkedIdentity

        Raises:
            IdentityLinkError: If verification fails
        """
        # Get pending link
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.actor_id == actor_id,
                LinkedIdentity.status == LinkStatus.PENDING,
            )
        )
        identity = result.scalar_one_or_none()

        if not identity:
            raise IdentityLinkError("No pending link found. Initiate link first.")

        # Verify challenge
        if identity.challenge != challenge:
            raise IdentityLinkError("Invalid challenge")

        # Check expiry
        if identity.challenge_expires_at:
            expires_at = identity.challenge_expires_at
            if expires_at.tzinfo is None:
                expires_at = expires_at.replace(tzinfo=timezone.utc)
            if datetime.now(timezone.utc) > expires_at:
                raise IdentityLinkError("Challenge expired. Please start over.")

        # Create on-chain bridge link
        link_result = await self.botcash.create_bridge_link(
            botcash_address=identity.botcash_address,
            platform="activitypub",
            platform_id=actor_id,
            proof=challenge,
        )

        if not link_result.success:
            raise IdentityLinkError(f"Failed to create on-chain link: {link_result.error}")

        # Activate identity
        identity.status = LinkStatus.ACTIVE
        identity.link_tx_id = link_result.tx_id
        identity.linked_at = datetime.now(timezone.utc)
        identity.challenge = None
        identity.challenge_expires_at = None

        await session.commit()

        logger.info(
            "Remote identity linked",
            actor_id=actor_id,
            botcash_address=identity.botcash_address,
            tx_id=link_result.tx_id,
        )

        return identity

    async def unlink(
        self,
        session: AsyncSession,
        actor_id: str,
    ) -> bool:
        """Unlink an ActivityPub actor from Botcash address.

        Args:
            session: Database session
            actor_id: Actor ID to unlink

        Returns:
            True if unlinked, False if no link found
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.actor_id == actor_id,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        identity = result.scalar_one_or_none()

        if not identity:
            return False

        identity.status = LinkStatus.UNLINKED
        identity.unlinked_at = datetime.now(timezone.utc)
        await session.commit()

        logger.info(
            "Identity unlinked",
            actor_id=actor_id,
            botcash_address=identity.botcash_address,
        )

        return True

    async def set_privacy_mode(
        self,
        session: AsyncSession,
        actor_id: str,
        mode: PrivacyMode,
    ) -> bool:
        """Set user's privacy mode.

        Args:
            session: Database session
            actor_id: Actor ID
            mode: Privacy mode to set

        Returns:
            True if updated, False if no link found
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.actor_id == actor_id,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        identity = result.scalar_one_or_none()

        if not identity:
            return False

        identity.privacy_mode = mode
        await session.commit()

        return True

    # === Helper Methods ===

    def _address_to_local_part(self, botcash_address: str) -> str:
        """Convert Botcash address to actor local part.

        Truncates address for readability while maintaining uniqueness.

        Args:
            botcash_address: Full Botcash address

        Returns:
            Local part suitable for actor handle
        """
        # Use first 20 chars of address (unique enough, readable)
        return botcash_address[:20] if len(botcash_address) > 20 else botcash_address

    def _actor_id_to_local_part(self, actor_id: str) -> str:
        """Create local part from remote actor ID.

        Used for remote users who link their Fediverse account.

        Args:
            actor_id: Full actor ID URL

        Returns:
            Local part (hash-based for uniqueness)
        """
        # Use hash of actor ID for uniqueness
        hash_bytes = hashlib.sha256(actor_id.encode()).digest()
        # Base64 encode and take first 16 chars
        encoded = base64.urlsafe_b64encode(hash_bytes).decode()[:16]
        return f"fed_{encoded}"
