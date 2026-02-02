"""Tests for protocol mapper between Nostr and Botcash."""

import pytest
import json

from botcash_nostr.nostr_types import NostrEvent, NostrKind
from botcash_nostr.protocol_mapper import ProtocolMapper


class TestNostrToBotcash:
    """Tests for Nostr -> Botcash message mapping."""

    @pytest.fixture
    def mapper(self):
        """Create a protocol mapper."""
        return ProtocolMapper(zap_conversion_rate=0.00000001)

    def test_text_note_to_post(self, mapper, sample_nostr_event):
        """Test mapping text note to post."""
        event = NostrEvent.from_dict(sample_nostr_event)
        result = mapper.nostr_to_botcash(event)

        assert result is not None
        assert result.message_type == "post"
        assert result.content == "Hello from Nostr!"
        assert result.metadata["nostr_event_id"] == "b" * 64
        assert result.metadata["nostr_pubkey"] == "a" * 64

    def test_text_note_reply_to_reply(self, mapper, sample_text_note_event):
        """Test mapping text note reply."""
        event = NostrEvent.from_dict(sample_text_note_event)
        result = mapper.nostr_to_botcash(event)

        assert result is not None
        assert result.message_type == "reply"
        assert result.reply_to == "e" * 64
        assert result.mentions == ["f" * 64]
        assert "hashtag" in result.metadata.get("tags", [])

    def test_dm_to_dm(self, mapper, sample_dm_event):
        """Test mapping encrypted DM."""
        event = NostrEvent.from_dict(sample_dm_event)
        result = mapper.nostr_to_botcash(event)

        assert result is not None
        assert result.message_type == "dm"
        assert result.content == "encrypted_content_here"
        assert result.metadata["recipient_pubkey"] == "h" * 64
        assert result.metadata["encrypted"] is True

    def test_reaction_to_upvote(self, mapper, sample_reaction_event):
        """Test mapping reaction to upvote."""
        event = NostrEvent.from_dict(sample_reaction_event)
        result = mapper.nostr_to_botcash(event)

        assert result is not None
        assert result.message_type == "upvote"
        assert result.content == "+"
        assert result.metadata["target_event_id"] == "j" * 64
        assert result.metadata["target_pubkey"] == "k" * 64

    def test_reaction_downvote(self, mapper):
        """Test mapping negative reaction to downvote."""
        event = NostrEvent(
            id="z" * 64,
            pubkey="a" * 64,
            created_at=1704067200,
            kind=NostrKind.REACTION,
            tags=[["e", "b" * 64], ["p", "c" * 64]],
            content="-",
            sig="d" * 128,
        )
        result = mapper.nostr_to_botcash(event)

        assert result is not None
        assert result.message_type == "downvote"

    def test_contacts_to_follow_list(self, mapper, sample_contacts_event):
        """Test mapping contacts to follow list."""
        event = NostrEvent.from_dict(sample_contacts_event)
        result = mapper.nostr_to_botcash(event)

        assert result is not None
        assert result.message_type == "follow_list"
        assert len(result.metadata["follows"]) == 2

    def test_metadata_to_profile(self, mapper, sample_metadata_event):
        """Test mapping metadata to profile."""
        event = NostrEvent.from_dict(sample_metadata_event)
        result = mapper.nostr_to_botcash(event)

        assert result is not None
        assert result.message_type == "profile"
        assert result.metadata["name"] == "alice"
        assert result.metadata["about"] == "Hello!"
        assert "picture" in result.metadata

    def test_zap_request_to_tip(self, mapper, sample_zap_request_event):
        """Test mapping zap request to tip request."""
        event = NostrEvent.from_dict(sample_zap_request_event)
        result = mapper.nostr_to_botcash(event)

        assert result is not None
        assert result.message_type == "tip_request"
        assert result.metadata["amount_msats"] == 1000000
        assert result.metadata["amount_sats"] == 1000
        assert result.metadata["amount_bcash"] == 1000 * 0.00000001

    def test_unsupported_kind(self, mapper):
        """Test handling unsupported event kinds."""
        event = NostrEvent(
            id="z" * 64,
            pubkey="a" * 64,
            created_at=1704067200,
            kind=99999,  # Unsupported kind
            tags=[],
            content="",
            sig="b" * 128,
        )
        result = mapper.nostr_to_botcash(event)
        assert result is None


