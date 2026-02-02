"""Test fixtures for Botcash Telegram Bridge."""

import pytest
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from botcash_telegram.models import Base


@pytest.fixture
async def db_session():
    """Create in-memory database session for tests."""
    engine = create_async_engine("sqlite+aiosqlite:///:memory:", echo=False)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    session_maker = async_sessionmaker(engine, expire_on_commit=False)

    async with session_maker() as session:
        yield session

    await engine.dispose()


@pytest.fixture
def mock_botcash_client(mocker):
    """Create mock Botcash client."""
    from botcash_telegram.botcash_client import BotcashClient, PostResult, Balance

    client = mocker.Mock(spec=BotcashClient)

    # Default mock behaviors
    client.validate_address = mocker.AsyncMock(return_value=True)
    client.generate_challenge = mocker.Mock(return_value="a" * 64)
    client.create_bridge_link = mocker.AsyncMock(
        return_value=PostResult(tx_id="b" * 64, success=True)
    )
    client.create_post = mocker.AsyncMock(
        return_value=PostResult(tx_id="c" * 64, success=True)
    )
    client.send_dm = mocker.AsyncMock(
        return_value=PostResult(tx_id="d" * 64, success=True)
    )
    client.get_balance = mocker.AsyncMock(
        return_value=Balance(address="bs1test", confirmed=100_000_000, pending=0)
    )
    client.get_feed = mocker.AsyncMock(return_value=[])

    return client
