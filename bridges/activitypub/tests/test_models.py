"""Tests for ActivityPub bridge database models."""

import pytest
from datetime import datetime, timezone
from sqlalchemy import select

from botcash_activitypub.models import (
    Follower,
    Following,
    LinkedIdentity,
    LinkStatus,
    PrivacyMode,
    RateLimitEntry,
    RelayedMessage,
    RemoteActor,
    SponsoredTransaction,
    StoredActivity,
    init_db,
)


class TestLinkStatus:
    """Tests for LinkStatus enum."""

    def test_link_statuses(self):
        """Test all link statuses exist."""
        assert LinkStatus.PENDING.value == "pending"
        assert LinkStatus.ACTIVE.value == "active"
        assert LinkStatus.UNLINKED.value == "unlinked"
        assert LinkStatus.SUSPENDED.value == "suspended"


class TestPrivacyMode:
    """Tests for PrivacyMode enum in models."""

    def test_privacy_modes(self):
        """Test all privacy modes exist."""
        assert PrivacyMode.FULL_MIRROR.value == "full_mirror"
        assert PrivacyMode.SELECTIVE.value == "selective"


class TestLinkedIdentity:
    """Tests for LinkedIdentity model."""

    @pytest.mark.asyncio
    async def test_create_linked_identity(self, session):
        """Test creating a linked identity."""
        identity = LinkedIdentity(
            actor_id="https://botcash.social/users/bs1test",
            actor_local_part="bs1test",
            botcash_address="bs1testaddress123456",
            public_key_pem="-----BEGIN PUBLIC KEY-----\ntest\n-----END PUBLIC KEY-----",
            private_key_pem="-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----",
            status=LinkStatus.ACTIVE,
            privacy_mode=PrivacyMode.SELECTIVE,
        )
        session.add(identity)
        await session.commit()

        result = await session.execute(
            select(LinkedIdentity).where(LinkedIdentity.actor_id == identity.actor_id)
        )
        saved = result.scalar_one()

        assert saved.botcash_address == "bs1testaddress123456"
        assert saved.status == LinkStatus.ACTIVE
        assert saved.privacy_mode == PrivacyMode.SELECTIVE

    @pytest.mark.asyncio
    async def test_linked_identity_timestamps(self, session):
        """Test that timestamps are set correctly."""
        identity = LinkedIdentity(
            actor_id="https://botcash.social/users/bs1time",
            actor_local_part="bs1time",
            botcash_address="bs1timetest1234567890",
        )
        session.add(identity)
        await session.commit()

        assert identity.created_at is not None
        assert identity.updated_at is not None


class TestRemoteActor:
    """Tests for RemoteActor model."""

    @pytest.mark.asyncio
    async def test_create_remote_actor(self, session):
        """Test creating a remote actor cache entry."""
        actor = RemoteActor(
            actor_id="https://mastodon.social/users/alice",
            instance_domain="mastodon.social",
            handle="@alice@mastodon.social",
            preferred_username="alice",
            display_name="Alice",
            inbox_url="https://mastodon.social/users/alice/inbox",
            outbox_url="https://mastodon.social/users/alice/outbox",
            shared_inbox_url="https://mastodon.social/inbox",
            public_key_id="https://mastodon.social/users/alice#main-key",
            public_key_pem="-----BEGIN PUBLIC KEY-----\ntest\n-----END PUBLIC KEY-----",
        )
        session.add(actor)
        await session.commit()

        result = await session.execute(
            select(RemoteActor).where(RemoteActor.actor_id == actor.actor_id)
        )
        saved = result.scalar_one()

        assert saved.preferred_username == "alice"
        assert saved.display_name == "Alice"
        assert saved.shared_inbox_url == "https://mastodon.social/inbox"


class TestFollower:
    """Tests for Follower model."""

    @pytest.mark.asyncio
    async def test_create_follower(self, session):
        """Test creating a follower relationship."""
        # First create identity and remote actor
        identity = LinkedIdentity(
            actor_id="https://botcash.social/users/bs1local",
            actor_local_part="bs1local",
            botcash_address="bs1localtest12345678",
        )
        session.add(identity)

        remote = RemoteActor(
            actor_id="https://mastodon.social/users/alice",
            instance_domain="mastodon.social",
            handle="@alice@mastodon.social",
            preferred_username="alice",
            inbox_url="https://mastodon.social/users/alice/inbox",
            public_key_id="https://mastodon.social/users/alice#main-key",
            public_key_pem="test",
        )
        session.add(remote)
        await session.commit()

        follower = Follower(
            identity_id=identity.id,
            remote_actor_id=remote.id,
            follow_activity_id="https://mastodon.social/activities/123",
        )
        session.add(follower)
        await session.commit()

        result = await session.execute(
            select(Follower).where(Follower.identity_id == identity.id)
        )
        saved = result.scalar_one()

        assert saved.remote_actor_id == remote.id


