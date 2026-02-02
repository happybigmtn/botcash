"""Tests for database models."""

from datetime import datetime, timedelta, timezone

import pytest

from botcash_twitter.models import (
    Base,
    CrossPostRecord,
    LinkedIdentity,
    LinkStatus,
    OAuthPendingState,
    OAuthToken,
    PrivacyMode,
    RateLimitEntry,
    SponsoredTransaction,
    init_db,
)


class TestLinkStatus:
    """Tests for LinkStatus enum."""

    def test_pending_value(self):
        assert LinkStatus.PENDING.value == "pending"

    def test_active_value(self):
        assert LinkStatus.ACTIVE.value == "active"

    def test_unlinked_value(self):
        assert LinkStatus.UNLINKED.value == "unlinked"

    def test_suspended_value(self):
        assert LinkStatus.SUSPENDED.value == "suspended"

    def test_expired_value(self):
        assert LinkStatus.EXPIRED.value == "expired"


class TestPrivacyMode:
    """Tests for PrivacyMode enum."""

    def test_full_mirror_value(self):
        assert PrivacyMode.FULL_MIRROR.value == "full_mirror"

    def test_selective_value(self):
        assert PrivacyMode.SELECTIVE.value == "selective"

    def test_disabled_value(self):
        assert PrivacyMode.DISABLED.value == "disabled"


class TestLinkedIdentity:
    """Tests for LinkedIdentity model."""

    async def test_create_identity(self, session):
        identity = LinkedIdentity(
            twitter_user_id="12345678",
            twitter_username="testuser",
            twitter_display_name="Test User",
            botcash_address="bs1testaddress",
            status=LinkStatus.ACTIVE,
            privacy_mode=PrivacyMode.SELECTIVE,
        )
        session.add(identity)
        await session.commit()

        assert identity.id is not None
        assert identity.twitter_user_id == "12345678"
        assert identity.created_at is not None

    async def test_identity_defaults(self, session):
        identity = LinkedIdentity(
            twitter_user_id="12345678",
            twitter_username="testuser",
            botcash_address="bs1testaddress",
        )
        session.add(identity)
        await session.commit()

        assert identity.status == LinkStatus.PENDING
        assert identity.privacy_mode == PrivacyMode.SELECTIVE
        assert identity.linked_at is None

    async def test_identity_linked_at(self, session):
        now = datetime.now(timezone.utc)
        identity = LinkedIdentity(
            twitter_user_id="12345678",
            twitter_username="testuser",
            botcash_address="bs1testaddress",
            status=LinkStatus.ACTIVE,
            linked_at=now,
        )
        session.add(identity)
        await session.commit()

        assert identity.linked_at is not None

    async def test_unique_twitter_user_id(self, session):
        identity1 = LinkedIdentity(
            twitter_user_id="12345678",
            twitter_username="testuser1",
            botcash_address="bs1address1",
        )
        session.add(identity1)
        await session.commit()

        identity2 = LinkedIdentity(
            twitter_user_id="12345678",
            twitter_username="testuser2",
            botcash_address="bs1address2",
        )
        session.add(identity2)

        with pytest.raises(Exception):  # IntegrityError
            await session.commit()

    async def test_unique_botcash_address(self, session):
        identity1 = LinkedIdentity(
            twitter_user_id="11111111",
            twitter_username="testuser1",
            botcash_address="bs1address",
        )
        session.add(identity1)
        await session.commit()

        identity2 = LinkedIdentity(
            twitter_user_id="22222222",
            twitter_username="testuser2",
            botcash_address="bs1address",
        )
        session.add(identity2)

        with pytest.raises(Exception):  # IntegrityError
            await session.commit()


class TestOAuthToken:
    """Tests for OAuthToken model."""

    async def test_create_token(self, session):
        token = OAuthToken(
            twitter_user_id="12345678",
            access_token="test_access_token",
            refresh_token="test_refresh_token",
            scope="tweet.read tweet.write",
        )
        session.add(token)
        await session.commit()

        assert token.id is not None
        assert token.token_type == "Bearer"

    async def test_token_expiry(self, session):
        expires = datetime.now(timezone.utc) + timedelta(hours=2)
        token = OAuthToken(
            twitter_user_id="12345678",
            access_token="test_access_token",
            scope="tweet.read",
            expires_at=expires,
        )
        session.add(token)
        await session.commit()

        assert token.expires_at is not None


