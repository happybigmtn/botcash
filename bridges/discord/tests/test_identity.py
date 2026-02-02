"""Tests for Discord bridge identity service."""

from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock

import pytest
from sqlalchemy import select

from botcash_discord.botcash_client import PostResult
from botcash_discord.identity import CHALLENGE_EXPIRY_MINUTES, IdentityLinkError, IdentityService
from botcash_discord.models import LinkedIdentity, LinkStatus, PrivacyMode


class TestIdentityServiceInitiateLink:
    """Tests for initiating identity links."""

    async def test_initiate_link_creates_pending_identity(
        self, db_session, mock_botcash_client
    ):
        """Test that initiating link creates a pending identity."""
        service = IdentityService(mock_botcash_client)
        address = "bs1" + "a" * 59

        async with db_session() as session:
            challenge, msg = await service.initiate_link(
                session,
                discord_user_id=123456789012345678,
                discord_username="testuser",
                discord_discriminator="1234",
                botcash_address=address,
            )

            # Verify challenge returned
            assert len(challenge) == 64
            assert "123456789012345678" in msg

            # Verify identity created
            result = await session.execute(
                select(LinkedIdentity).where(
                    LinkedIdentity.discord_user_id == 123456789012345678
                )
            )
            identity = result.scalar_one()

            assert identity.status == LinkStatus.PENDING
            assert identity.botcash_address == address
            assert identity.challenge is not None
            assert identity.challenge_expires_at is not None

    async def test_initiate_link_validates_address(
        self, db_session, mock_botcash_client
    ):
        """Test that invalid addresses are rejected."""
        mock_botcash_client.validate_address = AsyncMock(return_value=False)
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            with pytest.raises(IdentityLinkError) as exc_info:
                await service.initiate_link(
                    session,
                    discord_user_id=123456789012345678,
                    discord_username="testuser",
                    discord_discriminator=None,
                    botcash_address="invalid_address",
                )

            assert "Invalid Botcash address" in str(exc_info.value)

    async def test_initiate_link_rejects_already_linked_address(
        self, db_session, mock_botcash_client
    ):
        """Test that addresses already linked to another user are rejected."""
        service = IdentityService(mock_botcash_client)
        address = "bs1" + "a" * 59

        async with db_session() as session:
            # Create existing active link for different user
            existing = LinkedIdentity(
                discord_user_id=999888777666555444,
                botcash_address=address,
                status=LinkStatus.ACTIVE,
            )
            session.add(existing)
            await session.commit()

            with pytest.raises(IdentityLinkError) as exc_info:
                await service.initiate_link(
                    session,
                    discord_user_id=123456789012345678,
                    discord_username="testuser",
                    discord_discriminator=None,
                    botcash_address=address,
                )

            assert "already linked" in str(exc_info.value)

    async def test_initiate_link_rejects_already_active_user(
        self, db_session, mock_botcash_client
    ):
        """Test that users with active links must unlink first."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            # Create existing active link for same user
            existing = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "b" * 59,
                status=LinkStatus.ACTIVE,
            )
            session.add(existing)
            await session.commit()

            with pytest.raises(IdentityLinkError) as exc_info:
                await service.initiate_link(
                    session,
                    discord_user_id=123456789012345678,
                    discord_username="testuser",
                    discord_discriminator=None,
                    botcash_address="bs1" + "a" * 59,
                )

            assert "unlink first" in str(exc_info.value).lower()

    async def test_initiate_link_updates_pending_record(
        self, db_session, mock_botcash_client
    ):
        """Test that pending records are updated on re-initiation."""
        service = IdentityService(mock_botcash_client)
        old_address = "bs1" + "a" * 59
        new_address = "bs1" + "b" * 59

        async with db_session() as session:
            # Create pending link
            pending = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address=old_address,
                status=LinkStatus.PENDING,
                challenge="old_challenge",
            )
            session.add(pending)
            await session.commit()
            pending_id = pending.id

            # Re-initiate with new address
            await service.initiate_link(
                session,
                discord_user_id=123456789012345678,
                discord_username="testuser",
                discord_discriminator=None,
                botcash_address=new_address,
            )

            # Verify same record updated
            result = await session.execute(
                select(LinkedIdentity).where(LinkedIdentity.id == pending_id)
            )
            updated = result.scalar_one()

            assert updated.botcash_address == new_address
            assert updated.challenge != "old_challenge"


class TestIdentityServiceCompleteLink:
    """Tests for completing identity links."""

    async def test_complete_link_activates_identity(
        self, db_session, mock_botcash_client
    ):
        """Test that completing link activates the identity."""
        service = IdentityService(mock_botcash_client)
        mock_botcash_client.create_bridge_link = AsyncMock(
            return_value=PostResult(tx_id="tx123" * 10, success=True)
        )

        async with db_session() as session:
            # Create pending identity
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.PENDING,
                challenge="a" * 64,
                challenge_expires_at=datetime.now(timezone.utc) + timedelta(minutes=5),
            )
            session.add(identity)
            await session.commit()

            # Complete link
            result = await service.complete_link(
                session,
                discord_user_id=123456789012345678,
                signature="b" * 128,
            )

            assert result.status == LinkStatus.ACTIVE
            assert result.link_tx_id is not None
            assert result.linked_at is not None
            assert result.challenge is None

    async def test_complete_link_rejects_no_pending(
        self, db_session, mock_botcash_client
    ):
        """Test that completing without pending link fails."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            with pytest.raises(IdentityLinkError) as exc_info:
                await service.complete_link(
                    session,
                    discord_user_id=123456789012345678,
                    signature="b" * 128,
                )

            assert "No pending link" in str(exc_info.value)

    async def test_complete_link_rejects_expired_challenge(
        self, db_session, mock_botcash_client
    ):
        """Test that expired challenges are rejected."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            # Create pending identity with expired challenge
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.PENDING,
                challenge="a" * 64,
                challenge_expires_at=datetime.now(timezone.utc) - timedelta(minutes=1),
            )
            session.add(identity)
            await session.commit()

            with pytest.raises(IdentityLinkError) as exc_info:
                await service.complete_link(
                    session,
                    discord_user_id=123456789012345678,
                    signature="b" * 128,
                )

            assert "expired" in str(exc_info.value).lower()

    async def test_complete_link_rejects_short_signature(
        self, db_session, mock_botcash_client
    ):
        """Test that short signatures are rejected."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.PENDING,
                challenge="a" * 64,
                challenge_expires_at=datetime.now(timezone.utc) + timedelta(minutes=5),
            )
            session.add(identity)
            await session.commit()

            with pytest.raises(IdentityLinkError) as exc_info:
                await service.complete_link(
                    session,
                    discord_user_id=123456789012345678,
                    signature="short",
                )

            assert "Invalid signature" in str(exc_info.value)


