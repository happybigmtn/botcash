"""Cross-posting service for Botcash -> Twitter.

Monitors Botcash posts and cross-posts them to linked Twitter accounts.
"""

import hashlib
from datetime import datetime, timedelta, timezone
from typing import Any

import structlog
from sqlalchemy import func, select
from sqlalchemy.ext.asyncio import AsyncSession

from .botcash_client import BotcashClient
from .identity import IdentityService
from .models import CrossPostRecord, LinkedIdentity, LinkStatus, PrivacyMode, RateLimitEntry
from .twitter_client import TwitterClient, truncate_for_tweet

logger = structlog.get_logger()

# Rate limit: max tweets per user per window
DEFAULT_RATE_LIMIT_WINDOW_MINUTES = 15
DEFAULT_MAX_TWEETS_PER_WINDOW = 10


class CrossPostService:
    """Service for cross-posting Botcash posts to Twitter."""

    def __init__(
        self,
        botcash_client: BotcashClient,
        twitter_client: TwitterClient,
        identity_service: IdentityService,
        max_tweet_length: int = 280,
        attribution_suffix: str = "\n\n#Botcash",
        link_base_url: str = "https://bcash.network/post/",
        include_link: bool = True,
        rate_limit_window_minutes: int = DEFAULT_RATE_LIMIT_WINDOW_MINUTES,
        max_tweets_per_window: int = DEFAULT_MAX_TWEETS_PER_WINDOW,
    ):
        """Initialize cross-post service.

        Args:
            botcash_client: Botcash RPC client
            twitter_client: Twitter API client
            identity_service: Identity linking service
            max_tweet_length: Maximum tweet length
            attribution_suffix: Text to append to tweets
            link_base_url: Base URL for post links
            include_link: Whether to include link to original post
            rate_limit_window_minutes: Rate limit window in minutes
            max_tweets_per_window: Max tweets per user per window
        """
        self.botcash = botcash_client
        self.twitter = twitter_client
        self.identity = identity_service
        self.max_tweet_length = max_tweet_length
        self.attribution_suffix = attribution_suffix
        self.link_base_url = link_base_url
        self.include_link = include_link
        self.rate_limit_window_minutes = rate_limit_window_minutes
        self.max_tweets_per_window = max_tweets_per_window

    async def cross_post(
        self,
        session: AsyncSession,
        botcash_tx_id: str,
        content: str,
        botcash_address: str,
        force: bool = False,
    ) -> CrossPostRecord:
        """Cross-post a Botcash post to Twitter.

        Args:
            session: Database session
            botcash_tx_id: Botcash transaction ID
            content: Post content
            botcash_address: Author's Botcash address
            force: Force post even in SELECTIVE mode (opt-in)

        Returns:
            CrossPostRecord with result

        Raises:
            ValueError: If not linked or cross-posting disabled
        """
        # Check for existing cross-post
        existing = await self.get_crosspost_record(session, botcash_tx_id)
        if existing and existing.success:
            return existing

        # Get identity
        identity = await self.identity.get_identity_by_address(session, botcash_address)
        if not identity or identity.status != LinkStatus.ACTIVE:
            raise ValueError(f"No active Twitter link for {botcash_address}")

        # Check privacy mode
        if identity.privacy_mode == PrivacyMode.DISABLED:
            raise ValueError("Cross-posting is disabled for this account")

        if identity.privacy_mode == PrivacyMode.SELECTIVE and not force:
            raise ValueError(
                "Account is in selective mode. Use force=True for opt-in posts."
            )

        # Check rate limit
        if not await self._check_rate_limit(session, identity.twitter_user_id):
            error = "Rate limit exceeded. Please wait before posting again."
            record = await self._create_record(
                session, botcash_tx_id, identity.twitter_user_id, content, error=error
            )
            return record

        # Get valid access token
        access_token = await self.identity.get_valid_access_token(
            session, identity.twitter_user_id
        )
        if not access_token:
            error = "Twitter authorization expired. Please re-link your account."
            record = await self._create_record(
                session, botcash_tx_id, identity.twitter_user_id, content, error=error
            )
            return record

        # Format tweet
        link = f"{self.link_base_url}{botcash_tx_id[:16]}" if self.include_link else ""
        tweet_text = truncate_for_tweet(
            content=content,
            max_length=self.max_tweet_length,
            suffix=self.attribution_suffix,
            link=link,
        )

        # Post tweet
        result = await self.twitter.post_tweet(
            access_token=access_token,
            text=tweet_text,
        )

        # Record result
        record = await self._create_record(
            session,
            botcash_tx_id,
            identity.twitter_user_id,
            tweet_text,
            tweet_id=result.tweet_id if result.success else None,
            success=result.success,
            error=result.error,
        )

        # Update rate limit counter
        if result.success:
            await self._increment_rate_limit(session, identity.twitter_user_id)

        logger.info(
            "Cross-posted to Twitter",
            botcash_tx_id=botcash_tx_id,
            twitter_user_id=identity.twitter_user_id,
            tweet_id=result.tweet_id,
            success=result.success,
        )

        return record

    async def cross_post_batch(
        self,
        session: AsyncSession,
        posts: list[dict[str, Any]],
    ) -> list[CrossPostRecord]:
        """Cross-post multiple Botcash posts.

        Args:
            session: Database session
            posts: List of post dictionaries with txid, content, address

        Returns:
            List of CrossPostRecord results
        """
        results = []
        for post in posts:
            try:
                record = await self.cross_post(
                    session=session,
                    botcash_tx_id=post["txid"],
                    content=post["content"],
                    botcash_address=post["address"],
                    force=post.get("force", False),
                )
                results.append(record)
            except ValueError as e:
                logger.debug(
                    "Skipping post",
                    txid=post["txid"],
                    reason=str(e),
                )
        return results

    async def process_new_posts(
        self,
        session: AsyncSession,
    ) -> list[CrossPostRecord]:
        """Process new Botcash posts for all active linked accounts.

        This should be called periodically by the bridge service.

        Args:
            session: Database session

        Returns:
            List of CrossPostRecord results
        """
        results = []

        # Get all active identities with FULL_MIRROR mode
        active_identities = await session.execute(
            select(LinkedIdentity).where(
                LinkedIdentity.status == LinkStatus.ACTIVE,
                LinkedIdentity.privacy_mode == PrivacyMode.FULL_MIRROR,
            )
        )

        for identity in active_identities.scalars():
            # Get recent posts for this address
            posts = await self.botcash.get_feed(
                addresses=[identity.botcash_address],
                limit=10,
            )

            for post in posts:
                tx_id = post.get("txid")
                content = post.get("content", "")

                if not tx_id or not content:
                    continue

                # Skip if already cross-posted
                existing = await self.get_crosspost_record(session, tx_id)
                if existing:
                    continue

                # Cross-post
                try:
                    record = await self.cross_post(
                        session=session,
                        botcash_tx_id=tx_id,
                        content=content,
                        botcash_address=identity.botcash_address,
                    )
                    results.append(record)
                except ValueError as e:
                    logger.debug(
                        "Skipping post",
                        txid=tx_id,
                        reason=str(e),
                    )

        return results

    # === Record Management ===

    async def get_crosspost_record(
        self,
        session: AsyncSession,
        botcash_tx_id: str,
    ) -> CrossPostRecord | None:
        """Get cross-post record by Botcash transaction ID.

        Args:
            session: Database session
            botcash_tx_id: Botcash transaction ID

        Returns:
            CrossPostRecord if found, None otherwise
        """
        result = await session.execute(
            select(CrossPostRecord).where(
                CrossPostRecord.botcash_tx_id == botcash_tx_id
            )
        )
        return result.scalar_one_or_none()

    async def get_recent_crossposts(
        self,
        session: AsyncSession,
        twitter_user_id: str,
        limit: int = 20,
    ) -> list[CrossPostRecord]:
        """Get recent cross-posts for a user.

        Args:
            session: Database session
            twitter_user_id: Twitter user ID
            limit: Max results

        Returns:
            List of CrossPostRecord records
        """
        result = await session.execute(
            select(CrossPostRecord)
            .where(CrossPostRecord.twitter_user_id == twitter_user_id)
            .order_by(CrossPostRecord.created_at.desc())
            .limit(limit)
        )
        return list(result.scalars().all())

    async def retry_failed(
        self,
        session: AsyncSession,
        max_retries: int = 3,
    ) -> list[CrossPostRecord]:
        """Retry failed cross-posts.

        Args:
            session: Database session
            max_retries: Maximum retry attempts

        Returns:
            List of retried CrossPostRecord results
        """
        # Get failed records with retry count below max
        result = await session.execute(
            select(CrossPostRecord).where(
                CrossPostRecord.success == False,
                CrossPostRecord.retry_count < max_retries,
            )
        )

        results = []
        for record in result.scalars():
            # Get identity
            identity = await self.identity.get_identity_by_twitter_id(
                session, record.twitter_user_id
            )
            if not identity or identity.status != LinkStatus.ACTIVE:
                continue

            # Get original post content
            post = await self.botcash.get_post_by_txid(record.botcash_tx_id)
            if not post:
                continue

            # Retry
            record.retry_count += 1
            try:
                new_record = await self.cross_post(
                    session=session,
                    botcash_tx_id=record.botcash_tx_id,
                    content=post.get("content", ""),
                    botcash_address=identity.botcash_address,
                )
                results.append(new_record)
            except ValueError:
                pass

        return results

    # === Rate Limiting ===

    async def _check_rate_limit(
        self,
        session: AsyncSession,
        twitter_user_id: str,
    ) -> bool:
        """Check if user is within rate limit.

        Args:
            session: Database session
            twitter_user_id: Twitter user ID

        Returns:
            True if within limit, False if exceeded
        """
        window_start = datetime.now(timezone.utc) - timedelta(
            minutes=self.rate_limit_window_minutes
        )

        result = await session.execute(
            select(func.sum(RateLimitEntry.request_count)).where(
                RateLimitEntry.twitter_user_id == twitter_user_id,
                RateLimitEntry.window_start >= window_start,
            )
        )
        count = result.scalar() or 0

        return count < self.max_tweets_per_window

    async def _increment_rate_limit(
        self,
        session: AsyncSession,
        twitter_user_id: str,
    ) -> None:
        """Increment rate limit counter.

        Args:
            session: Database session
            twitter_user_id: Twitter user ID
        """
        # Use current minute as window start
        now = datetime.now(timezone.utc)
        window_start = now.replace(second=0, microsecond=0)

        # Try to get existing entry for this window
        result = await session.execute(
            select(RateLimitEntry).where(
                RateLimitEntry.twitter_user_id == twitter_user_id,
                RateLimitEntry.window_start == window_start,
            )
        )
        entry = result.scalar_one_or_none()

        if entry:
            entry.request_count += 1
        else:
            entry = RateLimitEntry(
                twitter_user_id=twitter_user_id,
                window_start=window_start,
                request_count=1,
            )
            session.add(entry)

        await session.commit()

    # === Helper Methods ===

    async def _create_record(
        self,
        session: AsyncSession,
        botcash_tx_id: str,
        twitter_user_id: str,
        content: str,
        tweet_id: str | None = None,
        success: bool = False,
        error: str | None = None,
    ) -> CrossPostRecord:
        """Create a cross-post record.

        Args:
            session: Database session
            botcash_tx_id: Botcash transaction ID
            twitter_user_id: Twitter user ID
            content: Tweet content
            tweet_id: Twitter tweet ID (if successful)
            success: Whether posting succeeded
            error: Error message (if failed)

        Returns:
            Created CrossPostRecord
        """
        content_hash = hashlib.sha256(content.encode()).hexdigest()

        # Check for existing record
        existing = await self.get_crosspost_record(session, botcash_tx_id)
        if existing:
            existing.tweet_id = tweet_id
            existing.success = success
            existing.error = error
            existing.tweet_content = content
            if success:
                existing.posted_at = datetime.now(timezone.utc)
            await session.commit()
            return existing

        record = CrossPostRecord(
            twitter_user_id=twitter_user_id,
            botcash_tx_id=botcash_tx_id,
            tweet_id=tweet_id,
            success=success,
            error=error,
            content_hash=content_hash,
            tweet_content=content,
            posted_at=datetime.now(timezone.utc) if success else None,
        )
        session.add(record)
        await session.commit()

        return record

    def format_tweet(self, content: str, tx_id: str) -> str:
        """Format content for tweeting.

        Args:
            content: Original post content
            tx_id: Botcash transaction ID

        Returns:
            Formatted tweet text
        """
        link = f"{self.link_base_url}{tx_id[:16]}" if self.include_link else ""
        return truncate_for_tweet(
            content=content,
            max_length=self.max_tweet_length,
            suffix=self.attribution_suffix,
            link=link,
        )
