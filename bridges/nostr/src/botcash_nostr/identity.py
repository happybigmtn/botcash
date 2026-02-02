"""Identity linking service for Nostr <-> Botcash bridge."""

from datetime import datetime, timedelta, timezone
from typing import Optional

import structlog
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from .botcash_client import BotcashClient
from .models import LinkedIdentity, LinkStatus, PrivacyMode
from .nostr_types import hex_to_npub, npub_to_hex

logger = structlog.get_logger()

# Challenge validity period
CHALLENGE_EXPIRY_MINUTES = 10


class IdentityLinkError(Exception):
    """Error during identity linking process."""
    pass


class IdentityService:
    """Service for managing identity links between Nostr and Botcash."""

    def __init__(self, botcash_client: BotcashClient):
        """Initialize identity service.

        Args:
            botcash_client: Botcash RPC client
        """
        self.botcash = botcash_client

    async def get_linked_identity(
        self,
        session: AsyncSession,
        nostr_pubkey: str,
    ) -> Optional[LinkedIdentity]:
        """Get linked identity for Nostr pubkey.

        Args:
            session: Database session
            nostr_pubkey: Nostr public key (hex, 64 chars)

        Returns:
            LinkedIdentity if found and active, None otherwise
        """
        # Normalize pubkey (accept npub or hex)
        if nostr_pubkey.startswith("npub"):
            nostr_pubkey = npub_to_hex(nostr_pubkey)

        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.nostr_pubkey == nostr_pubkey,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        return result.scalar_one_or_none()

    async def get_identity_by_address(
        self,
        session: AsyncSession,
        botcash_address: str,
    ) -> Optional[LinkedIdentity]:
        """Get linked identity by Botcash address.

        Args:
            session: Database session
            botcash_address: Botcash address

        Returns:
            LinkedIdentity if found and active, None otherwise
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.botcash_address == botcash_address,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        return result.scalar_one_or_none()

    async def initiate_link(
        self,
        session: AsyncSession,
        nostr_pubkey: str,
        botcash_address: str,
    ) -> tuple[str, str]:
        """Initiate identity linking process.

        Creates a pending link record with a challenge that the user must sign.

        Args:
            session: Database session
            nostr_pubkey: Nostr public key (hex or npub)
            botcash_address: Botcash address to link

        Returns:
            Tuple of (challenge, verification_message) for user to sign

        Raises:
            IdentityLinkError: If address is invalid or already linked
        """
        # Normalize pubkey
        if nostr_pubkey.startswith("npub"):
            nostr_pubkey = npub_to_hex(nostr_pubkey)

        if len(nostr_pubkey) != 64:
            raise IdentityLinkError(f"Invalid Nostr pubkey: must be 64 hex characters")

        # Get npub format for display
        nostr_npub = hex_to_npub(nostr_pubkey)

        # Validate Botcash address
        if not await self.botcash.validate_address(botcash_address):
            raise IdentityLinkError(f"Invalid Botcash address: {botcash_address}")

        # Check if address is already linked to another Nostr user
        existing_by_address = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.botcash_address == botcash_address,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        if existing_by_address.scalar_one_or_none():
            raise IdentityLinkError("This Botcash address is already linked to another Nostr account")

        # Check if Nostr user already has an active link
        existing_by_pubkey = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.nostr_pubkey == nostr_pubkey,
                LinkedIdentity.status.in_([LinkStatus.ACTIVE, LinkStatus.PENDING]),
            )
        )
        existing = existing_by_pubkey.scalar_one_or_none()

        # Generate challenge
        challenge = self.botcash.generate_challenge()
        expires_at = datetime.now(timezone.utc) + timedelta(minutes=CHALLENGE_EXPIRY_MINUTES)

        if existing:
            # Update existing pending/active record
            if existing.status == LinkStatus.ACTIVE:
                raise IdentityLinkError(
                    "You already have a linked address. Unlink first."
                )
            existing.botcash_address = botcash_address
            existing.nostr_npub = nostr_npub
            existing.challenge = challenge
            existing.challenge_expires_at = expires_at
        else:
            # Create new pending link
            identity = LinkedIdentity(
                nostr_pubkey=nostr_pubkey,
                nostr_npub=nostr_npub,
                botcash_address=botcash_address,
                status=LinkStatus.PENDING,
                challenge=challenge,
                challenge_expires_at=expires_at,
            )
            session.add(identity)

        await session.commit()

        # Generate verification message (to be signed as Nostr event)
        verification_msg = (
            f"I am linking Nostr pubkey {nostr_npub} to Botcash address {botcash_address}.\n"
            f"Challenge: {challenge}"
        )

        return challenge, verification_msg

    async def complete_link(
        self,
        session: AsyncSession,
        nostr_pubkey: str,
        signature: str,
        event_id: str | None = None,
    ) -> LinkedIdentity:
        """Complete identity linking by verifying signature.

        Args:
            session: Database session
            nostr_pubkey: Nostr public key (hex or npub)
            signature: Schnorr signature of the challenge message
            event_id: Optional Nostr event ID of the link announcement

        Returns:
            The completed LinkedIdentity

        Raises:
            IdentityLinkError: If verification fails
        """
        # Normalize pubkey
        if nostr_pubkey.startswith("npub"):
            nostr_pubkey = npub_to_hex(nostr_pubkey)

        # Get pending link
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.nostr_pubkey == nostr_pubkey,
                LinkedIdentity.status == LinkStatus.PENDING,
            )
        )
        identity = result.scalar_one_or_none()

        if not identity:
            raise IdentityLinkError("No pending link found. Initiate link first.")

        # Check expiry
        if identity.challenge_expires_at:
            # Handle both timezone-aware and naive datetimes (SQLite stores naive)
            expires_at = identity.challenge_expires_at
            if expires_at.tzinfo is None:
                expires_at = expires_at.replace(tzinfo=timezone.utc)
            if datetime.now(timezone.utc) > expires_at:
                raise IdentityLinkError("Challenge expired. Please start over.")

        # Validate signature format (Schnorr signatures are 64 bytes = 128 hex chars)
        if len(signature) < 128:
            raise IdentityLinkError("Invalid signature format. Expected 128-char hex Schnorr signature.")

        # Create on-chain bridge link
        link_result = await self.botcash.create_bridge_link(
            botcash_address=identity.botcash_address,
            platform="nostr",
            platform_id=nostr_pubkey,
            proof=signature,
        )

        if not link_result.success:
            raise IdentityLinkError(f"Failed to create on-chain link: {link_result.error}")

        # Update identity status
        identity.status = LinkStatus.ACTIVE
        identity.link_tx_id = link_result.tx_id
        identity.link_event_id = event_id
        identity.linked_at = datetime.now(timezone.utc)
        identity.challenge = None
        identity.challenge_expires_at = None

        await session.commit()

        logger.info(
            "Identity linked",
            nostr_pubkey=nostr_pubkey,
            nostr_npub=identity.nostr_npub,
            botcash_address=identity.botcash_address,
            tx_id=link_result.tx_id,
        )

        return identity

    async def unlink(
        self,
        session: AsyncSession,
        nostr_pubkey: str,
    ) -> bool:
        """Unlink Nostr account from Botcash address.

        Args:
            session: Database session
            nostr_pubkey: Nostr public key (hex or npub)

        Returns:
            True if unlinked successfully, False if no link found
        """
        # Normalize pubkey
        if nostr_pubkey.startswith("npub"):
            nostr_pubkey = npub_to_hex(nostr_pubkey)

        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.nostr_pubkey == nostr_pubkey,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        identity = result.scalar_one_or_none()

        if not identity:
            return False

        identity.status = LinkStatus.UNLINKED
        identity.unlinked_at = datetime.now(timezone.utc)
        await session.commit()

        logger.info(
            "Identity unlinked",
            nostr_pubkey=nostr_pubkey,
            botcash_address=identity.botcash_address,
        )

        return True

    async def set_privacy_mode(
        self,
        session: AsyncSession,
        nostr_pubkey: str,
        mode: PrivacyMode,
    ) -> bool:
        """Set user's privacy mode.

        Args:
            session: Database session
            nostr_pubkey: Nostr public key (hex or npub)
            mode: Privacy mode to set

        Returns:
            True if updated, False if no link found
        """
        # Normalize pubkey
        if nostr_pubkey.startswith("npub"):
            nostr_pubkey = npub_to_hex(nostr_pubkey)

        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.nostr_pubkey == nostr_pubkey,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        identity = result.scalar_one_or_none()

        if not identity:
            return False

        identity.privacy_mode = mode
        await session.commit()

        return True

    async def get_all_linked_pubkeys(
        self,
        session: AsyncSession,
    ) -> list[str]:
        """Get all linked Nostr pubkeys.

        Args:
            session: Database session

        Returns:
            List of linked Nostr pubkeys (hex)
        """
        result = await session.execute(
            select(LinkedIdentity.nostr_pubkey).where(
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        return [row[0] for row in result.fetchall()]

    async def get_botcash_address_for_pubkey(
        self,
        session: AsyncSession,
        nostr_pubkey: str,
    ) -> str | None:
        """Get Botcash address for a Nostr pubkey.

        Args:
            session: Database session
            nostr_pubkey: Nostr public key (hex or npub)

        Returns:
            Botcash address or None if not linked
        """
        identity = await self.get_linked_identity(session, nostr_pubkey)
        return identity.botcash_address if identity else None

    async def get_pubkey_for_botcash_address(
        self,
        session: AsyncSession,
        botcash_address: str,
    ) -> str | None:
        """Get Nostr pubkey for a Botcash address.

        Args:
            session: Database session
            botcash_address: Botcash address

        Returns:
            Nostr pubkey (hex) or None if not linked
        """
        identity = await self.get_identity_by_address(session, botcash_address)
        return identity.nostr_pubkey if identity else None
