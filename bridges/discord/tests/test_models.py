"""Tests for Discord bridge database models."""

from datetime import datetime, timezone

import pytest
from sqlalchemy import select

from botcash_discord.models import (
    Base,
    BridgedChannel,
    LinkedIdentity,
    LinkStatus,
    PrivacyMode,
    RateLimitEntry,
    RelayedMessage,
    SponsoredTransaction,
)


class TestLinkedIdentity:
    """Tests for LinkedIdentity model."""

    async def test_create_linked_identity(self, db_session):
        """Test creating a linked identity."""
        async with db_session() as session:
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                discord_username="testuser",
                discord_discriminator="1234",
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.PENDING,
            )
            session.add(identity)
            await session.commit()

            # Query back
            result = await session.execute(
                select(LinkedIdentity).where(
                    LinkedIdentity.discord_user_id == 123456789012345678
                )
            )
            fetched = result.scalar_one()

            assert fetched.discord_user_id == 123456789012345678
            assert fetched.discord_username == "testuser"
            assert fetched.status == LinkStatus.PENDING
            assert fetched.privacy_mode == PrivacyMode.SELECTIVE  # Default

    async def test_linked_identity_unique_discord_user(self, db_session):
        """Test that discord_user_id is unique."""
        async with db_session() as session:
            identity1 = LinkedIdentity(
                discord_user_id=123456789012345678,
                discord_username="user1",
                botcash_address="bs1" + "a" * 59,
            )
            session.add(identity1)
            await session.commit()

            identity2 = LinkedIdentity(
                discord_user_id=123456789012345678,  # Same ID
                discord_username="user2",
                botcash_address="bs1" + "b" * 59,
            )
            session.add(identity2)

            with pytest.raises(Exception):  # IntegrityError
                await session.commit()

    async def test_linked_identity_timestamps(self, db_session):
        """Test that timestamps are set correctly."""
        async with db_session() as session:
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
            )
            session.add(identity)
            await session.commit()

            assert identity.created_at is not None
            assert identity.updated_at is not None
            assert identity.linked_at is None  # Not linked yet

    async def test_link_status_enum(self, db_session):
        """Test LinkStatus enum values."""
        assert LinkStatus.PENDING.value == "pending"
        assert LinkStatus.ACTIVE.value == "active"
        assert LinkStatus.UNLINKED.value == "unlinked"
        assert LinkStatus.SUSPENDED.value == "suspended"

    async def test_privacy_mode_enum(self, db_session):
        """Test PrivacyMode enum values."""
        assert PrivacyMode.FULL_MIRROR.value == "full_mirror"
        assert PrivacyMode.SELECTIVE.value == "selective"
        assert PrivacyMode.READ_ONLY.value == "read_only"
        assert PrivacyMode.PRIVATE.value == "private"


class TestRelayedMessage:
    """Tests for RelayedMessage model."""

    async def test_create_relayed_message(self, db_session):
        """Test creating a relayed message."""
        async with db_session() as session:
            # First create an identity
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.ACTIVE,
            )
            session.add(identity)
            await session.commit()

            # Create relayed message
            message = RelayedMessage(
                identity_id=identity.id,
                direction="discord_to_bc",
                discord_message_id=987654321098765432,
                discord_channel_id=111222333444555666,
                discord_guild_id=777888999000111222,
                botcash_tx_id="abc123" * 10,
                message_type="post",
                content_hash="hash123",
            )
            session.add(message)
            await session.commit()

            assert message.id is not None
            assert message.direction == "discord_to_bc"
            assert message.fee_sponsored is False  # Default

    async def test_relayed_message_relationship(self, db_session):
        """Test relationship between identity and relayed messages."""
        async with db_session() as session:
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.ACTIVE,
            )
            session.add(identity)
            await session.commit()

            message = RelayedMessage(
                identity_id=identity.id,
                direction="discord_to_bc",
                message_type="post",
                content_hash="hash123",
            )
            session.add(message)
            await session.commit()

            # Access relationship
            await session.refresh(identity, ["relayed_messages"])
            assert len(identity.relayed_messages) == 1
            assert identity.relayed_messages[0].message_type == "post"


class TestRateLimitEntry:
    """Tests for RateLimitEntry model."""

    async def test_create_rate_limit_entry(self, db_session):
        """Test creating a rate limit entry."""
        async with db_session() as session:
            entry = RateLimitEntry(
                discord_user_id=123456789012345678,
                window_start=datetime.now(timezone.utc).replace(second=0, microsecond=0),
                message_count=1,
            )
            session.add(entry)
            await session.commit()

            assert entry.id is not None
            assert entry.message_count == 1


class TestSponsoredTransaction:
    """Tests for SponsoredTransaction model."""

    async def test_create_sponsored_transaction(self, db_session):
        """Test creating a sponsored transaction."""
        async with db_session() as session:
            tx = SponsoredTransaction(
                discord_user_id=123456789012345678,
                tx_id="abc123" * 10,
                fee_zatoshis=10000,
            )
            session.add(tx)
            await session.commit()

            assert tx.id is not None
            assert tx.fee_zatoshis == 10000

    async def test_sponsored_transaction_unique_tx_id(self, db_session):
        """Test that tx_id is unique."""
        async with db_session() as session:
            tx1 = SponsoredTransaction(
                discord_user_id=123456789012345678,
                tx_id="abc123" * 10,
                fee_zatoshis=10000,
            )
            session.add(tx1)
            await session.commit()

            tx2 = SponsoredTransaction(
                discord_user_id=999888777666555444,
                tx_id="abc123" * 10,  # Same tx_id
                fee_zatoshis=20000,
            )
            session.add(tx2)

            with pytest.raises(Exception):  # IntegrityError
                await session.commit()


class TestBridgedChannel:
    """Tests for BridgedChannel model."""

    async def test_create_bridged_channel(self, db_session):
        """Test creating a bridged channel."""
        async with db_session() as session:
            channel = BridgedChannel(
                discord_guild_id=123456789012345678,
                discord_channel_id=987654321098765432,
                channel_name="botcash-bridge",
                relay_posts=True,
                relay_replies=True,
            )
            session.add(channel)
            await session.commit()

            assert channel.id is not None
            assert channel.relay_posts is True
            assert channel.auto_post_format is True  # Default

    async def test_bridged_channel_unique_discord_channel(self, db_session):
        """Test that discord_channel_id is unique."""
        async with db_session() as session:
            channel1 = BridgedChannel(
                discord_guild_id=123456789012345678,
                discord_channel_id=987654321098765432,
                channel_name="channel1",
            )
            session.add(channel1)
            await session.commit()

            channel2 = BridgedChannel(
                discord_guild_id=123456789012345678,
                discord_channel_id=987654321098765432,  # Same channel ID
                channel_name="channel2",
            )
            session.add(channel2)

            with pytest.raises(Exception):  # IntegrityError
                await session.commit()
