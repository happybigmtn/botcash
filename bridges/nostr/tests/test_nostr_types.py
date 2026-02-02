"""Tests for Nostr protocol types and helpers."""

import pytest

from botcash_nostr.nostr_types import (
    NostrEvent,
    NostrFilter,
    NostrKind,
    create_contact_list,
    create_reaction,
    create_text_note,
    hex_to_note,
    hex_to_npub,
    hex_to_nsec,
    note_to_hex,
    npub_to_hex,
    nsec_to_hex,
    parse_zap_request,
)


class TestNostrEvent:
    """Tests for NostrEvent class."""

    def test_from_dict(self, sample_nostr_event):
        """Test creating NostrEvent from dictionary."""
        event = NostrEvent.from_dict(sample_nostr_event)

        assert event.id == "b" * 64
        assert event.pubkey == "a" * 64
        assert event.created_at == 1704067200
        assert event.kind == 1
        assert event.tags == []
        assert event.content == "Hello from Nostr!"
        assert event.sig == "c" * 128

    def test_to_dict(self, sample_nostr_event):
        """Test converting NostrEvent to dictionary."""
        event = NostrEvent.from_dict(sample_nostr_event)
        result = event.to_dict()

        assert result == sample_nostr_event

    def test_compute_id(self):
        """Test computing event ID hash."""
        event = NostrEvent(
            pubkey="a" * 64,
            created_at=1704067200,
            kind=1,
            tags=[],
            content="Hello!",
        )
        event_id = event.compute_id()

        # Should be a 64-char hex string
        assert len(event_id) == 64
        assert all(c in "0123456789abcdef" for c in event_id)

    def test_get_tag_values(self, sample_text_note_event):
        """Test extracting tag values."""
        event = NostrEvent.from_dict(sample_text_note_event)

        e_tags = event.get_tag_values("e")
        p_tags = event.get_tag_values("p")
        x_tags = event.get_tag_values("x")

        assert e_tags == ["e" * 64]
        assert p_tags == ["f" * 64]
        assert x_tags == []

    def test_get_reply_to(self, sample_text_note_event):
        """Test getting reply-to event ID."""
        event = NostrEvent.from_dict(sample_text_note_event)
        assert event.get_reply_to() == "e" * 64

    def test_get_reply_to_none(self, sample_nostr_event):
        """Test get_reply_to when no reply."""
        event = NostrEvent.from_dict(sample_nostr_event)
        assert event.get_reply_to() is None

    def test_get_mentions(self, sample_text_note_event):
        """Test getting mentioned pubkeys."""
        event = NostrEvent.from_dict(sample_text_note_event)
        assert event.get_mentions() == ["f" * 64]


class TestNostrFilter:
    """Tests for NostrFilter class."""

    def test_from_dict_basic(self):
        """Test creating filter from basic dictionary."""
        data = {
            "kinds": [1, 7],
            "limit": 10,
        }
        filter_ = NostrFilter.from_dict(data)

        assert filter_.kinds == [1, 7]
        assert filter_.limit == 10
        assert filter_.authors == []

    def test_from_dict_with_tags(self):
        """Test creating filter with tag filters."""
        data = {
            "kinds": [1],
            "#p": ["a" * 64, "b" * 64],
            "#e": ["c" * 64],
        }
        filter_ = NostrFilter.from_dict(data)

        assert filter_.tags["p"] == ["a" * 64, "b" * 64]
        assert filter_.tags["e"] == ["c" * 64]

    def test_matches_by_kind(self, sample_nostr_event):
        """Test filter matching by kind."""
        event = NostrEvent.from_dict(sample_nostr_event)

        filter_match = NostrFilter(kinds=[1, 7])
        filter_nomatch = NostrFilter(kinds=[4])

        assert filter_match.matches(event)
        assert not filter_nomatch.matches(event)

    def test_matches_by_author(self, sample_nostr_event):
        """Test filter matching by author."""
        event = NostrEvent.from_dict(sample_nostr_event)

        filter_match = NostrFilter(authors=["a" * 64])
        filter_nomatch = NostrFilter(authors=["z" * 64])

        assert filter_match.matches(event)
        assert not filter_nomatch.matches(event)

    def test_matches_by_since_until(self, sample_nostr_event):
        """Test filter matching by time range."""
        event = NostrEvent.from_dict(sample_nostr_event)

        filter_since_match = NostrFilter(since=1704067100)
        filter_since_nomatch = NostrFilter(since=1704067300)
        filter_until_match = NostrFilter(until=1704067300)
        filter_until_nomatch = NostrFilter(until=1704067100)

        assert filter_since_match.matches(event)
        assert not filter_since_nomatch.matches(event)
        assert filter_until_match.matches(event)
        assert not filter_until_nomatch.matches(event)


