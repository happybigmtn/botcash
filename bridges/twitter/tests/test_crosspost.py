"""Tests for cross-posting service."""

from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from botcash_twitter.crosspost import CrossPostService
from botcash_twitter.identity import IdentityService
from botcash_twitter.models import (
    CrossPostRecord,
    LinkedIdentity,
    LinkStatus,
    OAuthToken,
    PrivacyMode,
    RateLimitEntry,
)
from botcash_twitter.twitter_client import TweetResult


class TestCrossPostServiceInit:
    """Tests for CrossPostService initialization."""

    def test_init_defaults(self, mock_botcash_client, mock_twitter_client, mock_identity_service):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )
        assert service.botcash is mock_botcash_client
        assert service.twitter is mock_twitter_client
        assert service.identity is mock_identity_service
        assert service.max_tweet_length == 280
        assert service.attribution_suffix == "\n\n#Botcash"
        assert service.include_link is True
        assert service.rate_limit_window_minutes == 15
        assert service.max_tweets_per_window == 10

    def test_init_custom_settings(
        self, mock_botcash_client, mock_twitter_client, mock_identity_service
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
            max_tweet_length=240,
            attribution_suffix=" - via Botcash",
            link_base_url="https://example.com/post/",
            include_link=False,
            rate_limit_window_minutes=30,
            max_tweets_per_window=5,
        )
        assert service.max_tweet_length == 240
        assert service.attribution_suffix == " - via Botcash"
        assert service.link_base_url == "https://example.com/post/"
        assert service.include_link is False
        assert service.rate_limit_window_minutes == 30
        assert service.max_tweets_per_window == 5


class TestCrossPostServiceFormatTweet:
    """Tests for tweet formatting."""

    def test_format_tweet_short_content(
        self, mock_botcash_client, mock_twitter_client, mock_identity_service
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )
        result = service.format_tweet("Hello world!", "abc123def456")
        assert "Hello world!" in result
        assert "#Botcash" in result
        assert "bcash.network/post/abc123def456" in result

    def test_format_tweet_no_link(
        self, mock_botcash_client, mock_twitter_client, mock_identity_service
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
            include_link=False,
        )
        result = service.format_tweet("Hello world!", "abc123def456")
        assert "Hello world!" in result
        assert "#Botcash" in result
        assert "bcash.network" not in result

    def test_format_tweet_long_content_truncated(
        self, mock_botcash_client, mock_twitter_client, mock_identity_service
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
            include_link=False,  # No link for simpler length calculation
            attribution_suffix="",  # No suffix for simpler length calculation
        )
        long_content = "A" * 300
        result = service.format_tweet(long_content, "abc123")
        assert len(result) <= 280
        assert "..." in result

    def test_format_tweet_custom_attribution(
        self, mock_botcash_client, mock_twitter_client, mock_identity_service
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
            attribution_suffix=" 🚀",
        )
        result = service.format_tweet("Test post", "abc123")
        assert "🚀" in result


