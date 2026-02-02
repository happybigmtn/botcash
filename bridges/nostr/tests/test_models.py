"""Tests for Nostr bridge database models."""

import pytest
from datetime import datetime, timedelta, timezone

from botcash_nostr.models import (
    LinkedIdentity,
    LinkStatus,
    PrivacyMode,
    RateLimitEntry,
    RelayedMessage,
    SponsoredTransaction,
    StoredEvent,
    ZapConversion,
)


class TestLinkedIdentity:
    """Tests for LinkedIdentity model."""

    @pytest.mark.asyncio
    async def test_create_linked_identity(self, db_session, sample_nostr_pubkey, sample_botcash_address):
        """Test creating a linked identity."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            nostr_npub="npub1test",
            botcash_address=sample_botcash_address,
            status=LinkStatus.PENDING,
            challenge="test_challenge",
            challenge_expires_at=datetime.now(timezone.utc) + timedelta(minutes=10),
        )
        db_session.add(identity)
        await db_session.commit()

        assert identity.id is not None
        assert identity.nostr_pubkey == sample_nostr_pubkey
        assert identity.status == LinkStatus.PENDING
        assert identity.privacy_mode == PrivacyMode.SELECTIVE  # default

    @pytest.mark.asyncio
    async def test_identity_status_transitions(self, db_session, sample_nostr_pubkey, sample_botcash_address):
        """Test identity status transitions."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.PENDING,
        )
        db_session.add(identity)
        await db_session.commit()

        # Transition to ACTIVE
        identity.status = LinkStatus.ACTIVE
        identity.linked_at = datetime.now(timezone.utc)
        await db_session.commit()

        assert identity.status == LinkStatus.ACTIVE
        assert identity.linked_at is not None

        # Transition to UNLINKED
        identity.status = LinkStatus.UNLINKED
        identity.unlinked_at = datetime.now(timezone.utc)
        await db_session.commit()

        assert identity.status == LinkStatus.UNLINKED
        assert identity.unlinked_at is not None

    @pytest.mark.asyncio
    async def test_identity_privacy_modes(self, db_session, sample_nostr_pubkey, sample_botcash_address):
        """Test setting privacy modes."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
        )
        db_session.add(identity)
        await db_session.commit()

        for mode in PrivacyMode:
            identity.privacy_mode = mode
            await db_session.commit()
            assert identity.privacy_mode == mode


class TestRelayedMessage:
    """Tests for RelayedMessage model."""

    @pytest.mark.asyncio
    async def test_create_relayed_message(self, db_session, sample_nostr_pubkey, sample_botcash_address):
        """Test creating a relayed message."""
        # First create an identity
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        # Create relayed message
        message = RelayedMessage(
            identity_id=identity.id,
            direction="nostr_to_bc",
            nostr_event_id="a" * 64,
            nostr_kind=1,
            botcash_tx_id="b" * 64,
            message_type="post",
            content_hash="c" * 64,
            fee_sponsored=True,
            fee_amount_zatoshis=1000,
        )
        db_session.add(message)
        await db_session.commit()

        assert message.id is not None
        assert message.direction == "nostr_to_bc"
        assert message.fee_sponsored is True

    @pytest.mark.asyncio
    async def test_relayed_message_directions(self, db_session, sample_nostr_pubkey, sample_botcash_address):
        """Test both relay directions."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        # Nostr to Botcash
        msg1 = RelayedMessage(
            identity_id=identity.id,
            direction="nostr_to_bc",
            nostr_event_id="d" * 64,
            message_type="post",
            content_hash="e" * 64,
        )
        db_session.add(msg1)

        # Botcash to Nostr
        msg2 = RelayedMessage(
            identity_id=identity.id,
            direction="bc_to_nostr",
            botcash_tx_id="f" * 64,
            message_type="post",
            content_hash="g" * 64,
        )
        db_session.add(msg2)
        await db_session.commit()

        assert msg1.direction == "nostr_to_bc"
        assert msg2.direction == "bc_to_nostr"