class TestIdentityServiceUnlink:
    """Tests for unlinking identities."""

    async def test_unlink_active_identity(self, db_session, mock_botcash_client):
        """Test unlinking an active identity."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.ACTIVE,
            )
            session.add(identity)
            await session.commit()

            result = await service.unlink(session, 123456789012345678)

            assert result is True
            assert identity.status == LinkStatus.UNLINKED
            assert identity.unlinked_at is not None

    async def test_unlink_nonexistent_returns_false(
        self, db_session, mock_botcash_client
    ):
        """Test that unlinking nonexistent user returns False."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            result = await service.unlink(session, 123456789012345678)
            assert result is False


class TestIdentityServicePrivacyMode:
    """Tests for privacy mode management."""

    async def test_set_privacy_mode(self, db_session, mock_botcash_client):
        """Test setting privacy mode."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.ACTIVE,
                privacy_mode=PrivacyMode.SELECTIVE,
            )
            session.add(identity)
            await session.commit()

            result = await service.set_privacy_mode(
                session, 123456789012345678, PrivacyMode.FULL_MIRROR
            )

            assert result is True
            assert identity.privacy_mode == PrivacyMode.FULL_MIRROR

    async def test_set_privacy_mode_no_identity_returns_false(
        self, db_session, mock_botcash_client
    ):
        """Test setting privacy mode for nonexistent user."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            result = await service.set_privacy_mode(
                session, 123456789012345678, PrivacyMode.PRIVATE
            )
            assert result is False


class TestIdentityServiceQueries:
    """Tests for identity query methods."""

    async def test_get_linked_identity(self, db_session, mock_botcash_client):
        """Test getting linked identity by Discord user ID."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.ACTIVE,
            )
            session.add(identity)
            await session.commit()

            result = await service.get_linked_identity(session, 123456789012345678)

            assert result is not None
            assert result.discord_user_id == 123456789012345678

    async def test_get_linked_identity_ignores_pending(
        self, db_session, mock_botcash_client
    ):
        """Test that pending identities are not returned."""
        service = IdentityService(mock_botcash_client)

        async with db_session() as session:
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address="bs1" + "a" * 59,
                status=LinkStatus.PENDING,
            )
            session.add(identity)
            await session.commit()

            result = await service.get_linked_identity(session, 123456789012345678)

            assert result is None

    async def test_get_identity_by_address(self, db_session, mock_botcash_client):
        """Test getting identity by Botcash address."""
        service = IdentityService(mock_botcash_client)
        address = "bs1" + "a" * 59

        async with db_session() as session:
            identity = LinkedIdentity(
                discord_user_id=123456789012345678,
                botcash_address=address,
                status=LinkStatus.ACTIVE,
            )
            session.add(identity)
            await session.commit()

            result = await service.get_identity_by_address(session, address)

            assert result is not None
            assert result.botcash_address == address
