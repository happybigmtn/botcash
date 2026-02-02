"""Nostr protocol types and event handling (NIP-01, NIP-04, NIP-05, NIP-57)."""

import hashlib
import json
import time
from dataclasses import dataclass, field
from enum import IntEnum
from typing import Any

import bech32


class NostrKind(IntEnum):
    """Nostr event kinds (NIPs).

    References:
    - NIP-01: Basic protocol (kinds 0, 1, 3)
    - NIP-04: Encrypted Direct Messages (kind 4)
    - NIP-25: Reactions (kind 7)
    - NIP-57: Zaps (kinds 9734, 9735)
    """
    METADATA = 0           # User profile (name, about, picture)
    TEXT_NOTE = 1          # Short text note (like a tweet)
    CONTACTS = 3           # Contact list / follow list
    ENCRYPTED_DM = 4       # Encrypted direct message (NIP-04)
    REACTION = 7           # Reaction to another event (like/+/-)
    ZAP_REQUEST = 9734     # Zap request (NIP-57)
    ZAP_RECEIPT = 9735     # Zap receipt (NIP-57)


# Mapping from Nostr kinds to Botcash message types
NOSTR_TO_BOTCASH_TYPE = {
    NostrKind.METADATA: "profile",
    NostrKind.TEXT_NOTE: "post",
    NostrKind.CONTACTS: "follow",
    NostrKind.ENCRYPTED_DM: "dm",
    NostrKind.REACTION: "upvote",
    NostrKind.ZAP_REQUEST: "tip",
    NostrKind.ZAP_RECEIPT: "tip",
}

# Mapping from Botcash message types to Nostr kinds
BOTCASH_TO_NOSTR_KIND = {
    "profile": NostrKind.METADATA,
    "post": NostrKind.TEXT_NOTE,
    "follow": NostrKind.CONTACTS,
    "dm": NostrKind.ENCRYPTED_DM,
    "upvote": NostrKind.REACTION,
    "tip": NostrKind.ZAP_RECEIPT,
}


@dataclass
class NostrEvent:
    """A Nostr event (NIP-01).

    Events are the core data structure in Nostr. Each event has:
    - id: 32-byte SHA256 of the serialized event data
    - pubkey: 32-byte public key of the event creator
    - created_at: Unix timestamp in seconds
    - kind: Integer representing the event type
    - tags: Array of arrays (e.g., [["p", pubkey], ["e", event_id]])
    - content: String content (interpretation depends on kind)
    - sig: 64-byte Schnorr signature of the id
    """
    id: str = ""
    pubkey: str = ""
    created_at: int = 0
    kind: int = 0
    tags: list[list[str]] = field(default_factory=list)
    content: str = ""
    sig: str = ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "NostrEvent":
        """Parse event from dictionary."""
        return cls(
            id=data.get("id", ""),
            pubkey=data.get("pubkey", ""),
            created_at=data.get("created_at", 0),
            kind=data.get("kind", 0),
            tags=data.get("tags", []),
            content=data.get("content", ""),
            sig=data.get("sig", ""),
        )

    def to_dict(self) -> dict[str, Any]:
        """Convert event to dictionary."""
        return {
            "id": self.id,
            "pubkey": self.pubkey,
            "created_at": self.created_at,
            "kind": self.kind,
            "tags": self.tags,
            "content": self.content,
            "sig": self.sig,
        }

    def compute_id(self) -> str:
        """Compute the event ID (SHA256 of serialized event data)."""
        serialized = json.dumps([
            0,  # Reserved for future use
            self.pubkey,
            self.created_at,
            self.kind,
            self.tags,
            self.content,
        ], separators=(',', ':'), ensure_ascii=False)
        return hashlib.sha256(serialized.encode()).hexdigest()

    def get_tag_values(self, tag_name: str) -> list[str]:
        """Get all values for a specific tag type."""
        return [tag[1] for tag in self.tags if len(tag) >= 2 and tag[0] == tag_name]

    def get_reply_to(self) -> str | None:
        """Get the event ID this is a reply to (if any)."""
        e_tags = self.get_tag_values("e")
        return e_tags[0] if e_tags else None

    def get_mentions(self) -> list[str]:
        """Get mentioned pubkeys from p tags."""
        return self.get_tag_values("p")

    @property
    def is_valid_id(self) -> bool:
        """Check if the event ID matches the computed hash."""
        return self.id == self.compute_id()