class TestBotcashToNostr:
    """Tests for Botcash -> Nostr message mapping."""

    @pytest.fixture
    def mapper(self):
        """Create a protocol mapper."""
        return ProtocolMapper()

    def test_post_to_text_note(self, mapper, sample_nostr_pubkey):
        """Test mapping Botcash post to text note."""
        event = mapper.botcash_to_nostr(
            message_type="post",
            content="Hello from Botcash!",
            author_pubkey=sample_nostr_pubkey,
            metadata={"botcash_tx_id": "x" * 64},
        )

        assert event is not None
        assert event.kind == NostrKind.TEXT_NOTE
        assert "Hello from Botcash!" in event.content
        assert "Posted via Botcash" in event.content
        assert event.pubkey == sample_nostr_pubkey

    def test_reply_to_text_note(self, mapper, sample_nostr_pubkey):
        """Test mapping Botcash reply to text note."""
        event = mapper.botcash_to_nostr(
            message_type="reply",
            content="This is a reply",
            author_pubkey=sample_nostr_pubkey,
            metadata={
                "reply_to_event": "y" * 64,
                "mentions": ["z" * 64],
            },
        )

        assert event is not None
        assert event.kind == NostrKind.TEXT_NOTE
        assert ["e", "y" * 64] in event.tags
        assert ["p", "z" * 64] in event.tags

    def test_dm_to_encrypted_dm(self, mapper, sample_nostr_pubkey):
        """Test mapping Botcash DM to encrypted DM."""
        event = mapper.botcash_to_nostr(
            message_type="dm",
            content="encrypted_content",
            author_pubkey=sample_nostr_pubkey,
            metadata={"recipient_pubkey": "a" * 64},
        )

        assert event is not None
        assert event.kind == NostrKind.ENCRYPTED_DM
        assert ["p", "a" * 64] in event.tags

    def test_follow_to_contacts(self, mapper, sample_nostr_pubkey):
        """Test mapping Botcash follow to contacts."""
        event = mapper.botcash_to_nostr(
            message_type="follow",
            content="",
            author_pubkey=sample_nostr_pubkey,
            metadata={
                "follows": ["b" * 64, "c" * 64],
                "relay_url": "wss://relay.example.com",
            },
        )

        assert event is not None
        assert event.kind == NostrKind.CONTACTS
        assert len(event.tags) == 2

    def test_profile_to_metadata(self, mapper, sample_nostr_pubkey, sample_botcash_address):
        """Test mapping Botcash profile to metadata."""
        event = mapper.botcash_to_nostr(
            message_type="profile",
            content='{"name": "alice"}',
            author_pubkey=sample_nostr_pubkey,
            metadata={
                "botcash_address": sample_botcash_address,
                "about": "Updated bio",
            },
        )

        assert event is not None
        assert event.kind == NostrKind.METADATA

        profile = json.loads(event.content)
        assert profile["name"] == "alice"
        assert profile["about"] == "Updated bio"
        assert profile["botcash_address"] == sample_botcash_address

    def test_upvote_to_reaction(self, mapper, sample_nostr_pubkey):
        """Test mapping Botcash upvote to reaction."""
        event = mapper.botcash_to_nostr(
            message_type="upvote",
            content="",
            author_pubkey=sample_nostr_pubkey,
            metadata={
                "target_event_id": "d" * 64,
                "target_pubkey": "e" * 64,
            },
        )

        assert event is not None
        assert event.kind == NostrKind.REACTION
        assert event.content == "+"
        assert ["e", "d" * 64] in event.tags
        assert ["p", "e" * 64] in event.tags

    def test_upvote_without_target_returns_none(self, mapper, sample_nostr_pubkey):
        """Test that upvote without target returns None."""
        event = mapper.botcash_to_nostr(
            message_type="upvote",
            content="",
            author_pubkey=sample_nostr_pubkey,
            metadata={},  # No target
        )
        assert event is None

    def test_tip_to_zap_receipt(self, mapper, sample_nostr_pubkey):
        """Test mapping Botcash tip to zap receipt."""
        event = mapper.botcash_to_nostr(
            message_type="tip",
            content="",
            author_pubkey=sample_nostr_pubkey,
            metadata={
                "recipient_pubkey": "f" * 64,
                "amount_bcash": 0.00001,
                "target_event_id": "g" * 64,
                "botcash_tx_id": "h" * 64,
            },
        )

        assert event is not None
        assert event.kind == NostrKind.ZAP_RECEIPT
        assert ["p", "f" * 64] in event.tags
        assert ["e", "g" * 64] in event.tags

    def test_unsupported_type(self, mapper, sample_nostr_pubkey):
        """Test handling unsupported message types."""
        event = mapper.botcash_to_nostr(
            message_type="unknown_type",
            content="",
            author_pubkey=sample_nostr_pubkey,
        )
        assert event is None


class TestContentHash:
    """Tests for content hash computation."""

    def test_content_hash_consistency(self):
        """Test that same content produces same hash."""
        mapper = ProtocolMapper()
        content = "Hello, world!"

        hash1 = mapper.compute_content_hash(content)
        hash2 = mapper.compute_content_hash(content)

        assert hash1 == hash2
        assert len(hash1) == 64  # SHA256 hex

    def test_content_hash_uniqueness(self):
        """Test that different content produces different hash."""
        mapper = ProtocolMapper()

        hash1 = mapper.compute_content_hash("Hello")
        hash2 = mapper.compute_content_hash("World")

        assert hash1 != hash2
