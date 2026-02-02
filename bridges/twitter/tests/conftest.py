"""Pytest configuration and fixtures for X/Twitter bridge tests."""

from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock

import pytest
import pytest_asyncio
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from botcash_twitter.botcash_client import Balance, BotcashClient, PostResult
from botcash_twitter.config import BridgeConfig, PrivacyMode
from botcash_twitter.models import Base, LinkedIdentity, LinkStatus, OAuthToken
from botcash_twitter.identity import IdentityService
from botcash_twitter.twitter_client import (
    OAuthTokenResponse,
    Tweet,
    TweetResult,
    TwitterClient,
    TwitterUser,
)


@pytest.fixture
def config() -> BridgeConfig:
    """Create test configuration."""
    return BridgeConfig(
        botcash={"rpc_url": "http://localhost:8532"},
        twitter={
            "client_id": "test_client_id",
            "client_secret": "test_client_secret",
            "callback_url": "http://localhost:8080/callback",
            "bearer_token": "test_bearer_token",
        },
        database={"url": "sqlite+aiosqlite:///:memory:"},
        server={"host": "127.0.0.1", "port": 8080},
    )


@pytest_asyncio.fixture
async def session_maker():
    """Create in-memory database session maker for tests."""
    engine = create_async_engine("sqlite+aiosqlite:///:memory:", echo=False)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    maker = async_sessionmaker(engine, expire_on_commit=False)
    yield maker

    await engine.dispose()


@pytest_asyncio.fixture
async def session(session_maker) -> AsyncSession:
    """Create database session for tests."""
    async with session_maker() as session:
        yield session


@pytest.fixture
def mock_botcash_client() -> MagicMock:
    """Create mock Botcash client."""
    client = MagicMock(spec=BotcashClient)
    client.validate_address = AsyncMock(return_value=True)
    client.get_balance = AsyncMock(return_value=Balance(
        address="bs1testaddress",
        confirmed=100_000_000,  # 1 BCASH
        pending=0,
    ))
    client.create_post = AsyncMock(return_value=PostResult(
        tx_id="test_tx_id_123",
        success=True,
    ))
    client.create_bridge_link = AsyncMock(return_value=PostResult(
        tx_id="bridge_link_tx_123",
        success=True,
    ))
    client.get_feed = AsyncMock(return_value=[])
    client.get_blockchain_info = AsyncMock(return_value={"chain": "botcash"})
    client.generate_challenge = MagicMock(return_value="test_challenge_" + "a" * 48)
    client.close = AsyncMock()
    return client


@pytest.fixture
def mock_twitter_client() -> MagicMock:
    """Create mock Twitter client."""
    client = MagicMock(spec=TwitterClient)

    # OAuth methods
    client.get_authorization_url = MagicMock(
        return_value="https://twitter.com/i/oauth2/authorize?test=1"
    )
    client.exchange_code_for_token = AsyncMock(return_value=OAuthTokenResponse(
        access_token="test_access_token",
        refresh_token="test_refresh_token",
        token_type="Bearer",
        expires_in=7200,
        scope="tweet.read tweet.write users.read offline.access",
    ))
    client.refresh_access_token = AsyncMock(return_value=OAuthTokenResponse(
        access_token="new_access_token",
        refresh_token="new_refresh_token",
        token_type="Bearer",
        expires_in=7200,
        scope="tweet.read tweet.write users.read offline.access",
    ))
    client.revoke_token = AsyncMock(return_value=True)

    # User methods
    client.get_me = AsyncMock(return_value=TwitterUser(
        id="12345678",
        username="testuser",
        name="Test User",
        profile_image_url="https://pbs.twimg.com/test.jpg",
    ))
    client.get_user_by_id = AsyncMock(return_value=TwitterUser(
        id="12345678",
        username="testuser",
        name="Test User",
    ))

    # Tweet methods
    client.post_tweet = AsyncMock(return_value=TweetResult(
        tweet_id="tweet_123456",
        success=True,
    ))
    client.delete_tweet = AsyncMock(return_value=True)
    client.get_tweet = AsyncMock(return_value=Tweet(
        id="tweet_123456",
        text="Test tweet content",
        author_id="12345678",
    ))

    client.close = AsyncMock()

    return client


@pytest.fixture
def sample_twitter_user() -> TwitterUser:
    """Sample Twitter user."""
    return TwitterUser(
        id="12345678",
        username="testuser",
        name="Test User",
        profile_image_url="https://pbs.twimg.com/test.jpg",
    )


@pytest.fixture
def sample_botcash_address() -> str:
    """Sample Botcash address."""
    return "bs1testaddress1234567890abcdef"


@pytest_asyncio.fixture
async def sample_linked_identity(
    session: AsyncSession,
    sample_botcash_address: str,
) -> LinkedIdentity:
    """Create sample linked identity in database."""
    identity = LinkedIdentity(
        twitter_user_id="12345678",
        twitter_username="testuser",
        twitter_display_name="Test User",
        botcash_address=sample_botcash_address,
        status=LinkStatus.ACTIVE,
        privacy_mode=PrivacyMode.SELECTIVE,
        linked_at=datetime.now(timezone.utc),
    )
    session.add(identity)
    await session.commit()
    return identity


@pytest_asyncio.fixture
async def sample_oauth_token(
    session: AsyncSession,
) -> OAuthToken:
    """Create sample OAuth token in database."""
    token = OAuthToken(
        twitter_user_id="12345678",
        access_token="test_access_token",
        refresh_token="test_refresh_token",
        token_type="Bearer",
        scope="tweet.read tweet.write users.read offline.access",
        expires_at=datetime.now(timezone.utc) + timedelta(hours=2),
    )
    session.add(token)
    await session.commit()
    return token


@pytest.fixture
def sample_post_data() -> dict:
    """Sample Botcash post data."""
    return {
        "txid": "abc123def456",
        "content": "This is a test post from Botcash!",
        "address": "bs1testaddress1234567890abcdef",
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }


@pytest.fixture
def mock_identity_service(mock_botcash_client, mock_twitter_client) -> MagicMock:
    """Create mock identity service."""
    service = MagicMock(spec=IdentityService)
    service.botcash = mock_botcash_client
    service.twitter = mock_twitter_client
    service.get_identity_by_address = AsyncMock(return_value=None)
    service.get_identity_by_twitter_id = AsyncMock(return_value=None)
    service.get_valid_access_token = AsyncMock(return_value="test_access_token")
    service.get_token = AsyncMock(return_value=None)
    service.set_privacy_mode = AsyncMock(return_value=True)
    service.get_status = AsyncMock(return_value=None)
    return service
