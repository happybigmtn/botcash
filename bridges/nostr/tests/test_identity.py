"""Tests for identity linking service."""

from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock

import pytest

from botcash_nostr.botcash_client import PostResult
from botcash_nostr.identity import CHALLENGE_EXPIRY_MINUTES, IdentityLinkError, IdentityService
from botcash_nostr.models import LinkedIdentity, LinkStatus, PrivacyMode
from botcash_nostr.nostr_types import hex_to_npub


@pytest.fixture
def mock_botcash_client():
    """Create a mock Botcash client."""
    client = MagicMock()
    client.validate_address = AsyncMock(return_value=True)
    client.generate_challenge = MagicMock(return_value="challenge123")
    client.create_bridge_link = AsyncMock(return_value=PostResult(
        tx_id="tx_abc123",
        success=True,
    ))
    return client


@pytest.fixture
def identity_service(mock_botcash_client):
    """Create an identity service with mock client."""
    return IdentityService(mock_botcash_client)


class TestGetLinkedIdentity:
    """Tests for getting linked identity."""

    @pytest.mark.asyncio
    async def test_get_active_identity(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test getting an active linked identity."""
        # Create active identity
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        result = await identity_service.get_linked_identity(db_session, sample_nostr_pubkey)
        assert result is not None
        assert result.botcash_address == sample_botcash_address

    @pytest.mark.asyncio
    async def test_get_identity_by_npub(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test getting identity by npub format."""
        npub = hex_to_npub(sample_nostr_pubkey)

        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        result = await identity_service.get_linked_identity(db_session, npub)
        assert result is not None

    @pytest.mark.asyncio
    async def test_get_nonexistent_identity(self, db_session, identity_service, sample_nostr_pubkey):
        """Test getting a nonexistent identity."""
        result = await identity_service.get_linked_identity(db_session, sample_nostr_pubkey)
        assert result is None

    @pytest.mark.asyncio
    async def test_get_unlinked_identity_returns_none(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test that unlinked identity is not returned."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.UNLINKED,
        )
        db_session.add(identity)
        await db_session.commit()

        result = await identity_service.get_linked_identity(db_session, sample_nostr_pubkey)
        assert result is None


class TestInitiateLink:
    """Tests for initiating identity link."""

    @pytest.mark.asyncio
    async def test_initiate_link_success(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test successful link initiation."""
        challenge, msg = await identity_service.initiate_link(
            db_session, sample_nostr_pubkey, sample_botcash_address
        )

        assert challenge == "challenge123"
        assert "Challenge" in msg
        assert sample_botcash_address in msg

    @pytest.mark.asyncio
    async def test_initiate_link_validates_address(
        self, db_session, identity_service, mock_botcash_client, sample_nostr_pubkey
    ):
        """Test that invalid address is rejected."""
        mock_botcash_client.validate_address = AsyncMock(return_value=False)

        with pytest.raises(IdentityLinkError, match="Invalid Botcash address"):
            await identity_service.initiate_link(
                db_session, sample_nostr_pubkey, "invalid_address"
            )

    @pytest.mark.asyncio
    async def test_initiate_link_validates_pubkey(
        self, db_session, identity_service, sample_botcash_address
    ):
        """Test that invalid pubkey is rejected."""
        with pytest.raises(IdentityLinkError, match="Invalid Nostr pubkey"):
            await identity_service.initiate_link(
                db_session, "short_key", sample_botcash_address
            )

    @pytest.mark.asyncio
    async def test_initiate_link_rejects_duplicate_address(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test that already-linked address is rejected."""
        # Create existing link
        existing = LinkedIdentity(
            nostr_pubkey="b" * 64,  # Different pubkey
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(existing)
        await db_session.commit()

        with pytest.raises(IdentityLinkError, match="already linked"):
            await identity_service.initiate_link(
                db_session, sample_nostr_pubkey, sample_botcash_address
            )

    @pytest.mark.asyncio
    async def test_initiate_link_rejects_already_linked_user(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test that user with active link is rejected."""
        existing = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address="bs1other",
            status=LinkStatus.ACTIVE,
        )
        db_session.add(existing)
        await db_session.commit()

        with pytest.raises(IdentityLinkError, match="already have a linked"):
            await identity_service.initiate_link(
                db_session, sample_nostr_pubkey, sample_botcash_address
            )

    @pytest.mark.asyncio
    async def test_initiate_link_updates_pending(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test that pending link is updated."""
        # Create pending link
        existing = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address="bs1old",
            status=LinkStatus.PENDING,
        )
        db_session.add(existing)
        await db_session.commit()

        # Initiate new link
        await identity_service.initiate_link(
            db_session, sample_nostr_pubkey, sample_botcash_address
        )

        # Verify update
        await db_session.refresh(existing)
        assert existing.botcash_address == sample_botcash_address

    @pytest.mark.asyncio
    async def test_initiate_link_with_npub(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test link initiation with npub format."""
        npub = hex_to_npub(sample_nostr_pubkey)

        challenge, msg = await identity_service.initiate_link(
            db_session, npub, sample_botcash_address
        )

        assert challenge is not None


class TestCompleteLink:
    """Tests for completing identity link."""

    @pytest.mark.asyncio
    async def test_complete_link_success(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test successful link completion."""
        # Create pending link
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.PENDING,
            challenge="test_challenge",
            challenge_expires_at=datetime.now(timezone.utc) + timedelta(minutes=5),
        )
        db_session.add(identity)
        await db_session.commit()

        result = await identity_service.complete_link(
            db_session, sample_nostr_pubkey, "a" * 128
        )

        assert result.status == LinkStatus.ACTIVE
        assert result.link_tx_id == "tx_abc123"
        assert result.challenge is None

    @pytest.mark.asyncio
    async def test_complete_link_no_pending(
        self, db_session, identity_service, sample_nostr_pubkey
    ):
        """Test completing link with no pending link."""
        with pytest.raises(IdentityLinkError, match="No pending link"):
            await identity_service.complete_link(
                db_session, sample_nostr_pubkey, "a" * 128
            )

    @pytest.mark.asyncio
    async def test_complete_link_expired(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test completing expired link."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.PENDING,
            challenge="test_challenge",
            challenge_expires_at=datetime.now(timezone.utc) - timedelta(minutes=5),
        )
        db_session.add(identity)
        await db_session.commit()

        with pytest.raises(IdentityLinkError, match="expired"):
            await identity_service.complete_link(
                db_session, sample_nostr_pubkey, "a" * 128
            )

    @pytest.mark.asyncio
    async def test_complete_link_invalid_signature(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test completing link with invalid signature."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.PENDING,
            challenge="test_challenge",
            challenge_expires_at=datetime.now(timezone.utc) + timedelta(minutes=5),
        )
        db_session.add(identity)
        await db_session.commit()

        with pytest.raises(IdentityLinkError, match="Invalid signature"):
            await identity_service.complete_link(
                db_session, sample_nostr_pubkey, "short"
            )

    @pytest.mark.asyncio
    async def test_complete_link_bridge_failure(
        self, db_session, identity_service, mock_botcash_client,
        sample_nostr_pubkey, sample_botcash_address
    ):
        """Test completing link when bridge fails."""
        mock_botcash_client.create_bridge_link = AsyncMock(
            return_value=PostResult(tx_id="", success=False, error="Network error")
        )

        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.PENDING,
            challenge="test_challenge",
            challenge_expires_at=datetime.now(timezone.utc) + timedelta(minutes=5),
        )
        db_session.add(identity)
        await db_session.commit()

        with pytest.raises(IdentityLinkError, match="Failed to create"):
            await identity_service.complete_link(
                db_session, sample_nostr_pubkey, "a" * 128
            )


class TestUnlink:
    """Tests for unlinking identity."""

    @pytest.mark.asyncio
    async def test_unlink_success(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test successful unlink."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        result = await identity_service.unlink(db_session, sample_nostr_pubkey)
        assert result is True

        await db_session.refresh(identity)
        assert identity.status == LinkStatus.UNLINKED
        assert identity.unlinked_at is not None

    @pytest.mark.asyncio
    async def test_unlink_no_link(self, db_session, identity_service, sample_nostr_pubkey):
        """Test unlinking when no link exists."""
        result = await identity_service.unlink(db_session, sample_nostr_pubkey)
        assert result is False


class TestSetPrivacyMode:
    """Tests for setting privacy mode."""

    @pytest.mark.asyncio
    async def test_set_privacy_mode_success(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test successful privacy mode change."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
            privacy_mode=PrivacyMode.SELECTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        result = await identity_service.set_privacy_mode(
            db_session, sample_nostr_pubkey, PrivacyMode.FULL_MIRROR
        )
        assert result is True

        await db_session.refresh(identity)
        assert identity.privacy_mode == PrivacyMode.FULL_MIRROR

    @pytest.mark.asyncio
    async def test_set_privacy_mode_no_link(
        self, db_session, identity_service, sample_nostr_pubkey
    ):
        """Test setting privacy mode when no link exists."""
        result = await identity_service.set_privacy_mode(
            db_session, sample_nostr_pubkey, PrivacyMode.PRIVATE
        )
        assert result is False


class TestHelperMethods:
    """Tests for helper methods."""

    @pytest.mark.asyncio
    async def test_get_identity_by_address(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test getting identity by Botcash address."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        result = await identity_service.get_identity_by_address(
            db_session, sample_botcash_address
        )
        assert result is not None
        assert result.nostr_pubkey == sample_nostr_pubkey

    @pytest.mark.asyncio
    async def test_get_all_linked_pubkeys(
        self, db_session, identity_service, sample_botcash_address
    ):
        """Test getting all linked pubkeys."""
        # Create multiple identities
        for i in range(3):
            identity = LinkedIdentity(
                nostr_pubkey=f"{i}" * 64,
                botcash_address=f"bs1test{i}",
                status=LinkStatus.ACTIVE,
            )
            db_session.add(identity)

        # Add one unlinked
        unlinked = LinkedIdentity(
            nostr_pubkey="u" * 64,
            botcash_address="bs1unlinked",
            status=LinkStatus.UNLINKED,
        )
        db_session.add(unlinked)
        await db_session.commit()

        pubkeys = await identity_service.get_all_linked_pubkeys(db_session)
        assert len(pubkeys) == 3
        assert "u" * 64 not in pubkeys

    @pytest.mark.asyncio
    async def test_get_botcash_address_for_pubkey(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test getting Botcash address for pubkey."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        address = await identity_service.get_botcash_address_for_pubkey(
            db_session, sample_nostr_pubkey
        )
        assert address == sample_botcash_address

    @pytest.mark.asyncio
    async def test_get_pubkey_for_botcash_address(
        self, db_session, identity_service, sample_nostr_pubkey, sample_botcash_address
    ):
        """Test getting pubkey for Botcash address."""
        identity = LinkedIdentity(
            nostr_pubkey=sample_nostr_pubkey,
            botcash_address=sample_botcash_address,
            status=LinkStatus.ACTIVE,
        )
        db_session.add(identity)
        await db_session.commit()

        pubkey = await identity_service.get_pubkey_for_botcash_address(
            db_session, sample_botcash_address
        )
        assert pubkey == sample_nostr_pubkey
