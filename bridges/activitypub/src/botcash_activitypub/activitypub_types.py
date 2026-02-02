"""ActivityPub protocol types and utilities for Botcash bridge.

This module implements the core ActivityPub/ActivityStreams data types
needed for federation with Mastodon and other Fediverse servers.

References:
- ActivityPub spec: https://www.w3.org/TR/activitypub/
- ActivityStreams 2.0: https://www.w3.org/TR/activitystreams-core/
- Mastodon API: https://docs.joinmastodon.org/spec/activitypub/
"""

import hashlib
import json
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any, TypeAlias

# JSON-LD contexts for ActivityPub
ACTIVITY_STREAMS_CONTEXT = "https://www.w3.org/ns/activitystreams"
SECURITY_CONTEXT = "https://w3id.org/security/v1"
MASTODON_CONTEXT = {
    "manuallyApprovesFollowers": "as:manuallyApprovesFollowers",
    "sensitive": "as:sensitive",
    "Hashtag": "as:Hashtag",
    "toot": "http://joinmastodon.org/ns#",
}

# Standard ActivityPub context
AP_CONTEXT: list[str | dict] = [
    ACTIVITY_STREAMS_CONTEXT,
    SECURITY_CONTEXT,
]

# Content types
AP_CONTENT_TYPE = "application/activity+json"
AP_ACCEPT_HEADER = 'application/activity+json, application/ld+json; profile="https://www.w3.org/ns/activitystreams"'

# Type aliases
JsonDict: TypeAlias = dict[str, Any]


class ActivityType(str, Enum):
    """ActivityPub activity types."""
    # Core activities
    CREATE = "Create"
    UPDATE = "Update"
    DELETE = "Delete"

    # Social activities
    FOLLOW = "Follow"
    ACCEPT = "Accept"
    REJECT = "Reject"
    UNDO = "Undo"

    # Reactions
    LIKE = "Like"
    ANNOUNCE = "Announce"  # Boost/reblog

    # Other
    ADD = "Add"
    REMOVE = "Remove"


class ObjectType(str, Enum):
    """ActivityPub object types."""
    # Actors
    PERSON = "Person"
    SERVICE = "Service"
    APPLICATION = "Application"
    GROUP = "Group"
    ORGANIZATION = "Organization"

    # Content
    NOTE = "Note"
    ARTICLE = "Article"
    IMAGE = "Image"
    VIDEO = "Video"
    DOCUMENT = "Document"

    # Collections
    COLLECTION = "Collection"
    ORDERED_COLLECTION = "OrderedCollection"
    COLLECTION_PAGE = "CollectionPage"
    ORDERED_COLLECTION_PAGE = "OrderedCollectionPage"


@dataclass
class PublicKey:
    """RSA public key for HTTP signatures."""
    id: str  # e.g., https://botcash.social/users/bs1abc#main-key
    owner: str  # Actor ID
    public_key_pem: str

    def to_dict(self) -> JsonDict:
        """Convert to ActivityPub JSON-LD format."""
        return {
            "id": self.id,
            "owner": self.owner,
            "publicKeyPem": self.public_key_pem,
        }


@dataclass
class Actor:
    """ActivityPub Actor (Person, Service, etc.).

    Represents a Botcash address as an ActivityPub actor for federation.
    """
    id: str  # https://botcash.social/users/bs1abc123
    type: ObjectType = ObjectType.PERSON
    preferred_username: str = ""  # bs1abc123
    name: str = ""  # Display name
    summary: str = ""  # Bio/about
    url: str = ""  # Profile URL
    inbox: str = ""  # Inbox endpoint
    outbox: str = ""  # Outbox endpoint
    followers: str = ""  # Followers collection
    following: str = ""  # Following collection
    public_key: PublicKey | None = None
    icon: JsonDict | None = None  # Avatar
    image: JsonDict | None = None  # Header/banner
    manually_approves_followers: bool = False
    discoverable: bool = True
    published: str = ""  # ISO timestamp

    # Botcash-specific
    botcash_address: str = ""

    def to_dict(self) -> JsonDict:
        """Convert to ActivityPub JSON-LD format."""
        actor = {
            "@context": AP_CONTEXT,
            "id": self.id,
            "type": self.type.value,
            "preferredUsername": self.preferred_username,
            "name": self.name or self.preferred_username,
            "summary": self.summary,
            "url": self.url or self.id,
            "inbox": self.inbox,
            "outbox": self.outbox,
            "followers": self.followers,
            "following": self.following,
            "manuallyApprovesFollowers": self.manually_approves_followers,
            "discoverable": self.discoverable,
        }

        if self.public_key:
            actor["publicKey"] = self.public_key.to_dict()

        if self.icon:
            actor["icon"] = self.icon

        if self.image:
            actor["image"] = self.image

        if self.published:
            actor["published"] = self.published

        # Add Botcash reference in attachment
        if self.botcash_address:
            actor["attachment"] = [
                {
                    "type": "PropertyValue",
                    "name": "Botcash Address",
                    "value": self.botcash_address,
                }
            ]

        return actor


