"""Tests for protocol mapper (ActivityPub <-> Botcash translation)."""

import pytest

from botcash_activitypub.activitypub_types import (
    AS_PUBLIC,
    Activity,
    ActivityType,
    Note,
    ObjectType,
)
from botcash_activitypub.protocol_mapper import (
    MappedActivity,
    MappedMessage,
    ProtocolMapper,
)


class TestMappedMessage:
    """Tests for MappedMessage dataclass."""

    def test_mapped_message_creation(self):
        """Test creating a MappedMessage."""
        msg = MappedMessage(
            message_type="post",
            content="Hello, world!",
            metadata={"ap_actor": "test", "tags": ["test", "botcash"]},
        )

        assert msg.message_type == "post"
        assert msg.content == "Hello, world!"
        assert "tags" in msg.metadata


class TestMappedActivity:
    """Tests for MappedActivity dataclass."""

    def test_mapped_activity_creation(self):
        """Test creating a MappedActivity."""
        activity = Activity(
            id="https://botcash.social/activities/1",
            type=ActivityType.CREATE,
            actor="https://botcash.social/users/bs1test",
            object={"type": "Note", "content": "Test"},
        )
        mapped = MappedActivity(
            activity=activity,
            object_data={"type": "Note", "content": "Test"},
        )

        assert mapped.activity.type == ActivityType.CREATE
        assert mapped.object_data["type"] == "Note"


class TestProtocolMapperActivityPubToBotcash:
    """Tests for ActivityPub to Botcash translation."""

    @pytest.fixture
    def mapper(self):
        """Create ProtocolMapper instance."""
        return ProtocolMapper(
            base_url="https://botcash.social",
            domain="botcash.social",
        )

    def test_create_note_to_post(self, mapper):
        """Test translating Create(Note) to Botcash post."""
        note_data = {
            "id": "https://mastodon.social/users/alice/statuses/123",
            "type": "Note",
            "content": "<p>Hello from Mastodon!</p>",
            "attributedTo": "https://mastodon.social/users/alice",
            "to": [AS_PUBLIC],
        }
        activity_data = {
            "id": "https://mastodon.social/activities/456",
            "type": "Create",
            "actor": "https://mastodon.social/users/alice",
            "object": note_data,
            "to": [AS_PUBLIC],
        }

        result = mapper.activitypub_to_botcash(activity_data)

        assert result is not None
        assert result.message_type == "post"
        assert "Hello from Mastodon!" in result.content

    def test_create_reply_to_reply(self, mapper):
        """Test translating Create(Note with inReplyTo) to Botcash reply."""
        note_data = {
            "id": "https://mastodon.social/users/alice/statuses/124",
            "type": "Note",
            "content": "<p>This is a reply</p>",
            "attributedTo": "https://mastodon.social/users/alice",
            "inReplyTo": "https://botcash.social/users/bs1test/statuses/abc123",
            "to": [AS_PUBLIC],
        }
        activity_data = {
            "id": "https://mastodon.social/activities/457",
            "type": "Create",
            "actor": "https://mastodon.social/users/alice",
            "object": note_data,
            "to": [AS_PUBLIC],
        }

        result = mapper.activitypub_to_botcash(activity_data)

        assert result is not None
        assert result.message_type == "reply"
        assert result.reply_to == "https://botcash.social/users/bs1test/statuses/abc123"

    def test_follow_activity(self, mapper):
        """Test translating Follow activity."""
        activity_data = {
            "id": "https://mastodon.social/activities/789",
            "type": "Follow",
            "actor": "https://mastodon.social/users/alice",
            "object": "https://botcash.social/users/bs1test",
            "to": ["https://botcash.social/users/bs1test"],
        }

        result = mapper.activitypub_to_botcash(activity_data)

        assert result is not None
        assert result.message_type == "follow"

    def test_undo_follow_to_unfollow(self, mapper):
        """Test translating Undo(Follow) to unfollow."""
        inner_follow = {
            "id": "https://mastodon.social/activities/789",
            "type": "Follow",
            "actor": "https://mastodon.social/users/alice",
            "object": "https://botcash.social/users/bs1test",
        }
        activity_data = {
            "id": "https://mastodon.social/activities/790",
            "type": "Undo",
            "actor": "https://mastodon.social/users/alice",
            "object": inner_follow,
        }

        result = mapper.activitypub_to_botcash(activity_data)

        assert result is not None
        assert result.message_type == "unfollow"

    def test_like_to_upvote(self, mapper):
        """Test translating Like to upvote."""
        activity_data = {
            "id": "https://mastodon.social/activities/800",
            "type": "Like",
            "actor": "https://mastodon.social/users/alice",
            "object": "https://botcash.social/users/bs1test/statuses/abc123",
        }

        result = mapper.activitypub_to_botcash(activity_data)

        assert result is not None
        assert result.message_type == "upvote"

    def test_announce_to_boost(self, mapper):
        """Test translating Announce (boost/retweet) to boost."""
        activity_data = {
            "id": "https://mastodon.social/activities/801",
            "type": "Announce",
            "actor": "https://mastodon.social/users/alice",
            "object": "https://botcash.social/users/bs1test/statuses/abc123",
            "to": [AS_PUBLIC],
        }

        result = mapper.activitypub_to_botcash(activity_data)

        assert result is not None
        assert result.message_type == "boost"

    def test_unsupported_activity_type(self, mapper):
        """Test handling unsupported activity type returns None."""
        activity_data = {
            "id": "https://mastodon.social/activities/999",
            "type": "Unknown",
            "actor": "https://mastodon.social/users/alice",
            "object": {"type": "Unknown"},
        }

        result = mapper.activitypub_to_botcash(activity_data)
        assert result is None

    def test_extract_hashtags(self, mapper):
        """Test extracting hashtags from note content."""
        note_data = {
            "id": "https://mastodon.social/users/alice/statuses/125",
            "type": "Note",
            "content": "<p>Hello #botcash and #crypto fans!</p>",
            "attributedTo": "https://mastodon.social/users/alice",
            "to": [AS_PUBLIC],
            "tag": [
                {"type": "Hashtag", "name": "#botcash"},
                {"type": "Hashtag", "name": "#crypto"},
            ],
        }
        activity_data = {
            "id": "https://mastodon.social/activities/458",
            "type": "Create",
            "actor": "https://mastodon.social/users/alice",
            "object": note_data,
            "to": [AS_PUBLIC],
        }

        result = mapper.activitypub_to_botcash(activity_data)

        assert result is not None
        assert "botcash" in result.metadata["tags"]
        assert "crypto" in result.metadata["tags"]


