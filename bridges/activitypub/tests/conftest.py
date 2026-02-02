"""Pytest configuration and fixtures for ActivityPub bridge tests."""

import pytest
import pytest_asyncio
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from botcash_activitypub.config import BridgeConfig
from botcash_activitypub.models import Base


@pytest.fixture
def config() -> BridgeConfig:
    """Create test configuration."""
    return BridgeConfig(
        botcash={"rpc_url": "http://localhost:8532"},
        activitypub={
            "domain": "test.botcash.social",
            "base_url": "https://test.botcash.social",
            "host": "127.0.0.1",
            "port": 8080,
        },
        database={"url": "sqlite+aiosqlite:///:memory:"},
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