@dataclass
class Note:
    """ActivityPub Note object (post/status)."""
    id: str  # https://botcash.social/users/bs1abc/statuses/123
    content: str
    attributed_to: str  # Actor ID
    published: str = ""  # ISO timestamp
    to: list[str] = field(default_factory=list)  # Recipients (public = as:Public)
    cc: list[str] = field(default_factory=list)  # CC recipients
    in_reply_to: str | None = None  # Reply target
    url: str = ""
    sensitive: bool = False
    summary: str | None = None  # Content warning
    tag: list[JsonDict] = field(default_factory=list)  # Hashtags, mentions
    attachment: list[JsonDict] = field(default_factory=list)  # Media

    # Botcash-specific
    botcash_tx_id: str = ""

    def to_dict(self) -> JsonDict:
        """Convert to ActivityPub JSON-LD format."""
        note = {
            "id": self.id,
            "type": ObjectType.NOTE.value,
            "content": self.content,
            "attributedTo": self.attributed_to,
            "published": self.published or datetime.now(timezone.utc).isoformat(),
            "to": self.to,
            "cc": self.cc,
            "url": self.url or self.id,
            "sensitive": self.sensitive,
        }

        if self.in_reply_to:
            note["inReplyTo"] = self.in_reply_to

        if self.summary:
            note["summary"] = self.summary

        if self.tag:
            note["tag"] = self.tag

        if self.attachment:
            note["attachment"] = self.attachment

        # Add Botcash transaction reference
        if self.botcash_tx_id:
            note["source"] = {
                "mediaType": "text/x-botcash",
                "content": f"tx:{self.botcash_tx_id}",
            }

        return note


@dataclass
class Activity:
    """ActivityPub Activity wrapper."""
    id: str
    type: ActivityType
    actor: str  # Actor ID performing the activity
    object: str | JsonDict  # Target object (ID or inline object)
    published: str = ""
    to: list[str] = field(default_factory=list)
    cc: list[str] = field(default_factory=list)

    def to_dict(self) -> JsonDict:
        """Convert to ActivityPub JSON-LD format."""
        activity = {
            "@context": AP_CONTEXT,
            "id": self.id,
            "type": self.type.value,
            "actor": self.actor,
            "object": self.object if isinstance(self.object, str) else self.object,
            "published": self.published or datetime.now(timezone.utc).isoformat(),
        }

        if self.to:
            activity["to"] = self.to
        if self.cc:
            activity["cc"] = self.cc

        return activity


@dataclass
class OrderedCollection:
    """ActivityPub OrderedCollection for outbox/followers/following."""
    id: str
    total_items: int = 0
    first: str = ""  # First page URL
    last: str = ""  # Last page URL

    def to_dict(self) -> JsonDict:
        """Convert to ActivityPub JSON-LD format."""
        collection = {
            "@context": AP_CONTEXT,
            "id": self.id,
            "type": ObjectType.ORDERED_COLLECTION.value,
            "totalItems": self.total_items,
        }

        if self.first:
            collection["first"] = self.first
        if self.last:
            collection["last"] = self.last

        return collection


