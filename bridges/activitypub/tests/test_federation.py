"""Tests for federation service (Inbox/Outbox handlers)."""

import base64
import hashlib
import pytest
from unittest.mock import AsyncMock, MagicMock

from botcash_activitypub.activitypub_types import (
    AS_PUBLIC,
    Activity,
    ActivityType,
)
from botcash_activitypub.federation import (
    FederationError,
    FederationService,
    compute_digest,
    create_signature_string,
    sign_request,
)
from botcash_activitypub.identity import IdentityService, generate_rsa_keypair
from botcash_activitypub.models import LinkedIdentity, LinkStatus
from botcash_activitypub.protocol_mapper import ProtocolMapper


class TestComputeDigest:
    """Tests for HTTP digest computation."""

    def test_compute_digest(self):
        """Test SHA-256 digest computation."""
        body = b'{"type": "Create"}'
        digest = compute_digest(body)

        assert digest.startswith("SHA-256=")
        # Verify the hash is correct
        expected_hash = base64.b64encode(hashlib.sha256(body).digest()).decode()
        assert digest == f"SHA-256={expected_hash}"

    def test_compute_digest_empty_body(self):
        """Test digest of empty body."""
        digest = compute_digest(b"")
        assert digest.startswith("SHA-256=")


class TestCreateSignatureString:
    """Tests for HTTP signature string creation."""

    def test_create_signature_string(self):
        """Test creating signature string from headers."""
        headers = {
            "host": "mastodon.social",
            "date": "Mon, 01 Jan 2024 00:00:00 GMT",
            "digest": "SHA-256=abc123",
        }
        signed_headers = ["(request-target)", "host", "date", "digest"]

        result = create_signature_string(
            method="post",
            path="/inbox",
            headers=headers,
            signed_headers=signed_headers,
        )

        assert "(request-target): post /inbox" in result
        assert "host: mastodon.social" in result
        assert "date: Mon, 01 Jan 2024 00:00:00 GMT" in result
        assert "digest: SHA-256=abc123" in result
        # Headers should be joined by newlines
        assert "\n" in result


class TestSignRequest:
    """Tests for HTTP request signing."""

    def test_sign_request(self):
        """Test signing an HTTP request."""
        # Generate a test key pair
        public_key_pem, private_key_pem = generate_rsa_keypair()

        headers = {}
        signature = sign_request(
            private_key_pem=private_key_pem,
            key_id="https://botcash.social/users/bs1test#main-key",
            method="POST",
            url="https://mastodon.social/users/alice/inbox",
            headers=headers,
            body=b'{"type": "Follow"}',
        )

        assert "keyId=" in signature
        assert "algorithm=" in signature
        assert "headers=" in signature
        assert "signature=" in signature
        # Headers should be populated
        assert "Digest" in headers
        assert "Date" in headers
        assert "Host" in headers


class TestFederationService:
    """Tests for FederationService class."""

    @pytest.fixture
    def mock_identity_service(self):
        """Create mock IdentityService."""
        service = AsyncMock(spec=IdentityService)
        return service

    @pytest.fixture
    def mock_botcash_client(self):
        """Create mock Botcash client."""
        client = AsyncMock()
        client.create_post = AsyncMock(return_value=MagicMock(success=True, tx_id="tx123"))
        client.follow = AsyncMock(return_value=MagicMock(success=True, tx_id="tx456"))
        client.unfollow = AsyncMock(return_value=MagicMock(success=True, tx_id="tx789"))
        client.upvote = AsyncMock(return_value=MagicMock(success=True, tx_id="tx012"))
        return client

    @pytest.fixture
    def protocol_mapper(self, config):
        """Create ProtocolMapper instance."""
        return ProtocolMapper(
            base_url=config.activitypub.base_url,
            domain=config.activitypub.domain,
        )

    @pytest.fixture
    def federation_service(
        self, mock_identity_service, mock_botcash_client, protocol_mapper, config
    ):
        """Create FederationService instance."""
        return FederationService(
            identity_service=mock_identity_service,
            protocol_mapper=protocol_mapper,
            botcash_client=mock_botcash_client,
            base_url=config.activitypub.base_url,
            domain=config.activitypub.domain,
        )

    @pytest.mark.asyncio
    async def test_handle_inbox_unknown_actor(
        self, federation_service, mock_identity_service, session
    ):
        """Test handling inbox for unknown actor."""
        mock_identity_service.get_actor_by_local_part = AsyncMock(return_value=None)

        activity_data = {
            "id": "https://mastodon.social/activities/test",
            "type": "Follow",
            "actor": "https://mastodon.social/users/alice",
            "object": "https://botcash.social/users/nonexistent",
        }

        with pytest.raises(FederationError):
            await federation_service.handle_inbox(session, "nonexistent", activity_data)


class TestFederationError:
    """Tests for FederationError."""

    def test_error_message(self):
        """Test error message formatting."""
        error = FederationError("Delivery failed to inbox")
        assert "Delivery failed" in str(error)