class TestCrossPostServiceCrossPost:
    """Tests for cross_post method."""

    async def test_cross_post_success(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
        sample_oauth_token,
    ):
        # Set to FULL_MIRROR for auto cross-posting
        sample_linked_identity.privacy_mode = PrivacyMode.FULL_MIRROR
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_valid_access_token = AsyncMock(
            return_value="test_access_token"
        )
        mock_twitter_client.post_tweet = AsyncMock(
            return_value=TweetResult(tweet_id="t123", success=True)
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        record = await service.cross_post(
            session=session,
            botcash_tx_id="btx123",
            content="Hello from Botcash!",
            botcash_address=sample_linked_identity.botcash_address,
        )

        assert record.success is True
        assert record.tweet_id == "t123"
        assert record.botcash_tx_id == "btx123"

    async def test_cross_post_no_active_link(
        self, session, mock_botcash_client, mock_twitter_client, mock_identity_service
    ):
        mock_identity_service.get_identity_by_address = AsyncMock(return_value=None)

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        with pytest.raises(ValueError) as exc_info:
            await service.cross_post(
                session=session,
                botcash_tx_id="btx123",
                content="Test",
                botcash_address="nonexistent",
            )
        assert "No active Twitter link" in str(exc_info.value)

    async def test_cross_post_unlinked_identity(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.status = LinkStatus.UNLINKED
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        with pytest.raises(ValueError) as exc_info:
            await service.cross_post(
                session=session,
                botcash_tx_id="btx123",
                content="Test",
                botcash_address=sample_linked_identity.botcash_address,
            )
        assert "No active Twitter link" in str(exc_info.value)

    async def test_cross_post_disabled_mode(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.DISABLED
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        with pytest.raises(ValueError) as exc_info:
            await service.cross_post(
                session=session,
                botcash_tx_id="btx123",
                content="Test",
                botcash_address=sample_linked_identity.botcash_address,
            )
        assert "disabled" in str(exc_info.value).lower()

    async def test_cross_post_selective_without_force(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.SELECTIVE
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        with pytest.raises(ValueError) as exc_info:
            await service.cross_post(
                session=session,
                botcash_tx_id="btx123",
                content="Test",
                botcash_address=sample_linked_identity.botcash_address,
            )
        assert "selective mode" in str(exc_info.value).lower()

    async def test_cross_post_selective_with_force(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
        sample_oauth_token,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.SELECTIVE
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_valid_access_token = AsyncMock(
            return_value="test_token"
        )
        mock_twitter_client.post_tweet = AsyncMock(
            return_value=TweetResult(tweet_id="t123", success=True)
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        record = await service.cross_post(
            session=session,
            botcash_tx_id="btx_selective",
            content="Opted-in post",
            botcash_address=sample_linked_identity.botcash_address,
            force=True,
        )

        assert record.success is True

    async def test_cross_post_expired_token(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.FULL_MIRROR
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_valid_access_token = AsyncMock(return_value=None)

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        record = await service.cross_post(
            session=session,
            botcash_tx_id="btx_expired",
            content="Test",
            botcash_address=sample_linked_identity.botcash_address,
        )

        assert record.success is False
        assert "expired" in record.error.lower()

    async def test_cross_post_twitter_api_failure(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.FULL_MIRROR
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_valid_access_token = AsyncMock(
            return_value="test_token"
        )
        mock_twitter_client.post_tweet = AsyncMock(
            return_value=TweetResult(tweet_id="", success=False, error="API error")
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        record = await service.cross_post(
            session=session,
            botcash_tx_id="btx_apierror",
            content="Test",
            botcash_address=sample_linked_identity.botcash_address,
        )

        assert record.success is False
        assert record.error == "API error"

    async def test_cross_post_returns_existing_successful(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        # Create existing successful record
        existing = CrossPostRecord(
            twitter_user_id="12345678",
            botcash_tx_id="btx_existing",
            tweet_id="t_existing",
            success=True,
            content_hash="abc",
            tweet_content="Test",
        )
        session.add(existing)
        await session.commit()

        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        record = await service.cross_post(
            session=session,
            botcash_tx_id="btx_existing",
            content="Test",
            botcash_address=sample_linked_identity.botcash_address,
        )

        # Should return existing record without posting again
        assert record.id == existing.id
        assert record.tweet_id == "t_existing"
        mock_twitter_client.post_tweet.assert_not_called()


class TestCrossPostServiceRateLimit:
    """Tests for rate limiting."""

    async def test_rate_limit_check_within_limit(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
            max_tweets_per_window=10,
        )

        result = await service._check_rate_limit(session, "user123")
        assert result is True

    async def test_rate_limit_check_exceeded(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        # Add rate limit entries with unique timestamps (use seconds to avoid unique constraint)
        now = datetime.now(timezone.utc)
        for i in range(15):
            entry = RateLimitEntry(
                twitter_user_id="user_limited",
                window_start=now - timedelta(seconds=i * 10),  # Unique timestamps
                request_count=1,
            )
            session.add(entry)
        await session.commit()

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
            max_tweets_per_window=10,
        )

        result = await service._check_rate_limit(session, "user_limited")
        assert result is False

    async def test_rate_limit_increment(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        await service._increment_rate_limit(session, "user_inc")

        from sqlalchemy import select

        result = await session.execute(
            select(RateLimitEntry).where(RateLimitEntry.twitter_user_id == "user_inc")
        )
        entry = result.scalar_one()
        assert entry.request_count == 1

    async def test_rate_limit_increment_existing(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        # Create existing entry for current minute
        now = datetime.now(timezone.utc).replace(second=0, microsecond=0)
        entry = RateLimitEntry(
            twitter_user_id="user_inc2",
            window_start=now,
            request_count=3,
        )
        session.add(entry)
        await session.commit()

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        await service._increment_rate_limit(session, "user_inc2")

        await session.refresh(entry)
        assert entry.request_count == 4

    async def test_cross_post_rate_limited(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.FULL_MIRROR
        # Add enough entries to exceed limit with unique timestamps
        now = datetime.now(timezone.utc)
        for i in range(11):
            entry = RateLimitEntry(
                twitter_user_id=sample_linked_identity.twitter_user_id,
                window_start=now - timedelta(seconds=i * 10),
                request_count=1,
            )
            session.add(entry)
        await session.commit()

        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
            max_tweets_per_window=10,
        )

        record = await service.cross_post(
            session=session,
            botcash_tx_id="btx_ratelimited",
            content="Test",
            botcash_address=sample_linked_identity.botcash_address,
        )

        assert record.success is False
        assert "rate limit" in record.error.lower()
        mock_twitter_client.post_tweet.assert_not_called()


class TestCrossPostServiceBatch:
    """Tests for batch cross-posting."""

    async def test_cross_post_batch_success(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.FULL_MIRROR
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_valid_access_token = AsyncMock(
            return_value="test_token"
        )
        mock_twitter_client.post_tweet = AsyncMock(
            return_value=TweetResult(tweet_id="t_batch", success=True)
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        posts = [
            {
                "txid": "tx1",
                "content": "Post 1",
                "address": sample_linked_identity.botcash_address,
            },
            {
                "txid": "tx2",
                "content": "Post 2",
                "address": sample_linked_identity.botcash_address,
            },
        ]

        results = await service.cross_post_batch(session, posts)

        assert len(results) == 2
        assert all(r.success for r in results)

    async def test_cross_post_batch_skips_errors(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.FULL_MIRROR
        mock_identity_service.get_identity_by_address = AsyncMock(
            side_effect=[None, sample_linked_identity]
        )
        mock_identity_service.get_valid_access_token = AsyncMock(
            return_value="test_token"
        )
        mock_twitter_client.post_tweet = AsyncMock(
            return_value=TweetResult(tweet_id="t123", success=True)
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        posts = [
            {"txid": "tx_bad", "content": "Bad post", "address": "nonexistent"},
            {
                "txid": "tx_good",
                "content": "Good post",
                "address": sample_linked_identity.botcash_address,
            },
        ]

        results = await service.cross_post_batch(session, posts)

        # Only the second post should succeed
        assert len(results) == 1
        assert results[0].botcash_tx_id == "tx_good"

    async def test_cross_post_batch_with_force(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.SELECTIVE
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_valid_access_token = AsyncMock(
            return_value="test_token"
        )
        mock_twitter_client.post_tweet = AsyncMock(
            return_value=TweetResult(tweet_id="t123", success=True)
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        posts = [
            {
                "txid": "tx_force",
                "content": "Forced post",
                "address": sample_linked_identity.botcash_address,
                "force": True,
            },
        ]

        results = await service.cross_post_batch(session, posts)

        assert len(results) == 1
        assert results[0].success is True


class TestCrossPostServiceRecords:
    """Tests for record management."""

    async def test_get_crosspost_record(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        record = CrossPostRecord(
            twitter_user_id="user123",
            botcash_tx_id="btx_find",
            tweet_id="t_find",
            success=True,
            content_hash="hash",
            tweet_content="Test",
        )
        session.add(record)
        await session.commit()

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        found = await service.get_crosspost_record(session, "btx_find")

        assert found is not None
        assert found.tweet_id == "t_find"

    async def test_get_crosspost_record_not_found(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        found = await service.get_crosspost_record(session, "nonexistent")

        assert found is None

    async def test_get_recent_crossposts(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        # Create records
        for i in range(5):
            record = CrossPostRecord(
                twitter_user_id="user_recent",
                botcash_tx_id=f"btx_{i}",
                tweet_id=f"t_{i}",
                success=True,
                content_hash=f"hash_{i}",
                tweet_content=f"Content {i}",
            )
            session.add(record)
        await session.commit()

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        records = await service.get_recent_crossposts(session, "user_recent", limit=3)

        assert len(records) == 3

    async def test_get_recent_crossposts_empty(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        records = await service.get_recent_crossposts(session, "no_records")

        assert records == []


class TestCrossPostServiceProcessNewPosts:
    """Tests for process_new_posts."""

    async def test_process_new_posts_success(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.FULL_MIRROR
        session.add(sample_linked_identity)
        await session.commit()

        mock_botcash_client.get_feed = AsyncMock(
            return_value=[
                {"txid": "new_tx1", "content": "New post 1"},
                {"txid": "new_tx2", "content": "New post 2"},
            ]
        )

        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_valid_access_token = AsyncMock(
            return_value="test_token"
        )
        mock_twitter_client.post_tweet = AsyncMock(
            return_value=TweetResult(tweet_id="t_new", success=True)
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        results = await service.process_new_posts(session)

        assert len(results) == 2

    async def test_process_new_posts_skips_existing(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.FULL_MIRROR
        session.add(sample_linked_identity)

        # Create existing record
        existing = CrossPostRecord(
            twitter_user_id=sample_linked_identity.twitter_user_id,
            botcash_tx_id="existing_tx",
            tweet_id="t_existing",
            success=True,
            content_hash="hash",
            tweet_content="Existing",
        )
        session.add(existing)
        await session.commit()

        mock_botcash_client.get_feed = AsyncMock(
            return_value=[
                {"txid": "existing_tx", "content": "Existing post"},
                {"txid": "new_tx", "content": "New post"},
            ]
        )

        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_valid_access_token = AsyncMock(
            return_value="test_token"
        )
        mock_twitter_client.post_tweet = AsyncMock(
            return_value=TweetResult(tweet_id="t_new", success=True)
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        results = await service.process_new_posts(session)

        # Only new post should be processed
        assert len(results) == 1

    async def test_process_new_posts_skips_selective_mode(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.SELECTIVE
        session.add(sample_linked_identity)
        await session.commit()

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        results = await service.process_new_posts(session)

        # Should not process any (only processes FULL_MIRROR)
        assert len(results) == 0
        mock_botcash_client.get_feed.assert_not_called()


class TestCrossPostServiceRetryFailed:
    """Tests for retry_failed."""

    async def test_retry_failed_success(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
        sample_linked_identity,
    ):
        sample_linked_identity.privacy_mode = PrivacyMode.FULL_MIRROR
        session.add(sample_linked_identity)

        # Create failed record
        failed = CrossPostRecord(
            twitter_user_id=sample_linked_identity.twitter_user_id,
            botcash_tx_id="failed_tx",
            success=False,
            error="Temporary error",
            content_hash="hash",
            tweet_content="Failed post",
            retry_count=0,
        )
        session.add(failed)
        await session.commit()

        mock_botcash_client.get_post_by_txid = AsyncMock(
            return_value={"content": "Failed post"}
        )
        mock_identity_service.get_identity_by_twitter_id = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_identity_by_address = AsyncMock(
            return_value=sample_linked_identity
        )
        mock_identity_service.get_valid_access_token = AsyncMock(
            return_value="test_token"
        )
        mock_twitter_client.post_tweet = AsyncMock(
            return_value=TweetResult(tweet_id="t_retry", success=True)
        )

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        results = await service.retry_failed(session)

        assert len(results) == 1
        assert results[0].success is True

    async def test_retry_failed_max_retries_exceeded(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        # Create record with max retries reached
        failed = CrossPostRecord(
            twitter_user_id="user123",
            botcash_tx_id="max_retry_tx",
            success=False,
            error="Persistent error",
            content_hash="hash",
            tweet_content="Failed post",
            retry_count=3,
        )
        session.add(failed)
        await session.commit()

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        results = await service.retry_failed(session, max_retries=3)

        # Should not retry (already at max)
        assert len(results) == 0


class TestCrossPostServiceCreateRecord:
    """Tests for _create_record helper."""

    async def test_create_record_new(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        record = await service._create_record(
            session=session,
            botcash_tx_id="btx_new",
            twitter_user_id="user123",
            content="Test content",
            tweet_id="t_new",
            success=True,
        )

        assert record.botcash_tx_id == "btx_new"
        assert record.tweet_id == "t_new"
        assert record.success is True
        assert record.content_hash is not None

    async def test_create_record_updates_existing(
        self,
        session,
        mock_botcash_client,
        mock_twitter_client,
        mock_identity_service,
    ):
        # Create existing record
        existing = CrossPostRecord(
            twitter_user_id="user123",
            botcash_tx_id="btx_update",
            success=False,
            error="Initial error",
            content_hash="hash",
            tweet_content="Content",
        )
        session.add(existing)
        await session.commit()
        existing_id = existing.id

        service = CrossPostService(
            botcash_client=mock_botcash_client,
            twitter_client=mock_twitter_client,
            identity_service=mock_identity_service,
        )

        record = await service._create_record(
            session=session,
            botcash_tx_id="btx_update",
            twitter_user_id="user123",
            content="Updated content",
            tweet_id="t_success",
            success=True,
        )

        assert record.id == existing_id
        assert record.success is True
        assert record.tweet_id == "t_success"
