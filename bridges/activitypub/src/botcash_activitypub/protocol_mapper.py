"""Protocol mapper for bidirectional ActivityPub <-> Botcash message translation.

Maps between ActivityPub activities and Botcash social protocol messages.

Protocol mapping (from specs/bridges.md):
| Botcash | ActivityPub |
|---------|-------------|
| Post | Create (Note) |
| Follow | Follow |
| Upvote | Like + Announce |
| Comment | Create (in reply to) |
"""

import hashlib
import html
import re
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any

import structlog

from .activitypub_types import (
    AS_PUBLIC,
    Activity,
    ActivityType,
    Note,
    ObjectType,
    create_activity,
    create_note,
    parse_activity,
)

logger = structlog.get_logger()


@dataclass
class MappedMessage:
    """Result of mapping a message between protocols."""
    message_type: str  # post, reply, follow, unfollow, like, boost, dm
    content: str
    metadata: dict[str, Any]
    reply_to: str | None = None
    mentions: list[str] | None = None


@dataclass
class MappedActivity:
    """Result of mapping Botcash message to ActivityPub activity."""
    activity: Activity
    object_data: dict[str, Any] | None = None


class ProtocolMapper:
    """Maps messages between ActivityPub and Botcash protocols."""

    def __init__(self, base_url: str, domain: str):
        """Initialize mapper.

        Args:
            base_url: ActivityPub server base URL (e.g., https://botcash.social)
            domain: Domain for actor handles (e.g., botcash.social)
        """
        self.base_url = base_url.rstrip("/")
        self.domain = domain

    # === ActivityPub -> Botcash Mappings ===

    def activitypub_to_botcash(
        self,
        activity_data: dict[str, Any],
    ) -> MappedMessage | None:
        """Convert an ActivityPub activity to Botcash message format.

        Args:
            activity_data: JSON-LD activity document

        Returns:
            MappedMessage or None if activity type not supported
        """
        activity = parse_activity(activity_data)
        if not activity:
            logger.debug("Failed to parse activity", data=activity_data)
            return None

        if activity.type == ActivityType.CREATE:
            return self._map_create_to_post(activity_data)
        elif activity.type == ActivityType.FOLLOW:
            return self._map_follow(activity_data)
        elif activity.type == ActivityType.UNDO:
            return self._map_undo(activity_data)
        elif activity.type == ActivityType.LIKE:
            return self._map_like_to_upvote(activity_data)
        elif activity.type == ActivityType.ANNOUNCE:
            return self._map_announce_to_boost(activity_data)
        elif activity.type == ActivityType.DELETE:
            return self._map_delete(activity_data)
        else:
            logger.debug("Unsupported activity type", type=activity.type)
            return None

    def _map_create_to_post(
        self,
        activity_data: dict[str, Any],
    ) -> MappedMessage | None:
        """Map ActivityPub Create (Note) to Botcash post or reply."""
        activity_id = activity_data.get("id", "")
        actor = activity_data.get("actor", "")
        obj = activity_data.get("object", {})

        # Handle object as ID or inline
        if isinstance(obj, str):
            # Object is just an ID, can't process without fetching
            logger.debug("Create activity has object ID only, skipping", object_id=obj)
            return None

        obj_type = obj.get("type", "")
        if obj_type != ObjectType.NOTE.value:
            logger.debug("Create activity object is not a Note", type=obj_type)
            return None

        content = obj.get("content", "")
        # Strip HTML tags for Botcash (which uses plain text)
        plain_content = self._strip_html(content)

        in_reply_to = obj.get("inReplyTo")
        mentions = self._extract_mentions(obj.get("tag", []))
        hashtags = self._extract_hashtags(obj.get("tag", []))
        sensitive = obj.get("sensitive", False)
        summary = obj.get("summary")  # Content warning

        message_type = "reply" if in_reply_to else "post"

        return MappedMessage(
            message_type=message_type,
            content=plain_content,
            metadata={
                "ap_activity_id": activity_id,
                "ap_object_id": obj.get("id", ""),
                "ap_actor": actor,
                "created_at": obj.get("published", ""),
                "tags": hashtags,
                "sensitive": sensitive,
                "content_warning": summary,
            },
            reply_to=in_reply_to,
            mentions=mentions,
        )

    def _map_follow(
        self,
        activity_data: dict[str, Any],
    ) -> MappedMessage:
        """Map ActivityPub Follow to Botcash follow."""
        return MappedMessage(
            message_type="follow",
            content="",
            metadata={
                "ap_activity_id": activity_data.get("id", ""),
                "ap_actor": activity_data.get("actor", ""),
                "target_actor": activity_data.get("object", ""),
            },
        )

    def _map_undo(
        self,
        activity_data: dict[str, Any],
    ) -> MappedMessage | None:
        """Map ActivityPub Undo to appropriate Botcash action."""
        obj = activity_data.get("object", {})
        if isinstance(obj, str):
            return None

        obj_type = obj.get("type", "")
        if obj_type == ActivityType.FOLLOW.value:
            return MappedMessage(
                message_type="unfollow",
                content="",
                metadata={
                    "ap_activity_id": activity_data.get("id", ""),
                    "ap_actor": activity_data.get("actor", ""),
                    "target_actor": obj.get("object", ""),
                },
            )
        elif obj_type == ActivityType.LIKE.value:
            return MappedMessage(
                message_type="unlike",
                content="",
                metadata={
                    "ap_activity_id": activity_data.get("id", ""),
                    "ap_actor": activity_data.get("actor", ""),
                    "target_object": obj.get("object", ""),
                },
            )
        elif obj_type == ActivityType.ANNOUNCE.value:
            return MappedMessage(
                message_type="unboost",
                content="",
                metadata={
                    "ap_activity_id": activity_data.get("id", ""),
                    "ap_actor": activity_data.get("actor", ""),
                    "target_object": obj.get("object", ""),
                },
            )
        return None

    def _map_like_to_upvote(
        self,
        activity_data: dict[str, Any],
    ) -> MappedMessage:
        """Map ActivityPub Like to Botcash upvote."""
        return MappedMessage(
            message_type="upvote",
            content="+",
            metadata={
                "ap_activity_id": activity_data.get("id", ""),
                "ap_actor": activity_data.get("actor", ""),
                "target_object": activity_data.get("object", ""),
            },
        )

    def _map_announce_to_boost(
        self,
        activity_data: dict[str, Any],
    ) -> MappedMessage:
        """Map ActivityPub Announce (boost/reblog) to Botcash upvote.

        Note: Botcash doesn't have a separate boost concept, so we treat
        Announce as a strong upvote signal.
        """
        return MappedMessage(
            message_type="boost",
            content="",
            metadata={
                "ap_activity_id": activity_data.get("id", ""),
                "ap_actor": activity_data.get("actor", ""),
                "target_object": activity_data.get("object", ""),
            },
        )

    def _map_delete(
        self,
        activity_data: dict[str, Any],
    ) -> MappedMessage:
        """Map ActivityPub Delete.

        Note: Botcash transactions are immutable, so Delete is recorded
        but the content cannot actually be removed from the chain.
        """
        return MappedMessage(
            message_type="delete",
            content="",
            metadata={
                "ap_activity_id": activity_data.get("id", ""),
                "ap_actor": activity_data.get("actor", ""),
                "target_object": activity_data.get("object", ""),
            },
        )

    # === Botcash -> ActivityPub Mappings ===

    def botcash_to_activitypub(
        self,
        message_type: str,
        content: str,
        actor_local_part: str,
        metadata: dict[str, Any] | None = None,
    ) -> MappedActivity | None:
        """Convert a Botcash message to ActivityPub activity.

        Args:
            message_type: Botcash message type (post, dm, follow, etc.)
            content: Message content
            actor_local_part: Actor's local username
            metadata: Additional metadata (botcash_tx_id, reply_to_event, etc.)

        Returns:
            MappedActivity or None if type not supported
        """
        metadata = metadata or {}

        if message_type == "post":
            return self._map_post_to_create(content, actor_local_part, metadata)
        elif message_type == "reply":
            return self._map_reply_to_create(content, actor_local_part, metadata)
        elif message_type == "follow":
            return self._map_botcash_follow(actor_local_part, metadata)
        elif message_type == "unfollow":
            return self._map_botcash_unfollow(actor_local_part, metadata)
        elif message_type == "upvote":
            return self._map_upvote_to_like(actor_local_part, metadata)
        elif message_type == "profile":
            # Profile updates are handled via actor document updates
            return None
        else:
            logger.debug("Unsupported Botcash message type", message_type=message_type)
            return None

    def _map_post_to_create(
        self,
        content: str,
        actor_local_part: str,
        metadata: dict[str, Any],
    ) -> MappedActivity:
        """Map Botcash post to ActivityPub Create (Note)."""
        # Extract mentions and hashtags from content
        mentions = metadata.get("mentions", [])
        hashtags = self._extract_hashtags_from_text(content)

        # Convert plain text to HTML
        html_content = self._text_to_html(content)

        # Add Botcash attribution
        if metadata.get("botcash_tx_id"):
            html_content += f'<p><small>— Posted via Botcash (tx: {metadata["botcash_tx_id"][:8]}...)</small></p>'

        # Generate unique note ID
        note_id = metadata.get("botcash_tx_id", str(int(time.time() * 1000)))

        # Build note
        note = create_note(
            base_url=self.base_url,
            actor_local_part=actor_local_part,
            content=html_content,
            note_id=note_id,
            sensitive=metadata.get("sensitive", False),
            content_warning=metadata.get("content_warning"),
            mentions=mentions,
            hashtags=hashtags,
            botcash_tx_id=metadata.get("botcash_tx_id", ""),
        )

        # Wrap in Create activity
        activity = create_activity(
            base_url=self.base_url,
            actor_local_part=actor_local_part,
            activity_type=ActivityType.CREATE,
            activity_object=note.to_dict(),
        )

        return MappedActivity(activity=activity, object_data=note.to_dict())

    def _map_reply_to_create(
        self,
        content: str,
        actor_local_part: str,
        metadata: dict[str, Any],
    ) -> MappedActivity:
        """Map Botcash reply to ActivityPub Create (Note) with inReplyTo."""
        mentions = metadata.get("mentions", [])
        hashtags = self._extract_hashtags_from_text(content)

        # Convert plain text to HTML
        html_content = self._text_to_html(content)

        # Add Botcash attribution
        if metadata.get("botcash_tx_id"):
            html_content += f'<p><small>— Posted via Botcash (tx: {metadata["botcash_tx_id"][:8]}...)</small></p>'

        note_id = metadata.get("botcash_tx_id", str(int(time.time() * 1000)))

        # Build note with inReplyTo
        note = create_note(
            base_url=self.base_url,
            actor_local_part=actor_local_part,
            content=html_content,
            note_id=note_id,
            in_reply_to=metadata.get("reply_to_ap_object"),
            sensitive=metadata.get("sensitive", False),
            content_warning=metadata.get("content_warning"),
            mentions=mentions,
            hashtags=hashtags,
            botcash_tx_id=metadata.get("botcash_tx_id", ""),
        )

        activity = create_activity(
            base_url=self.base_url,
            actor_local_part=actor_local_part,
            activity_type=ActivityType.CREATE,
            activity_object=note.to_dict(),
        )

        return MappedActivity(activity=activity, object_data=note.to_dict())

    def _map_botcash_follow(
        self,
        actor_local_part: str,
        metadata: dict[str, Any],
    ) -> MappedActivity:
        """Map Botcash follow to ActivityPub Follow."""
        target_actor = metadata.get("target_actor_id", "")

        activity = create_activity(
            base_url=self.base_url,
            actor_local_part=actor_local_part,
            activity_type=ActivityType.FOLLOW,
            activity_object=target_actor,
        )

        return MappedActivity(activity=activity)

    def _map_botcash_unfollow(
        self,
        actor_local_part: str,
        metadata: dict[str, Any],
    ) -> MappedActivity:
        """Map Botcash unfollow to ActivityPub Undo(Follow)."""
        target_actor = metadata.get("target_actor_id", "")

        # Create the original Follow activity (to be undone)
        original_follow = {
            "type": ActivityType.FOLLOW.value,
            "actor": f"{self.base_url}/users/{actor_local_part}",
            "object": target_actor,
        }

        activity = create_activity(
            base_url=self.base_url,
            actor_local_part=actor_local_part,
            activity_type=ActivityType.UNDO,
            activity_object=original_follow,
        )

        return MappedActivity(activity=activity)

    def _map_upvote_to_like(
        self,
        actor_local_part: str,
        metadata: dict[str, Any],
    ) -> MappedActivity:
        """Map Botcash upvote to ActivityPub Like."""
        target_object = metadata.get("target_ap_object", "")

        activity = create_activity(
            base_url=self.base_url,
            actor_local_part=actor_local_part,
            activity_type=ActivityType.LIKE,
            activity_object=target_object,
        )

        return MappedActivity(activity=activity)

    # === Utility Methods ===

    def _strip_html(self, html_content: str) -> str:
        """Strip HTML tags from content.

        Args:
            html_content: HTML content

        Returns:
            Plain text content
        """
        # Simple HTML tag stripping
        text = re.sub(r'<br\s*/?>', '\n', html_content)
        text = re.sub(r'<p\s*/?>', '', text)
        text = re.sub(r'</p>', '\n', text)
        text = re.sub(r'<[^>]+>', '', text)
        text = html.unescape(text)
        return text.strip()

    def _text_to_html(self, text: str) -> str:
        """Convert plain text to simple HTML.

        Args:
            text: Plain text content

        Returns:
            HTML content
        """
        # Escape HTML entities
        escaped = html.escape(text)
        # Convert newlines to <br>
        html_content = escaped.replace('\n', '<br>')
        # Wrap in paragraph
        return f"<p>{html_content}</p>"

    def _extract_mentions(self, tags: list[dict]) -> list[str]:
        """Extract mention actor IDs from ActivityPub tags.

        Args:
            tags: List of tag objects

        Returns:
            List of actor IDs
        """
        mentions = []
        for tag in tags:
            if tag.get("type") == "Mention" and tag.get("href"):
                mentions.append(tag["href"])
        return mentions

    def _extract_hashtags(self, tags: list[dict]) -> list[str]:
        """Extract hashtag names from ActivityPub tags.

        Args:
            tags: List of tag objects

        Returns:
            List of hashtag names (without #)
        """
        hashtags = []
        for tag in tags:
            if tag.get("type") == "Hashtag" and tag.get("name"):
                name = tag["name"].lstrip("#")
                hashtags.append(name)
        return hashtags

    def _extract_hashtags_from_text(self, text: str) -> list[str]:
        """Extract hashtags from plain text.

        Args:
            text: Plain text content

        Returns:
            List of hashtag names (without #)
        """
        # Match #hashtag patterns
        matches = re.findall(r'#(\w+)', text)
        return list(set(matches))

    def compute_content_hash(self, content: str) -> str:
        """Compute SHA256 hash of content for deduplication.

        Args:
            content: Content to hash

        Returns:
            Hex-encoded SHA256 hash
        """
        return hashlib.sha256(content.encode()).hexdigest()

    def is_local_actor(self, actor_id: str) -> bool:
        """Check if an actor ID belongs to this server.

        Args:
            actor_id: Full actor ID URL

        Returns:
            True if local actor
        """
        return actor_id.startswith(self.base_url)

    def extract_local_part(self, actor_id: str) -> str | None:
        """Extract local part from a local actor ID.

        Args:
            actor_id: Full actor ID URL

        Returns:
            Local part or None if not a local actor
        """
        if not self.is_local_actor(actor_id):
            return None

        prefix = f"{self.base_url}/users/"
        if actor_id.startswith(prefix):
            return actor_id[len(prefix):]
        return None