class TestBech32Conversion:
    """Tests for bech32 encoding/decoding helpers."""

    def test_npub_roundtrip(self):
        """Test npub to hex and back."""
        original_hex = "a" * 64
        npub = hex_to_npub(original_hex)
        recovered_hex = npub_to_hex(npub)

        assert npub.startswith("npub1")
        assert recovered_hex == original_hex

    def test_npub_to_hex_invalid(self):
        """Test npub_to_hex with invalid input."""
        with pytest.raises(ValueError):
            npub_to_hex("invalid")

        with pytest.raises(ValueError):
            npub_to_hex("nsec1abc")  # Wrong prefix

    def test_hex_to_npub_invalid(self):
        """Test hex_to_npub with invalid input."""
        with pytest.raises(ValueError):
            hex_to_npub("abc")  # Too short

        with pytest.raises(ValueError):
            hex_to_npub("g" * 64)  # Invalid hex

    def test_nsec_roundtrip(self):
        """Test nsec to hex and back."""
        original_hex = "b" * 64
        nsec = hex_to_nsec(original_hex)
        recovered_hex = nsec_to_hex(nsec)

        assert nsec.startswith("nsec1")
        assert recovered_hex == original_hex

    def test_note_roundtrip(self):
        """Test note to hex and back."""
        original_hex = "c" * 64
        note = hex_to_note(original_hex)
        recovered_hex = note_to_hex(note)

        assert note.startswith("note1")
        assert recovered_hex == original_hex


class TestEventCreation:
    """Tests for event creation helpers."""

    def test_create_text_note(self):
        """Test creating a text note event."""
        event = create_text_note(
            pubkey="a" * 64,
            content="Hello, world!",
        )

        assert event.kind == NostrKind.TEXT_NOTE
        assert event.content == "Hello, world!"
        assert event.pubkey == "a" * 64
        assert event.id  # Should be computed
        assert event.tags == []

    def test_create_text_note_with_reply(self):
        """Test creating a text note reply."""
        event = create_text_note(
            pubkey="a" * 64,
            content="This is a reply",
            reply_to="b" * 64,
            mentions=["c" * 64],
        )

        assert event.kind == NostrKind.TEXT_NOTE
        assert ["e", "b" * 64] in event.tags
        assert ["p", "c" * 64] in event.tags

    def test_create_reaction(self):
        """Test creating a reaction event."""
        event = create_reaction(
            pubkey="a" * 64,
            target_event_id="b" * 64,
            target_pubkey="c" * 64,
            reaction="+",
        )

        assert event.kind == NostrKind.REACTION
        assert event.content == "+"
        assert ["e", "b" * 64] in event.tags
        assert ["p", "c" * 64] in event.tags

    def test_create_contact_list(self):
        """Test creating a contact list event."""
        contacts = [
            ("b" * 64, "wss://relay.example.com", "alice"),
            ("c" * 64, "wss://relay.example.com", "bob"),
        ]
        event = create_contact_list(
            pubkey="a" * 64,
            contacts=contacts,
        )

        assert event.kind == NostrKind.CONTACTS
        assert len(event.tags) == 2


class TestZapParsing:
    """Tests for zap event parsing."""

    def test_parse_zap_request(self, sample_zap_request_event):
        """Test parsing a zap request event."""
        event = NostrEvent.from_dict(sample_zap_request_event)
        result = parse_zap_request(event)

        assert result is not None
        assert result["sender"] == "a" * 64
        assert result["recipient"] == "m" * 64
        assert result["target_event"] == "n" * 64
        assert result["amount_msats"] == 1000000
        assert result["message"] == "Great post!"

    def test_parse_zap_request_invalid_kind(self, sample_nostr_event):
        """Test parsing zap request with wrong kind."""
        event = NostrEvent.from_dict(sample_nostr_event)
        result = parse_zap_request(event)
        assert result is None

    def test_parse_zap_request_missing_recipient(self):
        """Test parsing zap request without recipient."""
        event = NostrEvent(
            id="x" * 64,
            pubkey="a" * 64,
            created_at=1704067200,
            kind=NostrKind.ZAP_REQUEST,
            tags=[],  # No p tag
            content="",
        )
        result = parse_zap_request(event)
        assert result is None