@dataclass
class OrderedCollectionPage:
    """Page of an OrderedCollection."""
    id: str
    part_of: str  # Parent collection ID
    items: list[str | JsonDict] = field(default_factory=list)
    next: str = ""
    prev: str = ""

    def to_dict(self) -> JsonDict:
        """Convert to ActivityPub JSON-LD format."""
        page = {
            "@context": AP_CONTEXT,
            "id": self.id,
            "type": ObjectType.ORDERED_COLLECTION_PAGE.value,
            "partOf": self.part_of,
            "orderedItems": self.items,
        }

        if self.next:
            page["next"] = self.next
        if self.prev:
            page["prev"] = self.prev

        return page


# Public addressing
AS_PUBLIC = "https://www.w3.org/ns/activitystreams#Public"


def create_actor(
    base_url: str,
    botcash_address: str,
    display_name: str = "",
    summary: str = "",
    public_key_pem: str = "",
) -> Actor:
    """Create an Actor for a Botcash address.

    Args:
        base_url: Server base URL (e.g., https://botcash.social)
        botcash_address: Botcash address (e.g., bs1abc123...)
        display_name: Optional display name
        summary: Optional bio/summary
        public_key_pem: RSA public key in PEM format

    Returns:
        Actor instance
    """
    # Create local part from address (truncated for readability)
    local_part = botcash_address[:20] if len(botcash_address) > 20 else botcash_address
    actor_url = f"{base_url}/users/{local_part}"

    public_key = None
    if public_key_pem:
        public_key = PublicKey(
            id=f"{actor_url}#main-key",
            owner=actor_url,
            public_key_pem=public_key_pem,
        )

    return Actor(
        id=actor_url,
        type=ObjectType.PERSON,
        preferred_username=local_part,
        name=display_name or local_part,
        summary=summary or f"Botcash user {botcash_address}",
        url=actor_url,
        inbox=f"{actor_url}/inbox",
        outbox=f"{actor_url}/outbox",
        followers=f"{actor_url}/followers",
        following=f"{actor_url}/following",
        public_key=public_key,
        manually_approves_followers=False,
        discoverable=True,
        published=datetime.now(timezone.utc).isoformat(),
        botcash_address=botcash_address,
    )


def create_note(
    base_url: str,
    actor_local_part: str,
    content: str,
    note_id: str,
    in_reply_to: str | None = None,
    sensitive: bool = False,
    content_warning: str | None = None,
    mentions: list[str] | None = None,
    hashtags: list[str] | None = None,
    botcash_tx_id: str = "",
) -> Note:
    """Create a Note (post/status).

    Args:
        base_url: Server base URL
        actor_local_part: Actor's local username
        content: Post content (HTML)
        note_id: Unique note identifier
        in_reply_to: ID of note being replied to
        sensitive: Whether content is sensitive
        content_warning: Content warning text
        mentions: List of mentioned actor IDs
        hashtags: List of hashtags (without #)
        botcash_tx_id: Source Botcash transaction ID

    Returns:
        Note instance
    """
    actor_url = f"{base_url}/users/{actor_local_part}"
    note_url = f"{actor_url}/statuses/{note_id}"

    # Build tags for mentions and hashtags
    tags: list[JsonDict] = []

    if mentions:
        for mention in mentions:
            # Extract username from actor URL
            mention_name = mention.split("/")[-1] if "/" in mention else mention
            tags.append({
                "type": "Mention",
                "href": mention,
                "name": f"@{mention_name}",
            })

    if hashtags:
        for tag in hashtags:
            tag_clean = tag.lstrip("#")
            tags.append({
                "type": "Hashtag",
                "href": f"{base_url}/tags/{tag_clean}",
                "name": f"#{tag_clean}",
            })

    # Default to public
    to = [AS_PUBLIC]
    cc = [f"{actor_url}/followers"]

    if mentions:
        cc.extend(mentions)

    return Note(
        id=note_url,
        content=content,
        attributed_to=actor_url,
        published=datetime.now(timezone.utc).isoformat(),
        to=to,
        cc=cc,
        in_reply_to=in_reply_to,
        url=note_url,
        sensitive=sensitive,
        summary=content_warning,
        tag=tags,
        botcash_tx_id=botcash_tx_id,
    )


