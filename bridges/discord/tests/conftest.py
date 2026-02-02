"""Pytest fixtures for Discord bridge tests."""

from datetime import datetime, timezone
from typing import AsyncGenerator
from unittest.mock import AsyncMock, MagicMock

import pytest
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from botcash_discord.botcash_client import Balance, BotcashClient, PostResult
from botcash_discord.config import BridgeConfig, DiscordConfig, BotcashNodeConfig, FeeConfig, DatabaseConfig
from botcash_discord.identity import IdentityService
from botcash_discord.models import Base, LinkedIdentity, LinkStatus, PrivacyMode


@pytest.fixture
def mock_config() -> BridgeConfig:
    """Create a mock configuration."""
    return BridgeConfig(
        botcash=BotcashNodeConfig(
            rpc_url="http://localhost:8532",
            rpc_user="",
            rpc_password="",
            indexer_url="http://localhost:9067",
            bridge_address="bs1test...",
        ),
        discord=DiscordConfig(
            bot_token="MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0.GhIj12.abcdefghijklmnopqrstuvwxyz12345678",
            application_id=123456789012345678,
            allowed_guild_ids=[],
            allowed_channel_ids=[],
            admin_user_ids=[],
            rate_limit_messages_per_minute=10,
        ),
        fees=FeeConfig(
            sponsor_new_users=True,
            max_sponsored_per_day=100,
            require_link_deposit_bcash=0.0,
            min_balance_for_relay=0.0,
        ),
        database=DatabaseConfig(
            url="sqlite+aiosqlite:///:memory:",
        ),
    )


@pytest.fixture
async def db_session() -> AsyncGenerator[async_sessionmaker[AsyncSession], None]:
    """Create an in-memory database session for testing."""
    engine = create_async_engine("sqlite+aiosqlite:///:memory:", echo=False)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    session_maker = async_sessionmaker(engine, expire_on_commit=False)

    yield session_maker

    await engine.dispose()


@pytest.fixture
def mock_botcash_client() -> BotcashClient:
    """Create a mock Botcash client."""
    client = MagicMock(spec=BotcashClient)

    # Default mock returns
    client.validate_address = AsyncMock(return_value=True)
    client.generate_challenge = MagicMock(return_value="a" * 64)
    client.get_balance = AsyncMock(return_value=Balance(
        address="bs1test...",
        confirmed=100_000_000,  # 1 BCASH
        pending=0,
    ))
    client.create_bridge_link = AsyncMock(return_value=PostResult(
        tx_id="abc123" * 10,
        success=True,
    ))
    client.create_post = AsyncMock(return_value=PostResult(
        tx_id="def456" * 10,
        success=True,
    ))
    client.send_dm = AsyncMock(return_value=PostResult(
        tx_id="ghi789" * 10,
        success=True,
    ))
    client.get_feed = AsyncMock(return_value=[
        {"author": "bs1author123...", "content": "Hello world!", "tx_id": "tx123456..."},
    ])

    return client


@pytest.fixture
def mock_identity_service(mock_botcash_client: BotcashClient) -> IdentityService:
    """Create a mock identity service."""
    return IdentityService(mock_botcash_client)


@pytest.fixture
def sample_linked_identity() -> LinkedIdentity:
    """Create a sample linked identity."""
    return LinkedIdentity(
        id=1,
        discord_user_id=123456789012345678,
        discord_username="testuser",
        discord_discriminator="1234",
        botcash_address="bs1" + "a" * 59,
        status=LinkStatus.ACTIVE,
        privacy_mode=PrivacyMode.SELECTIVE,
        created_at=datetime.now(timezone.utc),
        updated_at=datetime.now(timezone.utc),
        linked_at=datetime.now(timezone.utc),
    )
