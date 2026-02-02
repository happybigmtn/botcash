"""Nostr WebSocket relay server for the Botcash bridge.

Implements NIP-01 (basic protocol), NIP-04 (encrypted DMs), and partial NIP-57 (zaps).
The relay translates between Nostr events and Botcash transactions.
"""

import asyncio
import json
from dataclasses import dataclass, field
from typing import Any

import structlog
import websockets
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker
from websockets.server import WebSocketServerProtocol

from .botcash_client import BotcashClient
from .identity import IdentityService
from .models import (
    LinkedIdentity,
    LinkStatus,
    PrivacyMode,
    RateLimitEntry,
    RelayedMessage,
    StoredEvent,
)
from .nostr_types import NostrEvent, NostrFilter, NostrKind
from .protocol_mapper import ProtocolMapper

logger = structlog.get_logger()


@dataclass
class Subscription:
    """Active subscription for a client."""
    subscription_id: str
    filters: list[NostrFilter]


@dataclass
class ClientConnection:
    """State for a connected WebSocket client."""
    websocket: WebSocketServerProtocol
    subscriptions: dict[str, Subscription] = field(default_factory=dict)
    pubkey: str | None = None  # If client authenticated via NIP-42


class NostrRelay:
    """Nostr WebSocket relay server with Botcash bridge functionality.

    The relay accepts Nostr events and either:
    1. Stores them locally for relay to other Nostr clients
    2. Translates them to Botcash transactions (if author is linked)
    3. Receives Botcash posts and creates corresponding Nostr events
    """

    def __init__(
        self,
        session_maker: async_sessionmaker,
        botcash_client: BotcashClient,
        identity_service: IdentityService,
        protocol_mapper: ProtocolMapper,
        allowed_kinds: list[int] | None = None,
        rate_limit_per_minute: int = 30,
    ):
        """Initialize the relay.

        Args:
            session_maker: SQLAlchemy async session maker
            botcash_client: Botcash RPC client
            identity_service: Identity linking service
            protocol_mapper: Protocol translation mapper
            allowed_kinds: List of allowed Nostr event kinds
            rate_limit_per_minute: Max events per pubkey per minute
        """
        self.session_maker = session_maker
        self.botcash = botcash_client
        self.identity = identity_service
        self.mapper = protocol_mapper
        self.allowed_kinds = allowed_kinds or [0, 1, 3, 4, 7, 9734, 9735]
        self.rate_limit = rate_limit_per_minute

        self.clients: dict[WebSocketServerProtocol, ClientConnection] = {}
        self._lock = asyncio.Lock()

    async def handle_connection(self, websocket: WebSocketServerProtocol) -> None:
        """Handle a WebSocket connection.

        Args:
            websocket: The WebSocket connection
        """
        client = ClientConnection(websocket=websocket)
        self.clients[websocket] = client

        logger.info("Client connected", remote=websocket.remote_address)

        try:
            async for message in websocket:
                await self._handle_message(client, message)
        except websockets.exceptions.ConnectionClosed:
            pass
        finally:
            del self.clients[websocket]
            logger.info("Client disconnected", remote=websocket.remote_address)

    async def _handle_message(self, client: ClientConnection, message: str) -> None:
        """Handle an incoming WebSocket message.

        Nostr relay protocol (NIP-01):
        - ["EVENT", <event>] - Publish event
        - ["REQ", <sub_id>, <filter>...] - Subscribe to events
        - ["CLOSE", <sub_id>] - Close subscription

        Args:
            client: Client connection state
            message: Raw WebSocket message
        """
        try:
            data = json.loads(message)
        except json.JSONDecodeError:
            await self._send_notice(client, "Invalid JSON")
            return

        if not isinstance(data, list) or len(data) < 2:
            await self._send_notice(client, "Invalid message format")
            return

        cmd = data[0]

        if cmd == "EVENT":
            await self._handle_event(client, data[1])
        elif cmd == "REQ":
            await self._handle_req(client, data[1], data[2:])
        elif cmd == "CLOSE":
            await self._handle_close(client, data[1])
        else:
            await self._send_notice(client, f"Unknown command: {cmd}")

    async def _handle_event(self, client: ClientConnection, event_data: dict) -> None:
        """Handle an EVENT message (publish event).

        Args:
            client: Client connection
            event_data: Event dictionary
        """
        try:
            event = NostrEvent.from_dict(event_data)
        except Exception as e:
            await self._send_ok(client, "", False, f"Invalid event: {e}")
            return

        # Validate event ID
        if not event.is_valid_id:
            await self._send_ok(client, event.id, False, "Invalid event ID")
            return

        # Check allowed kinds
        if event.kind not in self.allowed_kinds:
            await self._send_ok(client, event.id, False, f"Kind {event.kind} not allowed")
            return

        # Rate limiting
        async with self.session_maker() as session:
            if not await self._check_rate_limit(session, event.pubkey):
                await self._send_ok(client, event.id, False, "Rate limit exceeded")
                return

            # Store event
            stored = await self._store_event(session, event)
            if not stored:
                await self._send_ok(client, event.id, False, "Duplicate event")
                return

            # Bridge to Botcash if author is linked
            await self._bridge_to_botcash(session, event)

        # Broadcast to subscribers
        await self._broadcast_event(event)

        await self._send_ok(client, event.id, True, "")

    async def _handle_req(
        self,
        client: ClientConnection,
        sub_id: str,
        filters_data: list[dict],
    ) -> None:
        """Handle a REQ message (subscription request).

        Args:
            client: Client connection
            sub_id: Subscription ID
            filters_data: List of filter dictionaries
        """
        filters = [NostrFilter.from_dict(f) for f in filters_data]

        # Store subscription
        client.subscriptions[sub_id] = Subscription(
            subscription_id=sub_id,
            filters=filters,
        )

        # Send matching stored events
        async with self.session_maker() as session:
            events = await self._query_stored_events(session, filters)
            for event in events:
                await self._send_event(client, sub_id, event)

        # Send EOSE (End of Stored Events)
        await self._send(client, ["EOSE", sub_id])

    async def _handle_close(self, client: ClientConnection, sub_id: str) -> None:
        """Handle a CLOSE message (close subscription).

        Args:
            client: Client connection
            sub_id: Subscription ID
        """
        if sub_id in client.subscriptions:
            del client.subscriptions[sub_id]

    async def _store_event(self, session: AsyncSession, event: NostrEvent) -> bool:
        """Store an event in the database.

        Args:
            session: Database session
            event: Nostr event

        Returns:
            True if stored (new event), False if duplicate
        """
        # Check for duplicate
        result = await session.execute(
            select(StoredEvent).where(StoredEvent.event_id == event.id)
        )
        if result.scalar_one_or_none():
            return False

        # Store event
        stored = StoredEvent(
            event_id=event.id,
            pubkey=event.pubkey,
            kind=event.kind,
            created_at=event.created_at,
            content=event.content,
            tags_json=json.dumps(event.tags),
            sig=event.sig,
            from_botcash=False,
        )
        session.add(stored)
        await session.commit()
        return True

    async def _query_stored_events(
        self,
        session: AsyncSession,
        filters: list[NostrFilter],
        limit: int = 100,
    ) -> list[NostrEvent]:
        """Query stored events matching filters.

        Args:
            session: Database session
            filters: List of filters (OR'd together)
            limit: Maximum events to return

        Returns:
            List of matching events
        """
        # Build query for each filter and combine
        events = []

        for filter_ in filters:
            query = select(StoredEvent)

            if filter_.ids:
                query = query.where(StoredEvent.event_id.in_(filter_.ids))
            if filter_.authors:
                query = query.where(StoredEvent.pubkey.in_(filter_.authors))
            if filter_.kinds:
                query = query.where(StoredEvent.kind.in_(filter_.kinds))
            if filter_.since:
                query = query.where(StoredEvent.created_at >= filter_.since)
            if filter_.until:
                query = query.where(StoredEvent.created_at <= filter_.until)

            query = query.order_by(StoredEvent.created_at.desc())
            if filter_.limit:
                query = query.limit(min(filter_.limit, limit))
            else:
                query = query.limit(limit)

            result = await session.execute(query)
            for stored in result.scalars():
                event = NostrEvent(
                    id=stored.event_id,
                    pubkey=stored.pubkey,
                    kind=stored.kind,
                    created_at=stored.created_at,
                    content=stored.content,
                    tags=json.loads(stored.tags_json),
                    sig=stored.sig,
                )
                # Check tag filters (can't do in SQL easily)
                if all(filter_.matches(event) for filter_ in filters if filter_.tags):
                    events.append(event)

        return events

    async def _bridge_to_botcash(self, session: AsyncSession, event: NostrEvent) -> None:
        """Bridge a Nostr event to Botcash if the author is linked.

        Args:
            session: Database session
            event: Nostr event to bridge
        """
        # Check if author has linked identity
        identity = await self.identity.get_linked_identity(session, event.pubkey)
        if not identity:
            return

        # Check privacy mode
        if identity.privacy_mode == PrivacyMode.READ_ONLY:
            return
        if identity.privacy_mode == PrivacyMode.PRIVATE and event.kind != NostrKind.ENCRYPTED_DM:
            return

        # Map to Botcash message
        mapped = self.mapper.nostr_to_botcash(event)
        if not mapped:
            return

        # Create Botcash transaction based on message type
        result = None

        if mapped.message_type == "post":
            result = await self.botcash.create_post(
                from_address=identity.botcash_address,
                content=mapped.content,
                tags=mapped.metadata.get("tags"),
            )
        elif mapped.message_type == "reply":
            # Need to resolve reply_to event ID to Botcash tx ID
            reply_to_tx = await self._resolve_event_to_tx(session, mapped.reply_to)
            if reply_to_tx:
                result = await self.botcash.create_reply(
                    from_address=identity.botcash_address,
                    content=mapped.content,
                    reply_to_tx=reply_to_tx,
                )
        elif mapped.message_type == "dm":
            # Resolve recipient pubkey to Botcash address
            recipient_pubkey = mapped.metadata.get("recipient_pubkey")
            if recipient_pubkey:
                recipient_identity = await self.identity.get_linked_identity(
                    session, recipient_pubkey
                )
                if recipient_identity:
                    result = await self.botcash.send_dm(
                        from_address=identity.botcash_address,
                        to_address=recipient_identity.botcash_address,
                        content=mapped.content,
                    )
        elif mapped.message_type in ("upvote", "downvote"):
            target_tx = await self._resolve_event_to_tx(
                session, mapped.metadata.get("target_event_id")
            )
            if target_tx:
                result = await self.botcash.upvote(
                    from_address=identity.botcash_address,
                    target_tx=target_tx,
                )
        elif mapped.message_type == "tip":
            recipient_pubkey = mapped.metadata.get("recipient_pubkey")
            if recipient_pubkey:
                recipient_identity = await self.identity.get_linked_identity(
                    session, recipient_pubkey
                )
                if recipient_identity:
                    amount_bcash = mapped.metadata.get("amount_bcash", 0)
                    amount_zatoshis = int(amount_bcash * 100_000_000)
                    target_tx = await self._resolve_event_to_tx(
                        session, mapped.metadata.get("target_event_id")
                    )
                    result = await self.botcash.tip(
                        from_address=identity.botcash_address,
                        to_address=recipient_identity.botcash_address,
                        amount_zatoshis=amount_zatoshis,
                        target_tx=target_tx,
                    )

        # Record relayed message
        if result and result.success:
            relayed = RelayedMessage(
                identity_id=identity.id,
                direction="nostr_to_bc",
                nostr_event_id=event.id,
                nostr_kind=event.kind,
                botcash_tx_id=result.tx_id,
                message_type=mapped.message_type,
                content_hash=self.mapper.compute_content_hash(mapped.content),
            )
            session.add(relayed)
            await session.commit()

            logger.info(
                "Bridged Nostr event to Botcash",
                nostr_event_id=event.id,
                botcash_tx_id=result.tx_id,
                message_type=mapped.message_type,
            )

    async def _resolve_event_to_tx(
        self,
        session: AsyncSession,
        event_id: str | None,
    ) -> str | None:
        """Resolve a Nostr event ID to a Botcash transaction ID.

        Args:
            session: Database session
            event_id: Nostr event ID

        Returns:
            Botcash transaction ID or None
        """
        if not event_id:
            return None

        result = await session.execute(
            select(RelayedMessage.botcash_tx_id).where(
                RelayedMessage.nostr_event_id == event_id
            )
        )
        row = result.first()
        return row[0] if row else None

    async def _check_rate_limit(self, session: AsyncSession, pubkey: str) -> bool:
        """Check if pubkey is within rate limit.

        Args:
            session: Database session
            pubkey: Nostr public key

        Returns:
            True if within limit, False if exceeded
        """
        from datetime import datetime, timedelta, timezone

        now = datetime.now(timezone.utc)
        window_start = now.replace(second=0, microsecond=0)

        result = await session.execute(
            select(RateLimitEntry).where(
                RateLimitEntry.nostr_pubkey == pubkey,
                RateLimitEntry.window_start == window_start,
            )
        )
        entry = result.scalar_one_or_none()

        if entry:
            if entry.event_count >= self.rate_limit:
                return False
            entry.event_count += 1
        else:
            entry = RateLimitEntry(
                nostr_pubkey=pubkey,
                window_start=window_start,
                event_count=1,
            )
            session.add(entry)

        await session.commit()
        return True

    async def _broadcast_event(self, event: NostrEvent) -> None:
        """Broadcast an event to all clients with matching subscriptions.

        Args:
            event: Event to broadcast
        """
        for client in list(self.clients.values()):
            for sub in client.subscriptions.values():
                if any(f.matches(event) for f in sub.filters):
                    await self._send_event(client, sub.subscription_id, event)
                    break  # Only send once per client

    async def bridge_botcash_post(
        self,
        session: AsyncSession,
        tx_id: str,
        author_address: str,
        content: str,
        message_type: str,
        metadata: dict[str, Any] | None = None,
    ) -> NostrEvent | None:
        """Bridge a Botcash post to Nostr.

        Called by the indexer watcher when a new Botcash post is detected.

        Args:
            session: Database session
            tx_id: Botcash transaction ID
            author_address: Author's Botcash address
            content: Post content
            message_type: Botcash message type
            metadata: Additional metadata

        Returns:
            Created NostrEvent or None if author not linked
        """
        # Get author's linked identity
        identity = await self.identity.get_identity_by_address(session, author_address)
        if not identity:
            return None

        # Check privacy mode
        if identity.privacy_mode == PrivacyMode.READ_ONLY:
            return None
        if identity.privacy_mode == PrivacyMode.PRIVATE and message_type != "dm":
            return None

        # Map to Nostr event
        metadata = metadata or {}
        metadata["botcash_tx_id"] = tx_id
        metadata["botcash_address"] = author_address

        event = self.mapper.botcash_to_nostr(
            message_type=message_type,
            content=content,
            author_pubkey=identity.nostr_pubkey,
            metadata=metadata,
        )

        if not event:
            return None

        # Note: Event needs to be signed before broadcast
        # This would require the user's Nostr private key, which we don't have
        # In practice, the bridge would need to sign events with the relay's key
        # and use tags to indicate the original Botcash author

        # Store event
        stored = StoredEvent(
            event_id=event.id,
            pubkey=event.pubkey,
            kind=event.kind,
            created_at=event.created_at,
            content=event.content,
            tags_json=json.dumps(event.tags),
            sig=event.sig,  # Empty until signed
            from_botcash=True,
            botcash_tx_id=tx_id,
        )
        session.add(stored)

        # Record relayed message
        relayed = RelayedMessage(
            identity_id=identity.id,
            direction="bc_to_nostr",
            nostr_event_id=event.id,
            nostr_kind=event.kind,
            botcash_tx_id=tx_id,
            message_type=message_type,
            content_hash=self.mapper.compute_content_hash(content),
        )
        session.add(relayed)
        await session.commit()

        # Broadcast to subscribers
        await self._broadcast_event(event)

        logger.info(
            "Bridged Botcash post to Nostr",
            botcash_tx_id=tx_id,
            nostr_event_id=event.id,
            message_type=message_type,
        )

        return event

    # === WebSocket helpers ===

    async def _send(self, client: ClientConnection, data: list) -> None:
        """Send a message to a client."""
        try:
            await client.websocket.send(json.dumps(data))
        except websockets.exceptions.ConnectionClosed:
            pass

    async def _send_event(
        self,
        client: ClientConnection,
        sub_id: str,
        event: NostrEvent,
    ) -> None:
        """Send an EVENT message to a client."""
        await self._send(client, ["EVENT", sub_id, event.to_dict()])

    async def _send_ok(
        self,
        client: ClientConnection,
        event_id: str,
        success: bool,
        message: str,
    ) -> None:
        """Send an OK message to a client."""
        await self._send(client, ["OK", event_id, success, message])

    async def _send_notice(self, client: ClientConnection, message: str) -> None:
        """Send a NOTICE message to a client."""
        await self._send(client, ["NOTICE", message])


async def start_relay(
    host: str,
    port: int,
    session_maker: async_sessionmaker,
    botcash_client: BotcashClient,
    identity_service: IdentityService,
    protocol_mapper: ProtocolMapper,
    allowed_kinds: list[int] | None = None,
    rate_limit_per_minute: int = 30,
) -> None:
    """Start the Nostr relay WebSocket server.

    Args:
        host: Host address to bind
        port: Port to listen on
        session_maker: SQLAlchemy session maker
        botcash_client: Botcash RPC client
        identity_service: Identity linking service
        protocol_mapper: Protocol translation mapper
        allowed_kinds: Allowed event kinds
        rate_limit_per_minute: Rate limit per pubkey
    """
    relay = NostrRelay(
        session_maker=session_maker,
        botcash_client=botcash_client,
        identity_service=identity_service,
        protocol_mapper=protocol_mapper,
        allowed_kinds=allowed_kinds,
        rate_limit_per_minute=rate_limit_per_minute,
    )

    logger.info("Starting Nostr relay", host=host, port=port)

    async with websockets.serve(relay.handle_connection, host, port):
        await asyncio.Future()  # Run forever
