"""Identity linking service for X/Twitter <-> Botcash bridge.

Handles OAuth 2.0 PKCE flow to link Twitter accounts to Botcash addresses.
This is a one-way link used for cross-posting Botcash -> Twitter.
"""

from dataclasses import dataclass
from datetime import datetime, timedelta, timezone

import structlog
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from .botcash_client import BotcashClient
from .models import (
    LinkedIdentity,
    LinkStatus,
    OAuthPendingState,
    OAuthToken,
    PrivacyMode,
)
from .twitter_client import (
    OAuthTokenResponse,
    TwitterClient,
    TwitterUser,
    generate_code_challenge,
    generate_code_verifier,
    generate_state,
)

logger = structlog.get_logger()

# OAuth state validity period
STATE_EXPIRY_MINUTES = 10


class IdentityLinkError(Exception):
    """Error during identity linking process."""
    pass


@dataclass
class OAuthState:
    """OAuth state for authorization flow."""
    state: str
    code_verifier: str
    authorization_url: str


class IdentityService:
    """Service for managing Twitter <-> Botcash identity links."""

    def __init__(
        self,
        botcash_client: BotcashClient,
        twitter_client: TwitterClient,
        default_privacy_mode: PrivacyMode = PrivacyMode.SELECTIVE,
    ):
        """Initialize identity service.

        Args:
            botcash_client: Botcash RPC client
            twitter_client: Twitter API client
            default_privacy_mode: Default privacy mode for new links
        """
        self.botcash = botcash_client
        self.twitter = twitter_client
        self.default_privacy_mode = default_privacy_mode

    # === OAuth Flow ===

    async def initiate_link(
        self,
        session: AsyncSession,
        botcash_address: str,
    ) -> OAuthState:
        """Start OAuth flow to link a Twitter account.

        Args:
            session: Database session
            botcash_address: Botcash address to link

        Returns:
            OAuthState with authorization URL

        Raises:
            IdentityLinkError: If address is invalid or already linked
        """
        # Validate Botcash address
        if not await self.botcash.validate_address(botcash_address):
            raise IdentityLinkError(f"Invalid Botcash address: {botcash_address}")

        # Check if address is already linked
        existing = await self.get_identity_by_address(session, botcash_address)
        if existing and existing.status == LinkStatus.ACTIVE:
            raise IdentityLinkError(
                f"This Botcash address is already linked to @{existing.twitter_username}. "
                "Unlink first before linking to a new Twitter account."
            )

        # Generate OAuth parameters
        state = generate_state()
        code_verifier = generate_code_verifier()
        code_challenge = generate_code_challenge(code_verifier)

        # Store pending state
        expires_at = datetime.now(timezone.utc) + timedelta(minutes=STATE_EXPIRY_MINUTES)
        pending_state = OAuthPendingState(
            state=state,
            code_verifier=code_verifier,
            botcash_address=botcash_address,
            expires_at=expires_at,
        )
        session.add(pending_state)
        await session.commit()

        # Generate authorization URL
        auth_url = self.twitter.get_authorization_url(
            state=state,
            code_challenge=code_challenge,
        )

        logger.info(
            "Initiated Twitter link",
            botcash_address=botcash_address,
            state=state[:8] + "...",
        )

        return OAuthState(
            state=state,
            code_verifier=code_verifier,
            authorization_url=auth_url,
        )

    async def complete_link(
        self,
        session: AsyncSession,
        state: str,
        code: str,
    ) -> LinkedIdentity:
        """Complete OAuth flow after user authorizes.

        Args:
            session: Database session
            state: OAuth state from callback
            code: Authorization code from callback

        Returns:
            The created LinkedIdentity

        Raises:
            IdentityLinkError: If verification fails
        """
        # Get pending state
        result = await session.execute(
            select(OAuthPendingState).where(OAuthPendingState.state == state)
        )
        pending = result.scalar_one_or_none()

        if not pending:
            raise IdentityLinkError("Invalid or expired OAuth state")

        # Check expiry
        expires_at = pending.expires_at
        if expires_at.tzinfo is None:
            expires_at = expires_at.replace(tzinfo=timezone.utc)
        if datetime.now(timezone.utc) > expires_at:
            await session.delete(pending)
            await session.commit()
            raise IdentityLinkError("OAuth state expired. Please start over.")

        botcash_address = pending.botcash_address
        code_verifier = pending.code_verifier

        # Exchange code for tokens
        try:
            token_response = await self.twitter.exchange_code_for_token(
                code=code,
                code_verifier=code_verifier,
            )
        except Exception as e:
            await session.delete(pending)
            await session.commit()
            raise IdentityLinkError(f"Failed to exchange authorization code: {e}")

        # Get Twitter user info
        try:
            twitter_user = await self.twitter.get_me(token_response.access_token)
        except Exception as e:
            await session.delete(pending)
            await session.commit()
            raise IdentityLinkError(f"Failed to get Twitter user info: {e}")

        # Check if this Twitter account is already linked
        existing_twitter = await self.get_identity_by_twitter_id(session, twitter_user.id)
        if existing_twitter and existing_twitter.status == LinkStatus.ACTIVE:
            if existing_twitter.botcash_address != botcash_address:
                await session.delete(pending)
                await session.commit()
                raise IdentityLinkError(
                    f"This Twitter account (@{twitter_user.username}) is already linked "
                    f"to a different Botcash address. Unlink it first."
                )
            # Re-linking same account - just update tokens
            identity = existing_twitter
        else:
            # Check if we have an existing record for this address
            existing_address = await self.get_identity_by_address(session, botcash_address)
            if existing_address:
                identity = existing_address
                identity.twitter_user_id = twitter_user.id
                identity.twitter_username = twitter_user.username
                identity.twitter_display_name = twitter_user.name
            else:
                # Create new identity
                identity = LinkedIdentity(
                    twitter_user_id=twitter_user.id,
                    twitter_username=twitter_user.username,
                    twitter_display_name=twitter_user.name,
                    botcash_address=botcash_address,
                    status=LinkStatus.ACTIVE,
                    privacy_mode=self.default_privacy_mode,
                    linked_at=datetime.now(timezone.utc),
                )
                session.add(identity)

        # Update status
        identity.status = LinkStatus.ACTIVE
        identity.linked_at = datetime.now(timezone.utc)
        identity.unlinked_at = None

        # Store OAuth tokens
        existing_token = await session.execute(
            select(OAuthToken).where(OAuthToken.twitter_user_id == twitter_user.id)
        )
        token = existing_token.scalar_one_or_none()

        if token:
            token.access_token = token_response.access_token
            token.refresh_token = token_response.refresh_token
            token.scope = token_response.scope
            if token_response.expires_in:
                token.expires_at = datetime.now(timezone.utc) + timedelta(
                    seconds=token_response.expires_in
                )
        else:
            token = OAuthToken(
                twitter_user_id=twitter_user.id,
                access_token=token_response.access_token,
                refresh_token=token_response.refresh_token,
                token_type=token_response.token_type,
                scope=token_response.scope,
                expires_at=(
                    datetime.now(timezone.utc) + timedelta(seconds=token_response.expires_in)
                    if token_response.expires_in else None
                ),
            )
            session.add(token)

        # Create on-chain bridge link
        link_result = await self.botcash.create_bridge_link(
            botcash_address=botcash_address,
            platform="twitter",
            platform_id=twitter_user.id,
            proof=state,  # Use OAuth state as proof
        )

        if not link_result.success:
            logger.warning(
                "Failed to create on-chain bridge link",
                error=link_result.error,
            )
            # Continue anyway - the link is still valid locally

        # Clean up pending state
        await session.delete(pending)
        await session.commit()

        logger.info(
            "Twitter link completed",
            twitter_username=twitter_user.username,
            twitter_user_id=twitter_user.id,
            botcash_address=botcash_address,
        )

        return identity

    async def unlink(
        self,
        session: AsyncSession,
        botcash_address: str,
    ) -> bool:
        """Unlink a Twitter account from Botcash address.

        Args:
            session: Database session
            botcash_address: Botcash address to unlink

        Returns:
            True if unlinked, False if no link found
        """
        identity = await self.get_identity_by_address(session, botcash_address)
        if not identity or identity.status != LinkStatus.ACTIVE:
            return False

        # Revoke Twitter token
        token = await self.get_token(session, identity.twitter_user_id)
        if token:
            try:
                await self.twitter.revoke_token(token.access_token)
            except Exception as e:
                logger.warning("Failed to revoke Twitter token", error=str(e))
            await session.delete(token)

        identity.status = LinkStatus.UNLINKED
        identity.unlinked_at = datetime.now(timezone.utc)
        await session.commit()

        logger.info(
            "Twitter link removed",
            twitter_username=identity.twitter_username,
            botcash_address=botcash_address,
        )

        return True

    # === Identity Queries ===

    async def get_identity_by_address(
        self,
        session: AsyncSession,
        botcash_address: str,
    ) -> LinkedIdentity | None:
        """Get identity by Botcash address.

        Args:
            session: Database session
            botcash_address: Botcash address

        Returns:
            LinkedIdentity if found, None otherwise
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.botcash_address == botcash_address,
            )
        )
        return result.scalar_one_or_none()

    async def get_identity_by_twitter_id(
        self,
        session: AsyncSession,
        twitter_user_id: str,
    ) -> LinkedIdentity | None:
        """Get identity by Twitter user ID.

        Args:
            session: Database session
            twitter_user_id: Twitter user ID

        Returns:
            LinkedIdentity if found, None otherwise
        """
        result = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.twitter_user_id == twitter_user_id,
            )
        )
        return result.scalar_one_or_none()

    async def get_active_identities(
        self,
        session: AsyncSession,
        limit: int = 100,
        offset: int = 0,
    ) -> list[LinkedIdentity]:
        """Get all active identity links.

        Args:
            session: Database session
            limit: Max results
            offset: Pagination offset

        Returns:
            List of active LinkedIdentity records
        """
        result = await session.execute(
            select(LinkedIdentity)
            .where(LinkedIdentity.status == LinkStatus.ACTIVE)
            .order_by(LinkedIdentity.linked_at.desc())
            .limit(limit)
            .offset(offset)
        )
        return list(result.scalars().all())

    # === Token Management ===

    async def get_token(
        self,
        session: AsyncSession,
        twitter_user_id: str,
    ) -> OAuthToken | None:
        """Get OAuth token for a Twitter user.

        Args:
            session: Database session
            twitter_user_id: Twitter user ID

        Returns:
            OAuthToken if found, None otherwise
        """
        result = await session.execute(
            select(OAuthToken).where(OAuthToken.twitter_user_id == twitter_user_id)
        )
        return result.scalar_one_or_none()

    async def get_valid_access_token(
        self,
        session: AsyncSession,
        twitter_user_id: str,
    ) -> str | None:
        """Get a valid access token, refreshing if needed.

        Args:
            session: Database session
            twitter_user_id: Twitter user ID

        Returns:
            Valid access token or None if unavailable
        """
        token = await self.get_token(session, twitter_user_id)
        if not token:
            return None

        # Check if token is expired
        if token.expires_at:
            expires_at = token.expires_at
            if expires_at.tzinfo is None:
                expires_at = expires_at.replace(tzinfo=timezone.utc)
            # Refresh if expiring within 5 minutes
            if datetime.now(timezone.utc) > expires_at - timedelta(minutes=5):
                if token.refresh_token:
                    try:
                        new_token = await self.twitter.refresh_access_token(
                            token.refresh_token
                        )
                        token.access_token = new_token.access_token
                        token.refresh_token = new_token.refresh_token or token.refresh_token
                        token.expires_at = (
                            datetime.now(timezone.utc) + timedelta(seconds=new_token.expires_in)
                            if new_token.expires_in else None
                        )
                        await session.commit()
                        logger.info(
                            "Refreshed Twitter token",
                            twitter_user_id=twitter_user_id,
                        )
                    except Exception as e:
                        logger.error("Failed to refresh token", error=str(e))
                        # Mark identity as expired
                        identity = await self.get_identity_by_twitter_id(
                            session, twitter_user_id
                        )
                        if identity:
                            identity.status = LinkStatus.EXPIRED
                            await session.commit()
                        return None
                else:
                    # No refresh token, mark as expired
                    identity = await self.get_identity_by_twitter_id(
                        session, twitter_user_id
                    )
                    if identity:
                        identity.status = LinkStatus.EXPIRED
                        await session.commit()
                    return None

        return token.access_token

    # === Privacy Settings ===

    async def set_privacy_mode(
        self,
        session: AsyncSession,
        botcash_address: str,
        mode: PrivacyMode,
    ) -> bool:
        """Set user's privacy mode.

        Args:
            session: Database session
            botcash_address: Botcash address
            mode: Privacy mode to set

        Returns:
            True if updated, False if no link found
        """
        identity = await self.get_identity_by_address(session, botcash_address)
        if not identity or identity.status != LinkStatus.ACTIVE:
            return False

        identity.privacy_mode = mode
        await session.commit()

        logger.info(
            "Updated privacy mode",
            botcash_address=botcash_address,
            privacy_mode=mode.value,
        )

        return True

    async def get_status(
        self,
        session: AsyncSession,
        botcash_address: str,
    ) -> dict | None:
        """Get link status for a Botcash address.

        Args:
            session: Database session
            botcash_address: Botcash address

        Returns:
            Status dictionary or None if not linked
        """
        identity = await self.get_identity_by_address(session, botcash_address)
        if not identity:
            return None

        return {
            "botcash_address": identity.botcash_address,
            "twitter_user_id": identity.twitter_user_id,
            "twitter_username": identity.twitter_username,
            "twitter_display_name": identity.twitter_display_name,
            "status": identity.status.value,
            "privacy_mode": identity.privacy_mode.value,
            "linked_at": identity.linked_at.isoformat() if identity.linked_at else None,
        }