class TestFollowing:
    """Tests for Following model."""

    @pytest.mark.asyncio
    async def test_create_following(self, session):
        """Test creating a following relationship."""
        identity = LinkedIdentity(
            actor_id="https://botcash.social/users/bs1local2",
            actor_local_part="bs1local2",
            botcash_address="bs1localtest23456789",
        )
        session.add(identity)

        remote = RemoteActor(
            actor_id="https://mastodon.social/users/bob",
            instance_domain="mastodon.social",
            handle="@bob@mastodon.social",
            preferred_username="bob",
            inbox_url="https://mastodon.social/users/bob/inbox",
            public_key_id="https://mastodon.social/users/bob#main-key",
            public_key_pem="test",
        )
        session.add(remote)
        await session.commit()

        following = Following(
            identity_id=identity.id,
            remote_actor_id=remote.id,
            status="accepted",
        )
        session.add(following)
        await session.commit()

        result = await session.execute(
            select(Following).where(Following.identity_id == identity.id)
        )
        saved = result.scalar_one()

        assert saved.status == "accepted"


class TestRelayedMessage:
    """Tests for RelayedMessage model."""

    @pytest.mark.asyncio
    async def test_create_relayed_message(self, session):
        """Test creating a relayed message record."""
        identity = LinkedIdentity(
            actor_id="https://botcash.social/users/bs1relay",
            actor_local_part="bs1relay",
            botcash_address="bs1relaytest12345678",
        )
        session.add(identity)
        await session.commit()

        message = RelayedMessage(
            identity_id=identity.id,
            direction="bc_to_ap",
            botcash_tx_id="abc123def456",
            ap_activity_id="https://botcash.social/users/bs1test/statuses/789",
            message_type="post",
            content_hash="sha256hash",
        )
        session.add(message)
        await session.commit()

        result = await session.execute(
            select(RelayedMessage).where(RelayedMessage.botcash_tx_id == message.botcash_tx_id)
        )
        saved = result.scalar_one()

        assert saved.direction == "bc_to_ap"
        assert saved.ap_activity_id == "https://botcash.social/users/bs1test/statuses/789"


class TestStoredActivity:
    """Tests for StoredActivity model."""

    @pytest.mark.asyncio
    async def test_create_stored_activity(self, session):
        """Test creating a stored activity."""
        activity = StoredActivity(
            activity_id="https://botcash.social/activities/abc",
            activity_type="Create",
            actor_id="https://botcash.social/users/bs1test",
            activity_json='{"type": "Create"}',
            object_id="https://botcash.social/users/bs1test/statuses/123",
        )
        session.add(activity)
        await session.commit()

        result = await session.execute(
            select(StoredActivity).where(StoredActivity.activity_id == activity.activity_id)
        )
        saved = result.scalar_one()

        assert saved.activity_type == "Create"


class TestRateLimitEntry:
    """Tests for RateLimitEntry model."""

    @pytest.mark.asyncio
    async def test_create_rate_limit_entry(self, session):
        """Test creating a rate limit entry."""
        entry = RateLimitEntry(
            instance_domain="mastodon.social",
            window_start=datetime.now(timezone.utc),
            request_count=5,
        )
        session.add(entry)
        await session.commit()

        result = await session.execute(
            select(RateLimitEntry).where(RateLimitEntry.instance_domain == entry.instance_domain)
        )
        saved = result.scalar_one()

        assert saved.request_count == 5


class TestSponsoredTransaction:
    """Tests for SponsoredTransaction model."""

    @pytest.mark.asyncio
    async def test_create_sponsored_transaction(self, session):
        """Test creating a sponsored transaction record."""
        tx = SponsoredTransaction(
            actor_id="https://botcash.social/users/bs1test",
            tx_id="txid123",
            fee_zatoshis=1000,
        )
        session.add(tx)
        await session.commit()

        result = await session.execute(
            select(SponsoredTransaction).where(SponsoredTransaction.tx_id == tx.tx_id)
        )
        saved = result.scalar_one()

        assert saved.fee_zatoshis == 1000


class TestInitDb:
    """Tests for database initialization."""

    @pytest.mark.asyncio
    async def test_init_db(self, config):
        """Test database initialization creates tables."""
        session_maker = await init_db(config.database.url)
        assert session_maker is not None