def create_activity(
    base_url: str,
    actor_local_part: str,
    activity_type: ActivityType,
    activity_object: str | JsonDict,
    activity_id: str | None = None,
) -> Activity:
    """Create an Activity.

    Args:
        base_url: Server base URL
        actor_local_part: Actor's local username
        activity_type: Type of activity
        activity_object: Object ID or inline object
        activity_id: Optional activity ID (generated if not provided)

    Returns:
        Activity instance
    """
    actor_url = f"{base_url}/users/{actor_local_part}"

    if not activity_id:
        # Generate unique activity ID
        timestamp = int(time.time() * 1000)
        activity_id = f"{actor_url}/activities/{timestamp}"

    # Determine recipients based on object
    to = [AS_PUBLIC]
    cc = [f"{actor_url}/followers"]

    if isinstance(activity_object, dict) and "to" in activity_object:
        to = activity_object["to"]
    if isinstance(activity_object, dict) and "cc" in activity_object:
        cc = activity_object["cc"]

    return Activity(
        id=activity_id,
        type=activity_type,
        actor=actor_url,
        object=activity_object,
        published=datetime.now(timezone.utc).isoformat(),
        to=to,
        cc=cc,
    )


def parse_actor(data: JsonDict) -> Actor | None:
    """Parse an Actor from JSON-LD data.

    Args:
        data: JSON-LD actor document

    Returns:
        Actor instance or None if invalid
    """
    try:
        actor_type = data.get("type", "Person")
        if isinstance(actor_type, list):
            actor_type = actor_type[0]

        public_key = None
        if "publicKey" in data:
            pk = data["publicKey"]
            public_key = PublicKey(
                id=pk.get("id", ""),
                owner=pk.get("owner", ""),
                public_key_pem=pk.get("publicKeyPem", ""),
            )

        icon = data.get("icon")
        if isinstance(icon, list):
            icon = icon[0] if icon else None

        return Actor(
            id=data.get("id", ""),
            type=ObjectType(actor_type) if actor_type in [t.value for t in ObjectType] else ObjectType.PERSON,
            preferred_username=data.get("preferredUsername", ""),
            name=data.get("name", ""),
            summary=data.get("summary", ""),
            url=data.get("url", data.get("id", "")),
            inbox=data.get("inbox", ""),
            outbox=data.get("outbox", ""),
            followers=data.get("followers", ""),
            following=data.get("following", ""),
            public_key=public_key,
            icon=icon,
            image=data.get("image"),
            manually_approves_followers=data.get("manuallyApprovesFollowers", False),
            discoverable=data.get("discoverable", True),
            published=data.get("published", ""),
        )
    except Exception:
        return None


def parse_activity(data: JsonDict) -> Activity | None:
    """Parse an Activity from JSON-LD data.

    Args:
        data: JSON-LD activity document

    Returns:
        Activity instance or None if invalid
    """
    try:
        activity_type = data.get("type", "")
        if activity_type not in [t.value for t in ActivityType]:
            return None

        return Activity(
            id=data.get("id", ""),
            type=ActivityType(activity_type),
            actor=data.get("actor", ""),
            object=data.get("object", ""),
            published=data.get("published", ""),
            to=data.get("to", []) if isinstance(data.get("to"), list) else [data.get("to", "")],
            cc=data.get("cc", []) if isinstance(data.get("cc"), list) else [data.get("cc", "")],
        )
    except Exception:
        return None


def compute_content_hash(content: str) -> str:
    """Compute SHA256 hash of content for deduplication.

    Args:
        content: Content to hash

    Returns:
        Hex-encoded SHA256 hash
    """
    return hashlib.sha256(content.encode()).hexdigest()


def extract_instance_domain(actor_id: str) -> str:
    """Extract instance domain from actor ID.

    Args:
        actor_id: Full actor ID URL (e.g., https://mastodon.social/users/alice)

    Returns:
        Instance domain (e.g., mastodon.social)
    """
    from urllib.parse import urlparse
    parsed = urlparse(actor_id)
    return parsed.netloc


def extract_handle(actor_id: str, preferred_username: str) -> str:
    """Create @user@domain handle from actor ID.

    Args:
        actor_id: Full actor ID URL
        preferred_username: Actor's preferred username

    Returns:
        Handle string (e.g., @alice@mastodon.social)
    """
    domain = extract_instance_domain(actor_id)
    return f"@{preferred_username}@{domain}"