@dataclass
class NostrFilter:
    """A Nostr subscription filter (NIP-01).

    Filters specify which events a client wants to receive.
    """
    ids: list[str] = field(default_factory=list)
    authors: list[str] = field(default_factory=list)
    kinds: list[int] = field(default_factory=list)
    tags: dict[str, list[str]] = field(default_factory=dict)  # #e, #p tags
    since: int | None = None
    until: int | None = None
    limit: int | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "NostrFilter":
        """Parse filter from dictionary."""
        tags = {}
        for key, value in data.items():
            if key.startswith("#") and isinstance(value, list):
                tags[key[1:]] = value

        return cls(
            ids=data.get("ids", []),
            authors=data.get("authors", []),
            kinds=data.get("kinds", []),
            tags=tags,
            since=data.get("since"),
            until=data.get("until"),
            limit=data.get("limit"),
        )

    def matches(self, event: NostrEvent) -> bool:
        """Check if an event matches this filter."""
        if self.ids and event.id not in self.ids:
            return False
        if self.authors and event.pubkey not in self.authors:
            return False
        if self.kinds and event.kind not in self.kinds:
            return False
        if self.since and event.created_at < self.since:
            return False
        if self.until and event.created_at > self.until:
            return False

        # Check tag filters
        for tag_name, values in self.tags.items():
            event_values = event.get_tag_values(tag_name)
            if not any(v in event_values for v in values):
                return False

        return True


def npub_to_hex(npub: str) -> str:
    """Convert npub (bech32) to hex pubkey.

    Args:
        npub: Bech32-encoded public key (npub1...)

    Returns:
        Hex-encoded public key

    Raises:
        ValueError: If invalid npub format
    """
    if not npub.startswith("npub1"):
        raise ValueError("Invalid npub: must start with 'npub1'")

    hrp, data = bech32.bech32_decode(npub)
    if hrp != "npub" or data is None:
        raise ValueError("Invalid npub encoding")

    decoded = bech32.convertbits(data, 5, 8, False)
    if decoded is None or len(decoded) != 32:
        raise ValueError("Invalid npub: wrong length")

    return bytes(decoded).hex()


def hex_to_npub(pubkey_hex: str) -> str:
    """Convert hex pubkey to npub (bech32).

    Args:
        pubkey_hex: Hex-encoded public key (64 chars)

    Returns:
        Bech32-encoded public key (npub1...)

    Raises:
        ValueError: If invalid hex format
    """
    if len(pubkey_hex) != 64:
        raise ValueError("Invalid pubkey: must be 64 hex characters")

    try:
        pubkey_bytes = bytes.fromhex(pubkey_hex)
    except ValueError:
        raise ValueError("Invalid pubkey: not valid hex")

    converted = bech32.convertbits(list(pubkey_bytes), 8, 5, True)
    if converted is None:
        raise ValueError("Failed to convert pubkey")

    return bech32.bech32_encode("npub", converted)


def nsec_to_hex(nsec: str) -> str:
    """Convert nsec (bech32) to hex private key.

    Args:
        nsec: Bech32-encoded private key (nsec1...)

    Returns:
        Hex-encoded private key

    Raises:
        ValueError: If invalid nsec format
    """
    if not nsec.startswith("nsec1"):
        raise ValueError("Invalid nsec: must start with 'nsec1'")

    hrp, data = bech32.bech32_decode(nsec)
    if hrp != "nsec" or data is None:
        raise ValueError("Invalid nsec encoding")

    decoded = bech32.convertbits(data, 5, 8, False)
    if decoded is None or len(decoded) != 32:
        raise ValueError("Invalid nsec: wrong length")

    return bytes(decoded).hex()


def hex_to_nsec(privkey_hex: str) -> str:
    """Convert hex private key to nsec (bech32).

    Args:
        privkey_hex: Hex-encoded private key (64 chars)

    Returns:
        Bech32-encoded private key (nsec1...)

    Raises:
        ValueError: If invalid hex format
    """
    if len(privkey_hex) != 64:
        raise ValueError("Invalid privkey: must be 64 hex characters")

    try:
        privkey_bytes = bytes.fromhex(privkey_hex)
    except ValueError:
        raise ValueError("Invalid privkey: not valid hex")

    converted = bech32.convertbits(list(privkey_bytes), 8, 5, True)
    if converted is None:
        raise ValueError("Failed to convert privkey")

    return bech32.bech32_encode("nsec", converted)


def note_to_hex(note_id: str) -> str:
    """Convert note1... (bech32) to hex event ID.

    Args:
        note_id: Bech32-encoded event ID (note1...)

    Returns:
        Hex-encoded event ID

    Raises:
        ValueError: If invalid note format
    """
    if not note_id.startswith("note1"):
        raise ValueError("Invalid note: must start with 'note1'")

    hrp, data = bech32.bech32_decode(note_id)
    if hrp != "note" or data is None:
        raise ValueError("Invalid note encoding")

    decoded = bech32.convertbits(data, 5, 8, False)
    if decoded is None or len(decoded) != 32:
        raise ValueError("Invalid note: wrong length")

    return bytes(decoded).hex()


