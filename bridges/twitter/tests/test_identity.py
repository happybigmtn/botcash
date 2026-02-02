"""Tests for identity linking service."""

from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock

import pytest

from botcash_twitter.config import PrivacyMode
from botcash_twitter.identity import IdentityLinkError, IdentityService, OAuthState
from botcash_twitter.models import LinkedIdentity, LinkStatus, OAuthPendingState, OAuthToken


class TestOAuthState:
    """Tests for OAuthState dataclass."""

    def test_create_state(self):
        state = OAuthState(
            state="test_state",
            code_verifier="test_verifier",
            authorization_url="https://twitter.com/oauth",
        )
        assert state.state == "test_state"
        assert state.code_verifier == "test_verifier"
        assert state.authorization_url == "https://twitter.com/oauth"


class TestIdentityServiceInit:
    """Tests for IdentityService initialization."""

    def test_init(self, mock_botcash_client, mock_twitter_client):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )
        assert service.botcash is mock_botcash_client
        assert service.twitter is mock_twitter_client
        assert service.default_privacy_mode == PrivacyMode.SELECTIVE

    def test_init_custom_privacy_mode(self, mock_botcash_client, mock_twitter_client):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            default_privacy_mode=PrivacyMode.FULL_MIRROR,
        )
        assert service.default_privacy_mode == PrivacyMode.FULL_MIRROR


