"""Tests for database models."""

import pytest
from datetime import datetime, timezone

from botcash_telegram.models import (
    LinkedIdentity,
    LinkStatus,
    PrivacyMode,
    RelayedMessage,
    RateLimitEntry,
    SponsoredTransaction,
)


class TestLinkedIdentity:
    """Tests for LinkedIdentity model."""

    async def test_create_linked_identity(self, db_session):
        """Test creating a linked identity."""
        identity = LinkedIdentity(
            telegram_user_id=12345,
            telegram_username="testuser",
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.PENDING,
            challenge="c" * 64,
        )
        db_session.add(identity)
        await db_session.commit()

        assert identity.id is not None
        assert identity.status == LinkStatus.PENDING
        assert identity.privacy_mode == PrivacyMode.SELECTIVE  # default
        assert identity.created_at is not None

    async def test_linked_identity_status_transitions(self, db_session):
        """Test identity status transitions."""
        identity = LinkedIdentity(
            telegram_user_id=12345,
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.PENDING,
        )
        db_session.add(identity)
        await db_session.commit()

        # Transition to active
        identity.status = LinkStatus.ACTIVE
        identity.linked_at = datetime.now(timezone.utc)
        await db_session.commit()

        assert identity.status == LinkStatus.ACTIVE
        assert identity.linked_at is not None

        # Transition to unlinked
        identity.status = LinkStatus.UNLINKED
        identity.unlinked_at = datetime.now(timezone.utc)
        await db_session.commit()

        assert identity.status == LinkStatus.UNLINKED
        assert identity.unlinked_at is not None

    async def test_unique_telegram_user_id(self, db_session):
        """Test that telegram_user_id is unique."""
        identity1 = LinkedIdentity(
            telegram_user_id=12345,
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity1)
        await db_session.commit()

        # Try to add another with same telegram_user_id
        identity2 = LinkedIdentity(
            telegram_user_id=12345,
            botcash_address="bs1" + "b" * 59,
            status=LinkStatus.PENDING,
        )
        db_session.add(identity2)

        with pytest.raises(Exception):  # IntegrityError
            await db_session.commit()


class TestRelayedMessage:
    """Tests for RelayedMessage model."""

    async def test_create_relayed_message(self, db_session):
        """Test creating a relayed message record."""
        # First create identity
        identity = LinkedIdentity(
            telegram_user_id=12345,
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        # Create relayed message
        message = RelayedMessage(
            identity_id=identity.id,
            direction="tg_to_bc",
            telegram_message_id=100,
            telegram_chat_id=200,
            botcash_tx_id="t" * 64,
            message_type="post",
            content_hash="h" * 64,
        )
        db_session.add(message)
        await db_session.commit()

        assert message.id is not None
        assert message.direction == "tg_to_bc"
        assert message.fee_sponsored is False  # default

    async def test_relayed_message_relationship(self, db_session):
        """Test relayed message -> identity relationship."""
        identity = LinkedIdentity(
            telegram_user_id=12345,
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        message = RelayedMessage(
            identity_id=identity.id,
            direction="bc_to_tg",
            botcash_tx_id="t" * 64,
            message_type="post",
            content_hash="h" * 64,
        )
        db_session.add(message)
        await db_session.commit()

        # Access through relationship
        await db_session.refresh(message)
        assert message.identity.telegram_user_id == 12345


class TestRateLimitEntry:
    """Tests for RateLimitEntry model."""

    async def test_create_rate_limit_entry(self, db_session):
        """Test creating a rate limit entry."""
        now = datetime.now(timezone.utc).replace(second=0, microsecond=0)

        entry = RateLimitEntry(
            telegram_user_id=12345,
            window_start=now,
            message_count=1,
        )
        db_session.add(entry)
        await db_session.commit()

        assert entry.id is not None
        assert entry.message_count == 1

    async def test_increment_rate_limit(self, db_session):
        """Test incrementing rate limit count."""
        now = datetime.now(timezone.utc).replace(second=0, microsecond=0)

        entry = RateLimitEntry(
            telegram_user_id=12345,
            window_start=now,
            message_count=1,
        )
        db_session.add(entry)
        await db_session.commit()

        entry.message_count += 1
        await db_session.commit()

        assert entry.message_count == 2


class TestSponsoredTransaction:
    """Tests for SponsoredTransaction model."""

    async def test_create_sponsored_transaction(self, db_session):
        """Test creating a sponsored transaction record."""
        tx = SponsoredTransaction(
            telegram_user_id=12345,
            tx_id="t" * 64,
            fee_zatoshis=1000,
        )
        db_session.add(tx)
        await db_session.commit()

        assert tx.id is not None
        assert tx.created_at is not None

    async def test_unique_tx_id(self, db_session):
        """Test that tx_id is unique."""
        tx1 = SponsoredTransaction(
            telegram_user_id=12345,
            tx_id="t" * 64,
            fee_zatoshis=1000,
        )
        db_session.add(tx1)
        await db_session.commit()

        tx2 = SponsoredTransaction(
            telegram_user_id=67890,
            tx_id="t" * 64,  # Same tx_id
            fee_zatoshis=2000,
        )
        db_session.add(tx2)

        with pytest.raises(Exception):  # IntegrityError
            await db_session.commit()
