"""Tests for identity linking service."""

import pytest
from datetime import datetime, timedelta, timezone

from botcash_telegram.identity import IdentityService, IdentityLinkError
from botcash_telegram.models import LinkedIdentity, LinkStatus, PrivacyMode


class TestIdentityService:
    """Tests for IdentityService."""

    @pytest.fixture
    def identity_service(self, mock_botcash_client):
        """Create identity service with mock client."""
        return IdentityService(mock_botcash_client)

    async def test_initiate_link_creates_pending_identity(
        self, db_session, identity_service
    ):
        """Test initiating a link creates a pending identity record."""
        challenge, msg = await identity_service.initiate_link(
            db_session,
            telegram_user_id=12345,
            telegram_username="testuser",
            botcash_address="bs1" + "a" * 59,
        )

        # Should return challenge
        assert len(challenge) == 64
        assert "12345" in msg
        assert challenge in msg

        # Should create pending identity
        from sqlalchemy import select
        result = await db_session.execute(
            select(LinkedIdentity).where(LinkedIdentity.telegram_user_id == 12345)
        )
        identity = result.scalar_one()
        assert identity.status == LinkStatus.PENDING
        assert identity.challenge == challenge
        assert identity.botcash_address == "bs1" + "a" * 59

    async def test_initiate_link_rejects_invalid_address(
        self, db_session, identity_service, mock_botcash_client
    ):
        """Test initiating a link with invalid address fails."""
        mock_botcash_client.validate_address.return_value = False

        with pytest.raises(IdentityLinkError, match="Invalid Botcash address"):
            await identity_service.initiate_link(
                db_session,
                telegram_user_id=12345,
                telegram_username="testuser",
                botcash_address="invalid_address",
            )

    async def test_initiate_link_rejects_already_linked_address(
        self, db_session, identity_service
    ):
        """Test initiating a link with already-linked address fails."""
        # Create existing active identity
        existing = LinkedIdentity(
            telegram_user_id=11111,
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(existing)
        await db_session.commit()

        # Try to link same address to different user
        with pytest.raises(IdentityLinkError, match="already linked"):
            await identity_service.initiate_link(
                db_session,
                telegram_user_id=22222,
                telegram_username="testuser2",
                botcash_address="bs1" + "a" * 59,
            )

    async def test_complete_link_activates_identity(
        self, db_session, identity_service
    ):
        """Test completing a link activates the identity."""
        # Initiate link first
        await identity_service.initiate_link(
            db_session,
            telegram_user_id=12345,
            telegram_username="testuser",
            botcash_address="bs1" + "a" * 59,
        )

        # Complete with signature
        identity = await identity_service.complete_link(
            db_session,
            telegram_user_id=12345,
            signature="e" * 64,
        )

        assert identity.status == LinkStatus.ACTIVE
        assert identity.link_tx_id == "b" * 64
        assert identity.linked_at is not None
        assert identity.challenge is None

    async def test_complete_link_fails_without_pending(
        self, db_session, identity_service
    ):
        """Test completing a link without pending record fails."""
        with pytest.raises(IdentityLinkError, match="No pending link"):
            await identity_service.complete_link(
                db_session,
                telegram_user_id=99999,
                signature="e" * 64,
            )

    async def test_complete_link_fails_on_expired_challenge(
        self, db_session, identity_service
    ):
        """Test completing a link with expired challenge fails."""
        # Create expired pending identity
        expired = LinkedIdentity(
            telegram_user_id=12345,
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.PENDING,
            challenge="f" * 64,
            challenge_expires_at=datetime.now(timezone.utc) - timedelta(minutes=1),
        )
        db_session.add(expired)
        await db_session.commit()

        with pytest.raises(IdentityLinkError, match="expired"):
            await identity_service.complete_link(
                db_session,
                telegram_user_id=12345,
                signature="e" * 64,
            )

    async def test_unlink_changes_status(self, db_session, identity_service):
        """Test unlinking changes identity status."""
        # Create active identity
        identity = LinkedIdentity(
            telegram_user_id=12345,
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.ACTIVE,
            linked_at=datetime.now(timezone.utc),
        )
        db_session.add(identity)
        await db_session.commit()

        # Unlink
        result = await identity_service.unlink(db_session, 12345)
        assert result is True

        # Check status
        await db_session.refresh(identity)
        assert identity.status == LinkStatus.UNLINKED
        assert identity.unlinked_at is not None

    async def test_unlink_returns_false_if_not_found(
        self, db_session, identity_service
    ):
        """Test unlinking returns False if no identity found."""
        result = await identity_service.unlink(db_session, 99999)
        assert result is False

    async def test_get_linked_identity_returns_active_only(
        self, db_session, identity_service
    ):
        """Test get_linked_identity only returns active identities."""
        # Create unlinked identity
        unlinked = LinkedIdentity(
            telegram_user_id=12345,
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.UNLINKED,
        )
        db_session.add(unlinked)
        await db_session.commit()

        # Should not find unlinked
        result = await identity_service.get_linked_identity(db_session, 12345)
        assert result is None

    async def test_set_privacy_mode(self, db_session, identity_service):
        """Test setting privacy mode."""
        # Create active identity
        identity = LinkedIdentity(
            telegram_user_id=12345,
            botcash_address="bs1" + "a" * 59,
            status=LinkStatus.ACTIVE,
            privacy_mode=PrivacyMode.SELECTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        # Change privacy mode
        result = await identity_service.set_privacy_mode(
            db_session, 12345, PrivacyMode.FULL_MIRROR
        )
        assert result is True

        await db_session.refresh(identity)
        assert identity.privacy_mode == PrivacyMode.FULL_MIRROR