class TestIdentityServiceInitiateLink:
    """Tests for initiate_link method."""

    async def test_initiate_link_success(
        self, session, mock_botcash_client, mock_twitter_client, sample_botcash_address
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        oauth_state = await service.initiate_link(session, sample_botcash_address)

        assert isinstance(oauth_state, OAuthState)
        assert oauth_state.state
        assert oauth_state.code_verifier
        assert "twitter.com" in oauth_state.authorization_url

    async def test_initiate_link_invalid_address(
        self, session, mock_botcash_client, mock_twitter_client
    ):
        mock_botcash_client.validate_address = AsyncMock(return_value=False)
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        with pytest.raises(IdentityLinkError) as exc_info:
            await service.initiate_link(session, "invalid_address")

        assert "Invalid Botcash address" in str(exc_info.value)

    async def test_initiate_link_already_linked(
        self, session, mock_botcash_client, mock_twitter_client, sample_linked_identity
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        with pytest.raises(IdentityLinkError) as exc_info:
            await service.initiate_link(session, sample_linked_identity.botcash_address)

        assert "already linked" in str(exc_info.value)

    async def test_initiate_link_creates_pending_state(
        self, session, mock_botcash_client, mock_twitter_client, sample_botcash_address
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        oauth_state = await service.initiate_link(session, sample_botcash_address)

        # Check pending state was created
        from sqlalchemy import select
        result = await session.execute(
            select(OAuthPendingState).where(OAuthPendingState.state == oauth_state.state)
        )
        pending = result.scalar_one_or_none()

        assert pending is not None
        assert pending.botcash_address == sample_botcash_address
        assert pending.code_verifier == oauth_state.code_verifier


class TestIdentityServiceCompleteLink:
    """Tests for complete_link method."""

    async def test_complete_link_success(
        self, session, mock_botcash_client, mock_twitter_client, sample_botcash_address
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        # First initiate
        oauth_state = await service.initiate_link(session, sample_botcash_address)

        # Then complete
        identity = await service.complete_link(
            session,
            state=oauth_state.state,
            code="test_auth_code",
        )

        assert identity.twitter_user_id == "12345678"
        assert identity.twitter_username == "testuser"
        assert identity.botcash_address == sample_botcash_address
        assert identity.status == LinkStatus.ACTIVE

    async def test_complete_link_invalid_state(
        self, session, mock_botcash_client, mock_twitter_client
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        with pytest.raises(IdentityLinkError) as exc_info:
            await service.complete_link(session, state="invalid_state", code="code")

        assert "Invalid or expired" in str(exc_info.value)

    async def test_complete_link_expired_state(
        self, session, mock_botcash_client, mock_twitter_client, sample_botcash_address
    ):
        # Create expired pending state
        pending = OAuthPendingState(
            state="expired_state",
            code_verifier="test_verifier",
            botcash_address=sample_botcash_address,
            expires_at=datetime.now(timezone.utc) - timedelta(minutes=1),
        )
        session.add(pending)
        await session.commit()

        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        with pytest.raises(IdentityLinkError) as exc_info:
            await service.complete_link(session, state="expired_state", code="code")

        assert "expired" in str(exc_info.value)

    async def test_complete_link_creates_token(
        self, session, mock_botcash_client, mock_twitter_client, sample_botcash_address
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        oauth_state = await service.initiate_link(session, sample_botcash_address)
        await service.complete_link(session, state=oauth_state.state, code="code")

        # Check token was created
        from sqlalchemy import select
        result = await session.execute(
            select(OAuthToken).where(OAuthToken.twitter_user_id == "12345678")
        )
        token = result.scalar_one_or_none()

        assert token is not None
        assert token.access_token == "test_access_token"
        assert token.refresh_token == "test_refresh_token"


class TestIdentityServiceUnlink:
    """Tests for unlink method."""

    async def test_unlink_success(
        self, session, mock_botcash_client, mock_twitter_client,
        sample_linked_identity, sample_oauth_token
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        result = await service.unlink(session, sample_linked_identity.botcash_address)

        assert result is True

        # Refresh identity
        await session.refresh(sample_linked_identity)
        assert sample_linked_identity.status == LinkStatus.UNLINKED
        assert sample_linked_identity.unlinked_at is not None

    async def test_unlink_not_found(
        self, session, mock_botcash_client, mock_twitter_client
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        result = await service.unlink(session, "nonexistent_address")

        assert result is False

    async def test_unlink_revokes_token(
        self, session, mock_botcash_client, mock_twitter_client,
        sample_linked_identity, sample_oauth_token
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        await service.unlink(session, sample_linked_identity.botcash_address)

        mock_twitter_client.revoke_token.assert_called_once_with("test_access_token")


class TestIdentityServiceQueries:
    """Tests for identity query methods."""

    async def test_get_identity_by_address(
        self, session, mock_botcash_client, mock_twitter_client, sample_linked_identity
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        identity = await service.get_identity_by_address(
            session, sample_linked_identity.botcash_address
        )

        assert identity is not None
        assert identity.id == sample_linked_identity.id

    async def test_get_identity_by_address_not_found(
        self, session, mock_botcash_client, mock_twitter_client
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        identity = await service.get_identity_by_address(session, "nonexistent")

        assert identity is None

    async def test_get_identity_by_twitter_id(
        self, session, mock_botcash_client, mock_twitter_client, sample_linked_identity
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        identity = await service.get_identity_by_twitter_id(session, "12345678")

        assert identity is not None
        assert identity.twitter_user_id == "12345678"

    async def test_get_active_identities(
        self, session, mock_botcash_client, mock_twitter_client, sample_linked_identity
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        identities = await service.get_active_identities(session)

        assert len(identities) == 1
        assert identities[0].id == sample_linked_identity.id


class TestIdentityServiceTokenManagement:
    """Tests for token management methods."""

    async def test_get_token(
        self, session, mock_botcash_client, mock_twitter_client, sample_oauth_token
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        token = await service.get_token(session, "12345678")

        assert token is not None
        assert token.access_token == "test_access_token"

    async def test_get_valid_access_token(
        self, session, mock_botcash_client, mock_twitter_client, sample_oauth_token
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        token = await service.get_valid_access_token(session, "12345678")

        assert token == "test_access_token"

    async def test_get_valid_access_token_refreshes_expired(
        self, session, mock_botcash_client, mock_twitter_client
    ):
        # Create expired token
        token = OAuthToken(
            twitter_user_id="12345678",
            access_token="old_token",
            refresh_token="test_refresh_token",
            scope="tweet.read",
            expires_at=datetime.now(timezone.utc) - timedelta(hours=1),  # Expired
        )
        session.add(token)
        await session.commit()

        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        access_token = await service.get_valid_access_token(session, "12345678")

        assert access_token == "new_access_token"  # From mock refresh
        mock_twitter_client.refresh_access_token.assert_called_once()


class TestIdentityServicePrivacySettings:
    """Tests for privacy settings methods."""

    async def test_set_privacy_mode(
        self, session, mock_botcash_client, mock_twitter_client, sample_linked_identity
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        from botcash_twitter.models import PrivacyMode
        result = await service.set_privacy_mode(
            session, sample_linked_identity.botcash_address, PrivacyMode.FULL_MIRROR
        )

        assert result is True
        await session.refresh(sample_linked_identity)
        assert sample_linked_identity.privacy_mode == PrivacyMode.FULL_MIRROR

    async def test_set_privacy_mode_not_found(
        self, session, mock_botcash_client, mock_twitter_client
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        from botcash_twitter.models import PrivacyMode
        result = await service.set_privacy_mode(
            session, "nonexistent", PrivacyMode.FULL_MIRROR
        )

        assert result is False

    async def test_get_status(
        self, session, mock_botcash_client, mock_twitter_client, sample_linked_identity
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        status = await service.get_status(session, sample_linked_identity.botcash_address)

        assert status is not None
        assert status["twitter_username"] == "testuser"
        assert status["status"] == "active"
        assert status["privacy_mode"] == "selective"

    async def test_get_status_not_found(
        self, session, mock_botcash_client, mock_twitter_client
    ):
        service = IdentityService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
        )

        status = await service.get_status(session, "nonexistent")

        assert status is None