class TestCrossPostRecord:
    """Tests for CrossPostRecord model."""

    async def test_create_record(self, session):
        record = CrossPostRecord(
            twitter_user_id="12345678",
            botcash_tx_id="tx_abc123",
            content_hash="hash123",
            tweet_content="Test tweet",
        )
        session.add(record)
        await session.commit()

        assert record.id is not None
        assert record.success is False
        assert record.retry_count == 0

    async def test_successful_record(self, session):
        record = CrossPostRecord(
            twitter_user_id="12345678",
            botcash_tx_id="tx_abc123",
            tweet_id="tweet_123",
            content_hash="hash123",
            tweet_content="Test tweet",
            success=True,
            posted_at=datetime.now(timezone.utc),
        )
        session.add(record)
        await session.commit()

        assert record.success is True
        assert record.tweet_id == "tweet_123"

    async def test_failed_record(self, session):
        record = CrossPostRecord(
            twitter_user_id="12345678",
            botcash_tx_id="tx_abc123",
            content_hash="hash123",
            success=False,
            error="Rate limit exceeded",
        )
        session.add(record)
        await session.commit()

        assert record.success is False
        assert record.error == "Rate limit exceeded"

    async def test_unique_botcash_tx_id(self, session):
        record1 = CrossPostRecord(
            twitter_user_id="12345678",
            botcash_tx_id="tx_abc123",
            content_hash="hash123",
        )
        session.add(record1)
        await session.commit()

        record2 = CrossPostRecord(
            twitter_user_id="12345678",
            botcash_tx_id="tx_abc123",
            content_hash="hash456",
        )
        session.add(record2)

        with pytest.raises(Exception):  # IntegrityError
            await session.commit()


class TestRateLimitEntry:
    """Tests for RateLimitEntry model."""

    async def test_create_entry(self, session):
        entry = RateLimitEntry(
            twitter_user_id="12345678",
            window_start=datetime.now(timezone.utc),
            request_count=1,
        )
        session.add(entry)
        await session.commit()

        assert entry.id is not None

    async def test_increment_count(self, session):
        entry = RateLimitEntry(
            twitter_user_id="12345678",
            window_start=datetime.now(timezone.utc),
            request_count=1,
        )
        session.add(entry)
        await session.commit()

        entry.request_count += 1
        await session.commit()

        assert entry.request_count == 2


class TestOAuthPendingState:
    """Tests for OAuthPendingState model."""

    async def test_create_state(self, session):
        state = OAuthPendingState(
            state="test_state_123",
            code_verifier="test_verifier",
            botcash_address="bs1testaddress",
            expires_at=datetime.now(timezone.utc) + timedelta(minutes=10),
        )
        session.add(state)
        await session.commit()

        assert state.id is not None
        assert state.state == "test_state_123"

    async def test_unique_state(self, session):
        state1 = OAuthPendingState(
            state="test_state_123",
            code_verifier="verifier1",
            botcash_address="bs1address1",
            expires_at=datetime.now(timezone.utc) + timedelta(minutes=10),
        )
        session.add(state1)
        await session.commit()

        state2 = OAuthPendingState(
            state="test_state_123",
            code_verifier="verifier2",
            botcash_address="bs1address2",
            expires_at=datetime.now(timezone.utc) + timedelta(minutes=10),
        )
        session.add(state2)

        with pytest.raises(Exception):  # IntegrityError
            await session.commit()


class TestSponsoredTransaction:
    """Tests for SponsoredTransaction model."""

    async def test_create_transaction(self, session):
        tx = SponsoredTransaction(
            botcash_address="bs1testaddress",
            tx_id="tx_123",
            fee_zatoshis=10000,
        )
        session.add(tx)
        await session.commit()

        assert tx.id is not None
        assert tx.fee_zatoshis == 10000


class TestInitDb:
    """Tests for init_db function."""

    async def test_init_db_creates_tables(self):
        session_maker = await init_db("sqlite+aiosqlite:///:memory:")
        async with session_maker() as session:
            # Try to create an identity - should work if tables exist
            identity = LinkedIdentity(
                twitter_user_id="12345678",
                twitter_username="testuser",
                botcash_address="bs1testaddress",
            )
            session.add(identity)
            await session.commit()
            assert identity.id is not None
