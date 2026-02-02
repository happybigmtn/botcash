"""Identity linking service for Telegram <-> Botcash bridge."""

from datetime import datetime, timedelta, timezone
from typing import Optional

import structlog
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from .botcash_client import BotcashClient
from .models import LinkedIdentity, LinkStatus, PrivacyMode

logger = structlog.get_logger()

# Challenge validity period
CHALLENGE_EXPIRY_MINUTES = 10


class IdentityLinkError(Exception):
    """Error during identity linking process."""
    pass


class IdentityService:
    """Service for managing identity links between Telegram and Botcash."""

    def __init__(self, botcash_client: BotcashClient):
        """Initialize identity service.

        Args:
            botcash_client: Botcash RPC client
        """
        self.botcash = botcash_client

    async def get_linked_identity(
        self,
        session: AsyncSession,
        telegram_user_id: int,
    ) -> Optional[LinkedIdentity]:
        """Get linked identity for Telegram user.

        Args:
            session: Database session
            telegram_user_id: Telegram user ID

        Returns:
            LinkedIdentity if found and active, None otherwise
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.telegram_user_id == telegram_user_id,
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
        telegram_user_id: int,
        telegram_username: Optional[str],
        botcash_address: str,
    ) -> tuple[str, str]:
        """Initiate identity linking process.

        Creates a pending link record with a challenge that the user must sign.

        Args:
            session: Database session
            telegram_user_id: Telegram user ID
            telegram_username: Telegram username (optional)
            botcash_address: Botcash address to link

        Returns:
            Tuple of (challenge, verification_message) for user to sign

        Raises:
            IdentityLinkError: If address is invalid or already linked
        """
        # Validate Botcash address
        if not await self.botcash.validate_address(botcash_address):
            raise IdentityLinkError(f"Invalid Botcash address: {botcash_address}")

        # Check if address is already linked to another Telegram user
        existing_by_address = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.botcash_address == botcash_address,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        if existing_by_address.scalar_one_or_none():
            raise IdentityLinkError("This Botcash address is already linked to another Telegram account")

        # Check if Telegram user already has an active link
        existing_by_user = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.telegram_user_id == telegram_user_id,
                LinkedIdentity.status.in_([LinkStatus.ACTIVE, LinkStatus.PENDING]),
            )
        )
        existing = existing_by_user.scalar_one_or_none()

        # Generate challenge
        challenge = self.botcash.generate_challenge()
        expires_at = datetime.now(timezone.utc) + timedelta(minutes=CHALLENGE_EXPIRY_MINUTES)

        if existing:
            # Update existing pending/active record
            if existing.status == LinkStatus.ACTIVE:
                raise IdentityLinkError(
                    "You already have a linked address. Use /unlink first."
                )
            existing.botcash_address = botcash_address
            existing.telegram_username = telegram_username
            existing.challenge = challenge
            existing.challenge_expires_at = expires_at
        else:
            # Create new pending link
            identity = LinkedIdentity(
                telegram_user_id=telegram_user_id,
                telegram_username=telegram_username,
                botcash_address=botcash_address,
                status=LinkStatus.PENDING,
                challenge=challenge,
                challenge_expires_at=expires_at,
            )
            session.add(identity)

        await session.commit()

        # Generate verification message
        verification_msg = (
            f"I am linking Telegram user {telegram_user_id} to Botcash.\n"
            f"Challenge: {challenge}"
        )

        return challenge, verification_msg

    async def complete_link(
        self,
        session: AsyncSession,
        telegram_user_id: int,
        signature: str,
    ) -> LinkedIdentity:
        """Complete identity linking by verifying signature.

        Args:
            session: Database session
            telegram_user_id: Telegram user ID
            signature: Signature of the challenge message

        Returns:
            The completed LinkedIdentity

        Raises:
            IdentityLinkError: If verification fails
        """
        # Get pending link
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.telegram_user_id == telegram_user_id,
                LinkedIdentity.status == LinkStatus.PENDING,
            )
        )
        identity = result.scalar_one_or_none()

        if not identity:
            raise IdentityLinkError("No pending link found. Use /link <address> first.")

        # Check expiry
        if identity.challenge_expires_at:
            # Handle both timezone-aware and naive datetimes (SQLite stores naive)
            expires_at = identity.challenge_expires_at
            if expires_at.tzinfo is None:
                expires_at = expires_at.replace(tzinfo=timezone.utc)
            if datetime.now(timezone.utc) > expires_at:
                raise IdentityLinkError("Challenge expired. Please start over with /link <address>")

        # For now, we trust the signature (in production, verify against Botcash address)
        # TODO: Implement proper signature verification via z_verifymessage RPC
        if len(signature) < 64:
            raise IdentityLinkError("Invalid signature format. Expected hex-encoded signature.")

        # Create on-chain bridge link
        link_result = await self.botcash.create_bridge_link(
            botcash_address=identity.botcash_address,
            platform="telegram",
            platform_id=str(telegram_user_id),
            proof=signature,
        )

        if not link_result.success:
            raise IdentityLinkError(f"Failed to create on-chain link: {link_result.error}")

        # Update identity status
        identity.status = LinkStatus.ACTIVE
        identity.link_tx_id = link_result.tx_id
        identity.linked_at = datetime.now(timezone.utc)
        identity.challenge = None
        identity.challenge_expires_at = None

        await session.commit()

        logger.info(
            "Identity linked",
            telegram_user_id=telegram_user_id,
            botcash_address=identity.botcash_address,
            tx_id=link_result.tx_id,
        )

        return identity

    async def unlink(
        self,
        session: AsyncSession,
        telegram_user_id: int,
    ) -> bool:
        """Unlink Telegram account from Botcash address.

        Args:
            session: Database session
            telegram_user_id: Telegram user ID

        Returns:
            True if unlinked successfully, False if no link found
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.telegram_user_id == telegram_user_id,
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
            telegram_user_id=telegram_user_id,
            botcash_address=identity.botcash_address,
        )

        return True

    async def set_privacy_mode(
        self,
        session: AsyncSession,
        telegram_user_id: int,
        mode: PrivacyMode,
    ) -> bool:
        """Set user's privacy mode.

        Args:
            session: Database session
            telegram_user_id: Telegram user ID
            mode: Privacy mode to set

        Returns:
            True if updated, False if no link found
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.telegram_user_id == telegram_user_id,
                LinkedIdentity.status == LinkStatus.ACTIVE,
            )
        )
        identity = result.scalar_one_or_none()

        if not identity:
            return False

        identity.privacy_mode = mode
        await session.commit()

        return True
