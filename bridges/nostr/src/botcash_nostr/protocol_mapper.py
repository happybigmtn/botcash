"""Protocol mapper for bidirectional Nostr <-> Botcash message translation."""

import hashlib
import json
import time
from dataclasses import dataclass
from typing import Any

import structlog

from .nostr_types import (
    BOTCASH_TO_NOSTR_KIND,
    NOSTR_TO_BOTCASH_TYPE,
    NostrEvent,
    NostrKind,
    create_contact_list,
    create_reaction,
    create_text_note,
    hex_to_npub,
    hex_to_note,
    parse_zap_request,
    parse_zap_receipt,
)

logger = structlog.get_logger()


@dataclass
class MappedMessage:
    """Result of mapping a message between protocols."""
    message_type: str
    content: str
    metadata: dict[str, Any]
    reply_to: str | None = None
    mentions: list[str] | None = None


class ProtocolMapper:
    """Maps messages between Nostr and Botcash protocols.

    Protocol mapping (from specs/bridges.md):
    | Botcash | Nostr | Notes |
    |---------|-------|-------|
    | Post | Kind 1 (note) | Direct mapping |
    | DM | Kind 4 (encrypted) | Both encrypted |
    | Follow | Kind 3 (contacts) | Sync follow lists |
    | Profile | Kind 0 (metadata) | Sync bios |
    | Upvote | Kind 7 (reaction) | + zap for BCASH value |
    """

    def __init__(self, zap_conversion_rate: float = 0.00000001):
        """Initialize mapper.

        Args:
            zap_conversion_rate: Conversion rate from satoshis to BCASH
                                 Default: 1 sat = 0.00000001 BCASH
        """
        self.zap_conversion_rate = zap_conversion_rate

    def nostr_to_botcash(self, event: NostrEvent) -> MappedMessage | None:
        """Convert a Nostr event to Botcash message format.

        Args:
            event: Nostr event to convert

        Returns:
            MappedMessage or None if event kind not supported
        """
        kind = event.kind

        if kind == NostrKind.TEXT_NOTE:
            return self._map_text_note_to_post(event)
        elif kind == NostrKind.ENCRYPTED_DM:
            return self._map_dm(event)
        elif kind == NostrKind.CONTACTS:
            return self._map_contacts_to_follows(event)
        elif kind == NostrKind.METADATA:
            return self._map_metadata_to_profile(event)
        elif kind == NostrKind.REACTION:
            return self._map_reaction_to_upvote(event)
        elif kind == NostrKind.ZAP_REQUEST:
            return self._map_zap_request_to_tip(event)
        elif kind == NostrKind.ZAP_RECEIPT:
            return self._map_zap_receipt_to_tip(event)
        else:
            logger.debug("Unsupported Nostr kind", kind=kind)
            return None

    def botcash_to_nostr(
        self,
        message_type: str,
        content: str,
        author_pubkey: str,
        metadata: dict[str, Any] | None = None,
    ) -> NostrEvent | None:
        """Convert a Botcash message to Nostr event format.

        Args:
            message_type: Botcash message type (post, dm, follow, etc.)
            content: Message content
            author_pubkey: Author's Nostr pubkey (hex)
            metadata: Additional metadata (reply_to_event, mentions, etc.)

        Returns:
            NostrEvent (unsigned) or None if type not supported
        """
        metadata = metadata or {}

        if message_type == "post":
            return self._map_post_to_text_note(content, author_pubkey, metadata)
        elif message_type == "reply":
            return self._map_reply_to_text_note(content, author_pubkey, metadata)
        elif message_type == "dm":
            return self._map_dm_to_nostr(content, author_pubkey, metadata)
        elif message_type == "follow":
            return self._map_follow_to_contacts(author_pubkey, metadata)
        elif message_type == "profile":
            return self._map_profile_to_metadata(content, author_pubkey, metadata)
        elif message_type == "upvote":
            return self._map_upvote_to_reaction(author_pubkey, metadata)
        elif message_type == "tip":
            # Tips become zap receipts
            return self._map_tip_to_zap(author_pubkey, metadata)
        else:
            logger.debug("Unsupported Botcash message type", message_type=message_type)
            return None

    # === Nostr -> Botcash mappings ===

    def _map_text_note_to_post(self, event: NostrEvent) -> MappedMessage:
        """Map Nostr text note (kind 1) to Botcash post."""
        reply_to = event.get_reply_to()
        mentions = event.get_mentions()

        # Extract hashtags from content
        tags = []
        for word in event.content.split():
            if word.startswith("#") and len(word) > 1:
                tags.append(word[1:])

        return MappedMessage(
            message_type="reply" if reply_to else "post",
            content=event.content,
            metadata={
                "nostr_event_id": event.id,
                "nostr_pubkey": event.pubkey,
                "created_at": event.created_at,
                "tags": tags,
            },
            reply_to=reply_to,
            mentions=mentions,
        )

    def _map_dm(self, event: NostrEvent) -> MappedMessage:
        """Map Nostr encrypted DM (kind 4) to Botcash DM."""
        recipients = event.get_tag_values("p")
        recipient = recipients[0] if recipients else None

        return MappedMessage(
            message_type="dm",
            content=event.content,  # Still encrypted
            metadata={
                "nostr_event_id": event.id,
                "nostr_pubkey": event.pubkey,
                "recipient_pubkey": recipient,
                "created_at": event.created_at,
                "encrypted": True,
            },
        )

    def _map_contacts_to_follows(self, event: NostrEvent) -> MappedMessage:
        """Map Nostr contacts (kind 3) to Botcash follows."""
        # Extract followed pubkeys from p tags
        contacts = event.get_tag_values("p")

        return MappedMessage(
            message_type="follow_list",
            content="",
            metadata={
                "nostr_event_id": event.id,
                "nostr_pubkey": event.pubkey,
                "follows": contacts,
                "created_at": event.created_at,
            },
        )

    def _map_metadata_to_profile(self, event: NostrEvent) -> MappedMessage:
        """Map Nostr metadata (kind 0) to Botcash profile."""
        try:
            profile_data = json.loads(event.content)
        except json.JSONDecodeError:
            profile_data = {}

        return MappedMessage(
            message_type="profile",
            content=json.dumps(profile_data),
            metadata={
                "nostr_event_id": event.id,
                "nostr_pubkey": event.pubkey,
                "name": profile_data.get("name", ""),
                "about": profile_data.get("about", ""),
                "picture": profile_data.get("picture", ""),
                "nip05": profile_data.get("nip05", ""),
                "created_at": event.created_at,
            },
        )

    def _map_reaction_to_upvote(self, event: NostrEvent) -> MappedMessage:
        """Map Nostr reaction (kind 7) to Botcash upvote."""
        target_events = event.get_tag_values("e")
        target_pubkeys = event.get_tag_values("p")

        # Reaction content: "+" for like, "-" for dislike, or emoji
        reaction = event.content or "+"
        is_upvote = reaction not in ["-"]

        return MappedMessage(
            message_type="upvote" if is_upvote else "downvote",
            content=reaction,
            metadata={
                "nostr_event_id": event.id,
                "nostr_pubkey": event.pubkey,
                "target_event_id": target_events[0] if target_events else None,
                "target_pubkey": target_pubkeys[0] if target_pubkeys else None,
                "created_at": event.created_at,
            },
        )

    def _map_zap_request_to_tip(self, event: NostrEvent) -> MappedMessage | None:
        """Map Nostr zap request (kind 9734) to Botcash tip."""
        zap_info = parse_zap_request(event)
        if not zap_info:
            return None

        # Convert millisats to BCASH
        amount_sats = zap_info["amount_msats"] // 1000
        amount_bcash = amount_sats * self.zap_conversion_rate

        return MappedMessage(
            message_type="tip_request",
            content=zap_info.get("message", ""),
            metadata={
                "nostr_event_id": event.id,
                "sender_pubkey": zap_info["sender"],
                "recipient_pubkey": zap_info["recipient"],
                "target_event_id": zap_info.get("target_event"),
                "amount_msats": zap_info["amount_msats"],
                "amount_sats": amount_sats,
                "amount_bcash": amount_bcash,
                "created_at": event.created_at,
            },
        )

    def _map_zap_receipt_to_tip(self, event: NostrEvent) -> MappedMessage | None:
        """Map Nostr zap receipt (kind 9735) to Botcash tip completion."""
        zap_info = parse_zap_receipt(event)
        if not zap_info:
            return None

        # Convert millisats to BCASH
        amount_sats = zap_info["amount_msats"] // 1000
        amount_bcash = amount_sats * self.zap_conversion_rate

        return MappedMessage(
            message_type="tip",
            content=zap_info.get("message", ""),
            metadata={
                "nostr_event_id": event.id,
                "receipt_id": zap_info.get("receipt_id"),
                "sender_pubkey": zap_info["sender"],
                "recipient_pubkey": zap_info["recipient"],
                "target_event_id": zap_info.get("target_event"),
                "amount_msats": zap_info["amount_msats"],
                "amount_sats": amount_sats,
                "amount_bcash": amount_bcash,
                "bolt11": zap_info.get("bolt11"),
                "created_at": event.created_at,
            },
        )

    # === Botcash -> Nostr mappings ===

    def _map_post_to_text_note(
        self,
        content: str,
        author_pubkey: str,
        metadata: dict[str, Any],
    ) -> NostrEvent:
        """Map Botcash post to Nostr text note (kind 1)."""
        # Add Botcash attribution
        attribution = f"\n\n— Posted via Botcash"
        if metadata.get("botcash_tx_id"):
            attribution += f" (tx: {metadata['botcash_tx_id'][:8]}...)"

        full_content = content + attribution

        return create_text_note(
            pubkey=author_pubkey,
            content=full_content,
            mentions=metadata.get("mentions"),
        )

    def _map_reply_to_text_note(
        self,
        content: str,
        author_pubkey: str,
        metadata: dict[str, Any],
    ) -> NostrEvent:
        """Map Botcash reply to Nostr text note with reply tags."""
        reply_to_event = metadata.get("reply_to_event")
        mentions = metadata.get("mentions", [])

        # Add Botcash attribution
        attribution = f"\n\n— Posted via Botcash"
        if metadata.get("botcash_tx_id"):
            attribution += f" (tx: {metadata['botcash_tx_id'][:8]}...)"

        full_content = content + attribution

        return create_text_note(
            pubkey=author_pubkey,
            content=full_content,
            reply_to=reply_to_event,
            mentions=mentions,
        )

    def _map_dm_to_nostr(
        self,
        content: str,
        author_pubkey: str,
        metadata: dict[str, Any],
    ) -> NostrEvent:
        """Map Botcash DM to Nostr encrypted DM (kind 4)."""
        recipient_pubkey = metadata.get("recipient_pubkey", "")

        event = NostrEvent(
            pubkey=author_pubkey,
            created_at=int(time.time()),
            kind=NostrKind.ENCRYPTED_DM,
            tags=[["p", recipient_pubkey]],
            content=content,  # Should already be encrypted
        )
        event.id = event.compute_id()
        return event

    def _map_follow_to_contacts(
        self,
        author_pubkey: str,
        metadata: dict[str, Any],
    ) -> NostrEvent:
        """Map Botcash follow to Nostr contacts (kind 3)."""
        follows = metadata.get("follows", [])
        relay_url = metadata.get("relay_url", "")

        # Create contacts list (pubkey, relay, petname)
        contacts = [(pk, relay_url, "") for pk in follows]

        return create_contact_list(
            pubkey=author_pubkey,
            contacts=contacts,
        )

    def _map_profile_to_metadata(
        self,
        content: str,
        author_pubkey: str,
        metadata: dict[str, Any],
    ) -> NostrEvent:
        """Map Botcash profile to Nostr metadata (kind 0)."""
        # Try to parse existing content as JSON, or create new
        try:
            profile_data = json.loads(content)
        except json.JSONDecodeError:
            profile_data = {}

        # Merge with metadata
        if metadata.get("name"):
            profile_data["name"] = metadata["name"]
        if metadata.get("about"):
            profile_data["about"] = metadata["about"]
        if metadata.get("picture"):
            profile_data["picture"] = metadata["picture"]

        # Add Botcash reference
        profile_data["botcash_address"] = metadata.get("botcash_address", "")

        event = NostrEvent(
            pubkey=author_pubkey,
            created_at=int(time.time()),
            kind=NostrKind.METADATA,
            tags=[],
            content=json.dumps(profile_data),
        )
        event.id = event.compute_id()
        return event

    def _map_upvote_to_reaction(
        self,
        author_pubkey: str,
        metadata: dict[str, Any],
    ) -> NostrEvent | None:
        """Map Botcash upvote to Nostr reaction (kind 7)."""
        target_event_id = metadata.get("target_event_id")
        target_pubkey = metadata.get("target_pubkey")

        if not target_event_id or not target_pubkey:
            return None

        return create_reaction(
            pubkey=author_pubkey,
            target_event_id=target_event_id,
            target_pubkey=target_pubkey,
            reaction="+",
        )

    def _map_tip_to_zap(
        self,
        author_pubkey: str,
        metadata: dict[str, Any],
    ) -> NostrEvent | None:
        """Map Botcash tip to Nostr zap receipt (kind 9735).

        Note: This is a simplified representation since we can't create
        a real Lightning invoice. The zap receipt indicates a BCASH tip was sent.
        """
        recipient_pubkey = metadata.get("recipient_pubkey")
        if not recipient_pubkey:
            return None

        # Convert BCASH to sats for Nostr display
        amount_bcash = metadata.get("amount_bcash", 0)
        amount_sats = int(amount_bcash / self.zap_conversion_rate)
        amount_msats = amount_sats * 1000

        # Create a simplified zap receipt
        tags = [
            ["p", recipient_pubkey],
            ["amount", str(amount_msats)],
            ["description", json.dumps({
                "source": "botcash",
                "tx_id": metadata.get("botcash_tx_id", ""),
                "amount_bcash": str(amount_bcash),
            })],
        ]

        if metadata.get("target_event_id"):
            tags.append(["e", metadata["target_event_id"]])

        event = NostrEvent(
            pubkey=author_pubkey,
            created_at=int(time.time()),
            kind=NostrKind.ZAP_RECEIPT,
            tags=tags,
            content="",
        )
        event.id = event.compute_id()
        return event

    def compute_content_hash(self, content: str) -> str:
        """Compute hash of content for deduplication.

        Args:
            content: Message content

        Returns:
            SHA256 hash as hex string
        """
        return hashlib.sha256(content.encode()).hexdigest()