class TestStoredEvent:
    """Tests for StoredEvent model."""

    @pytest.mark.asyncio
    async def test_create_stored_event(self, db_session):
        """Test creating a stored event."""
        event = StoredEvent(
            event_id="h" * 64,
            pubkey="i" * 64,
            kind=1,
            created_at=1704067200,
            content="Hello, Nostr!",
            tags_json="[]",
            sig="j" * 128,
            from_botcash=False,
        )
        db_session.add(event)
        await db_session.commit()

        assert event.id is not None
        assert event.event_id == "h" * 64
        assert event.from_botcash is False

    @pytest.mark.asyncio
    async def test_stored_event_from_botcash(self, db_session):
        """Test creating a stored event from Botcash."""
        event = StoredEvent(
            event_id="k" * 64,
            pubkey="l" * 64,
            kind=1,
            created_at=1704067200,
            content="Hello from Botcash!",
            tags_json="[]",
            sig="m" * 128,
            from_botcash=True,
            botcash_tx_id="n" * 64,
        )
        db_session.add(event)
        await db_session.commit()

        assert event.from_botcash is True
        assert event.botcash_tx_id == "n" * 64


class TestRateLimitEntry:
    """Tests for RateLimitEntry model."""

    @pytest.mark.asyncio
    async def test_create_rate_limit_entry(self, db_session, sample_nostr_pubkey):
        """Test creating a rate limit entry."""
        now = datetime.now(timezone.utc).replace(second=0, microsecond=0)

        entry = RateLimitEntry(
            nostr_pubkey=sample_nostr_pubkey,
            window_start=now,
            event_count=1,
        )
        db_session.add(entry)
        await db_session.commit()

        assert entry.id is not None
        assert entry.event_count == 1

    @pytest.mark.asyncio
    async def test_rate_limit_increment(self, db_session, sample_nostr_pubkey):
        """Test incrementing rate limit counter."""
        now = datetime.now(timezone.utc).replace(second=0, microsecond=0)

        entry = RateLimitEntry(
            nostr_pubkey=sample_nostr_pubkey,
            window_start=now,
            event_count=1,
        )
        db_session.add(entry)
        await db_session.commit()

        entry.event_count += 1
        await db_session.commit()

        assert entry.event_count == 2


class TestSponsoredTransaction:
    """Tests for SponsoredTransaction model."""

    @pytest.mark.asyncio
    async def test_create_sponsored_transaction(self, db_session, sample_nostr_pubkey):
        """Test creating a sponsored transaction."""
        tx = SponsoredTransaction(
            nostr_pubkey=sample_nostr_pubkey,
            tx_id="o" * 64,
            fee_zatoshis=1000,
        )
        db_session.add(tx)
        await db_session.commit()

        assert tx.id is not None
        assert tx.fee_zatoshis == 1000
        assert tx.created_at is not None


class TestZapConversion:
    """Tests for ZapConversion model."""

    @pytest.mark.asyncio
    async def test_create_zap_conversion(self, db_session, sample_botcash_address):
        """Test creating a zap conversion."""
        conversion = ZapConversion(
            zap_request_id="p" * 64,
            sender_pubkey="q" * 64,
            recipient_pubkey="r" * 64,
            recipient_botcash_address=sample_botcash_address,
            amount_msats=1000000,
            amount_zatoshis=10,
            status="pending",
        )
        db_session.add(conversion)
        await db_session.commit()

        assert conversion.id is not None
        assert conversion.status == "pending"
        assert conversion.amount_msats == 1000000

    @pytest.mark.asyncio
    async def test_zap_conversion_completion(self, db_session, sample_botcash_address):
        """Test completing a zap conversion."""
        conversion = ZapConversion(
            zap_request_id="s" * 64,
            sender_pubkey="t" * 64,
            recipient_pubkey="u" * 64,
            recipient_botcash_address=sample_botcash_address,
            amount_msats=1000000,
            amount_zatoshis=10,
            status="pending",
        )
        db_session.add(conversion)
        await db_session.commit()

        # Complete the conversion
        conversion.status = "completed"
        conversion.botcash_tx_id = "v" * 64
        conversion.completed_at = datetime.now(timezone.utc)
        await db_session.commit()

        assert conversion.status == "completed"
        assert conversion.botcash_tx_id is not None
        assert conversion.completed_at is not None

    @pytest.mark.asyncio
    async def test_zap_conversion_failure(self, db_session, sample_botcash_address):
        """Test failing a zap conversion."""
        conversion = ZapConversion(
            zap_request_id="w" * 64,
            sender_pubkey="x" * 64,
            recipient_pubkey="y" * 64,
            recipient_botcash_address=sample_botcash_address,
            amount_msats=1000000,
            amount_zatoshis=10,
            status="pending",
        )
        db_session.add(conversion)
        await db_session.commit()

        # Fail the conversion
        conversion.status = "failed"
        conversion.error_message = "Insufficient funds"
        await db_session.commit()

        assert conversion.status == "failed"
        assert conversion.error_message == "Insufficient funds"
