"""Tests for identity service."""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from botcash_activitypub.identity import (
    ActorNotFoundError,
    IdentityLinkError,
    IdentityService,
    generate_rsa_keypair,
)
from botcash_activitypub.models import LinkedIdentity, LinkStatus, PrivacyMode


class TestGenerateRsaKeypair:
    """Tests for RSA key generation."""

    def test_generate_keypair(self):
        """Test RSA keypair generation."""
        # Note: returns (public_key, private_key) - this is the correct order
        public_key, private_key = generate_rsa_keypair()

        assert "-----BEGIN PUBLIC KEY-----" in public_key
        assert "-----END PUBLIC KEY-----" in public_key
        assert "-----BEGIN PRIVATE KEY-----" in private_key
        assert "-----END PRIVATE KEY-----" in private_key

    def test_keypairs_are_unique(self):
        """Test that generated keypairs are unique."""
        key1 = generate_rsa_keypair()
        key2 = generate_rsa_keypair()
        assert key1[0] != key2[0]
        assert key1[1] != key2[1]


class TestIdentityService:
    """Tests for IdentityService class."""

    @pytest.fixture
    def mock_botcash_client(self):
        """Create mock Botcash client."""
        client = AsyncMock()
        client.validate_address = AsyncMock(return_value=True)
        client.generate_challenge = MagicMock(return_value="test_challenge_123")
        client.compute_challenge_hash = MagicMock(return_value="hash123")
        client.create_bridge_link = AsyncMock(
            return_value=MagicMock(success=True, tx_id="link_tx_123")
        )
        return client

    @pytest.fixture
    def identity_service(self, mock_botcash_client, config):
        """Create IdentityService instance."""
        return IdentityService(
            botcash_client=mock_botcash_client,
            base_url=config.activitypub.base_url,
            domain=config.activitypub.domain,
        )

    @pytest.mark.asyncio
    async def test_get_or_create_actor_new(self, identity_service, session):
        """Test creating a new actor."""
        actor = await identity_service.get_or_create_actor(session, "bs1newuser12345678901")

        assert actor is not None
        assert actor.botcash_address == "bs1newuser12345678901"
        assert actor.status == LinkStatus.ACTIVE
        assert actor.public_key_pem is not None
        assert actor.private_key_pem is not None

    @pytest.mark.asyncio
    async def test_get_or_create_actor_existing(self, identity_service, session):
        """Test getting existing actor."""
        # Create first
        actor1 = await identity_service.get_or_create_actor(session, "bs1existing123456789")
        # Get again
        actor2 = await identity_service.get_or_create_actor(session, "bs1existing123456789")

        assert actor1.actor_id == actor2.actor_id
        assert actor1.public_key_pem == actor2.public_key_pem

    @pytest.mark.asyncio
    async def test_get_actor_by_local_part(self, identity_service, session):
        """Test getting actor by local part (username)."""
        await identity_service.get_or_create_actor(session, "bs1localpart12345678901")
        # Local part is truncated to 20 chars
        actor = await identity_service.get_actor_by_local_part(session, "bs1localpart12345678")

        assert actor is not None
        assert actor.actor_local_part == "bs1localpart12345678"

    @pytest.mark.asyncio
    async def test_get_actor_by_local_part_not_found(self, identity_service, session):
        """Test getting non-existent actor by local part."""
        actor = await identity_service.get_actor_by_local_part(session, "nonexistent")
        assert actor is None

    @pytest.mark.asyncio
    async def test_get_actor_by_address(self, identity_service, session):
        """Test getting actor by Botcash address."""
        await identity_service.get_or_create_actor(session, "bs1byaddress123456789")
        actor = await identity_service.get_actor_by_address(session, "bs1byaddress123456789")

        assert actor is not None
        assert actor.botcash_address == "bs1byaddress123456789"

    @pytest.mark.asyncio
    async def test_webfinger_lookup(self, identity_service, session):
        """Test WebFinger response generation."""
        await identity_service.get_or_create_actor(session, "bs1webfinger1234567890")
        # Local part is truncated to 20 chars (bs1webfinger12345678)
        response = await identity_service.webfinger_lookup(
            session, "acct:bs1webfinger12345678@test.botcash.social"
        )

        assert response is not None
        assert "subject" in response
        assert "links" in response
        assert len(response["links"]) > 0

    @pytest.mark.asyncio
    async def test_webfinger_not_found(self, identity_service, session):
        """Test WebFinger response for non-existent user."""
        response = await identity_service.webfinger_lookup(
            session, "acct:nonexistent@test.botcash.social"
        )
        assert response is None

    @pytest.mark.asyncio
    async def test_initiate_remote_link(self, identity_service, session, mock_botcash_client):
        """Test initiating a remote identity link."""
        actor_url = "https://mastodon.social/users/alice"
        botcash_address = "bs1remotelink123456789"

        challenge, message = await identity_service.initiate_remote_link(
            session, actor_url, botcash_address
        )

        assert challenge is not None
        assert challenge == "test_challenge_123"
        assert "Challenge:" in message

    @pytest.mark.asyncio
    async def test_initiate_remote_link_invalid_address(
        self, identity_service, session, mock_botcash_client
    ):
        """Test initiating link with invalid address."""
        mock_botcash_client.validate_address.return_value = False

        with pytest.raises(IdentityLinkError) as exc_info:
            await identity_service.initiate_remote_link(
                session, "https://mastodon.social/users/alice", "invalid_address"
            )

        assert "Invalid Botcash address" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_set_privacy_mode(self, identity_service, session):
        """Test setting privacy mode."""
        actor = await identity_service.get_or_create_actor(session, "bs1privacy12345678901")
        result = await identity_service.set_privacy_mode(
            session, actor.actor_id, PrivacyMode.READ_ONLY
        )

        assert result is True

        # Verify the change
        updated = await identity_service.get_actor_by_address(session, "bs1privacy12345678901")
        assert updated.privacy_mode == PrivacyMode.READ_ONLY

    @pytest.mark.asyncio
    async def test_unlink(self, identity_service, session):
        """Test unlinking an identity."""
        actor = await identity_service.get_or_create_actor(session, "bs1unlink123456789012")
        result = await identity_service.unlink(session, actor.actor_id)

        assert result is True

        # Verify the status change
        updated = await identity_service.get_actor_by_address(session, "bs1unlink123456789012")
        # After unlink, the identity should not be returned by get_actor_by_address
        # because it filters by ACTIVE status
        assert updated is None


class TestActorNotFoundError:
    """Tests for ActorNotFoundError."""

    def test_error_message(self):
        """Test error message formatting."""
        error = ActorNotFoundError("https://example.com/users/test")
        assert "https://example.com/users/test" in str(error)


class TestIdentityLinkError:
    """Tests for IdentityLinkError."""

    def test_error_message(self):
        """Test error message formatting."""
        error = IdentityLinkError("Link verification failed")
        assert "Link verification failed" in str(error)