def hex_to_note(event_id_hex: str) -> str:
    """Convert hex event ID to note1... (bech32).

    Args:
        event_id_hex: Hex-encoded event ID (64 chars)

    Returns:
        Bech32-encoded event ID (note1...)

    Raises:
        ValueError: If invalid hex format
    """
    if len(event_id_hex) != 64:
        raise ValueError("Invalid event ID: must be 64 hex characters")

    try:
        event_bytes = bytes.fromhex(event_id_hex)
    except ValueError:
        raise ValueError("Invalid event ID: not valid hex")

    converted = bech32.convertbits(list(event_bytes), 8, 5, True)
    if converted is None:
        raise ValueError("Failed to convert event ID")

    return bech32.bech32_encode("note", converted)


def create_text_note(pubkey: str, content: str, reply_to: str | None = None,
                     mentions: list[str] | None = None) -> NostrEvent:
    """Create a text note event (kind 1).

    Args:
        pubkey: Author's public key (hex)
        content: Note content
        reply_to: Event ID to reply to (optional)
        mentions: List of pubkeys to mention (optional)

    Returns:
        Unsigned NostrEvent (needs signing)
    """
    tags: list[list[str]] = []

    if reply_to:
        tags.append(["e", reply_to])

    for mention in mentions or []:
        tags.append(["p", mention])

    event = NostrEvent(
        pubkey=pubkey,
        created_at=int(time.time()),
        kind=NostrKind.TEXT_NOTE,
        tags=tags,
        content=content,
    )
    event.id = event.compute_id()
    return event


def create_reaction(pubkey: str, target_event_id: str, target_pubkey: str,
                    reaction: str = "+") -> NostrEvent:
    """Create a reaction event (kind 7).

    Args:
        pubkey: Reactor's public key (hex)
        target_event_id: Event ID being reacted to
        target_pubkey: Author of the event being reacted to
        reaction: Reaction content ("+" for like, "-" for dislike, or emoji)

    Returns:
        Unsigned NostrEvent (needs signing)
    """
    event = NostrEvent(
        pubkey=pubkey,
        created_at=int(time.time()),
        kind=NostrKind.REACTION,
        tags=[
            ["e", target_event_id],
            ["p", target_pubkey],
        ],
        content=reaction,
    )
    event.id = event.compute_id()
    return event


def create_contact_list(pubkey: str, contacts: list[tuple[str, str, str]]) -> NostrEvent:
    """Create a contact list event (kind 3).

    Args:
        pubkey: Author's public key (hex)
        contacts: List of (pubkey, relay_url, petname) tuples

    Returns:
        Unsigned NostrEvent (needs signing)
    """
    tags = [["p", pk, relay, name] for pk, relay, name in contacts]

    event = NostrEvent(
        pubkey=pubkey,
        created_at=int(time.time()),
        kind=NostrKind.CONTACTS,
        tags=tags,
        content="",
    )
    event.id = event.compute_id()
    return event


def parse_zap_request(event: NostrEvent) -> dict[str, Any] | None:
    """Parse a zap request event (kind 9734).

    Args:
        event: Zap request event

    Returns:
        Dictionary with zap details or None if invalid
    """
    if event.kind != NostrKind.ZAP_REQUEST:
        return None

    # Get recipient pubkey
    p_tags = event.get_tag_values("p")
    if not p_tags:
        return None

    # Get target event (optional)
    e_tags = event.get_tag_values("e")

    # Get amount from relays tag (millisats)
    amount_tags = [tag for tag in event.tags if tag[0] == "amount"]
    amount_msats = int(amount_tags[0][1]) if amount_tags else 0

    return {
        "sender": event.pubkey,
        "recipient": p_tags[0],
        "target_event": e_tags[0] if e_tags else None,
        "amount_msats": amount_msats,
        "message": event.content,
    }


def parse_zap_receipt(event: NostrEvent) -> dict[str, Any] | None:
    """Parse a zap receipt event (kind 9735).

    Args:
        event: Zap receipt event

    Returns:
        Dictionary with zap receipt details or None if invalid
    """
    if event.kind != NostrKind.ZAP_RECEIPT:
        return None

    # Get the original zap request from description tag
    desc_tags = [tag for tag in event.tags if tag[0] == "description"]
    if not desc_tags:
        return None

    try:
        zap_request_data = json.loads(desc_tags[0][1])
        zap_request = NostrEvent.from_dict(zap_request_data)
    except (json.JSONDecodeError, KeyError):
        return None

    zap_info = parse_zap_request(zap_request)
    if not zap_info:
        return None

    # Add receipt-specific info
    zap_info["receipt_id"] = event.id
    zap_info["zapper"] = event.pubkey  # LNURL provider pubkey

    # Get bolt11 invoice
    bolt11_tags = [tag for tag in event.tags if tag[0] == "bolt11"]
    zap_info["bolt11"] = bolt11_tags[0][1] if bolt11_tags else None

    return zap_info