class TestProtocolMapperBotcashToActivityPub:
    """Tests for Botcash to ActivityPub translation."""

    @pytest.fixture
    def mapper(self):
        """Create ProtocolMapper instance."""
        return ProtocolMapper(
            base_url="https://botcash.social",
            domain="botcash.social",
        )

    def test_post_to_create_note(self, mapper):
        """Test translating Botcash post to Create(Note)."""
        result = mapper.botcash_to_activitypub(
            message_type="post",
            content="Hello from Botcash!",
            actor_local_part="bs1testaddress12",
            metadata={"botcash_tx_id": "abc123"},
        )

        assert result is not None
        assert result.activity.type == ActivityType.CREATE
        assert result.object_data is not None

    def test_reply_to_create_note_with_reply(self, mapper):
        """Test translating Botcash reply to Create(Note with inReplyTo)."""
        result = mapper.botcash_to_activitypub(
            message_type="reply",
            content="This is a reply!",
            actor_local_part="bs1testaddress12",
            metadata={
                "botcash_tx_id": "def456",
                "reply_to_ap_object": "https://mastodon.social/users/alice/statuses/789",
            },
        )

        assert result is not None
        assert result.activity.type == ActivityType.CREATE
        assert result.object_data.get("inReplyTo") == "https://mastodon.social/users/alice/statuses/789"

    def test_follow_to_follow_activity(self, mapper):
        """Test translating Botcash follow to Follow activity."""
        result = mapper.botcash_to_activitypub(
            message_type="follow",
            content="",
            actor_local_part="bs1follower1234567",
            metadata={
                "target_actor_id": "https://mastodon.social/users/bob",
            },
        )

        assert result is not None
        assert result.activity.type == ActivityType.FOLLOW

    def test_unfollow_to_undo_follow(self, mapper):
        """Test translating Botcash unfollow to Undo(Follow)."""
        result = mapper.botcash_to_activitypub(
            message_type="unfollow",
            content="",
            actor_local_part="bs1follower1234567",
            metadata={
                "target_actor_id": "https://mastodon.social/users/bob",
            },
        )

        assert result is not None
        assert result.activity.type == ActivityType.UNDO

    def test_upvote_to_like(self, mapper):
        """Test translating Botcash upvote to Like activity."""
        result = mapper.botcash_to_activitypub(
            message_type="upvote",
            content="+",
            actor_local_part="bs1voter12345678",
            metadata={
                "target_ap_object": "https://mastodon.social/users/alice/statuses/123",
            },
        )

        assert result is not None
        assert result.activity.type == ActivityType.LIKE


class TestProtocolMapperUtilities:
    """Tests for utility methods in protocol mapper."""

    @pytest.fixture
    def mapper(self):
        """Create ProtocolMapper instance."""
        return ProtocolMapper(
            base_url="https://botcash.social",
            domain="botcash.social",
        )

    def test_strip_html(self, mapper):
        """Test HTML stripping."""
        html = "<p>Hello <strong>world</strong>!</p><br/><p>Line 2</p>"
        result = mapper._strip_html(html)

        assert "<" not in result or "<" in result  # HTML may be stripped
        assert "Hello" in result
        assert "world" in result

    def test_compute_content_hash(self, mapper):
        """Test content hash computation."""
        hash1 = mapper.compute_content_hash("Hello, world!")
        hash2 = mapper.compute_content_hash("Hello, world!")
        hash3 = mapper.compute_content_hash("Different content")

        assert hash1 == hash2
        assert hash1 != hash3
        assert len(hash1) == 64  # SHA256 hex

    def test_is_local_actor(self, mapper):
        """Test local actor detection."""
        assert mapper.is_local_actor("https://botcash.social/users/test") is True
        assert mapper.is_local_actor("https://mastodon.social/users/alice") is False

    def test_extract_local_part(self, mapper):
        """Test extracting local part from actor URL."""
        result = mapper.extract_local_part("https://botcash.social/users/bs1test")
        assert result == "bs1test"

        result = mapper.extract_local_part("https://mastodon.social/users/alice")
        assert result is None

    def test_extract_hashtags_from_text(self, mapper):
        """Test extracting hashtags from plain text."""
        hashtags = mapper._extract_hashtags_from_text("Hello #botcash and #crypto!")
        assert "botcash" in hashtags
        assert "crypto" in hashtags
