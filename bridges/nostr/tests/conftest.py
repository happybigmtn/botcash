"""Pytest configuration and fixtures for Nostr bridge tests."""

import pytest
import pytest_asyncio
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from botcash_nostr.models import Base


@pytest_asyncio.fixture
async def db_session():
    """Create an in-memory database session for testing."""
    engine = create_async_engine("sqlite+aiosqlite:///:memory:", echo=False)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    session_maker = async_sessionmaker(engine, expire_on_commit=False)

    async with session_maker() as session:
        yield session

    await engine.dispose()


@pytest.fixture
def sample_nostr_pubkey() -> str:
    """Sample Nostr public key (hex)."""
    return "a" * 64


@pytest.fixture
def sample_nostr_npub() -> str:
    """Sample Nostr npub."""
    return "npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsn0kdv"


@pytest.fixture
def sample_botcash_address() -> str:
    """Sample Botcash address."""
    return "bs1testtesttesttesttesttesttesttesttesttesttesttesttesttesttes"


@pytest.fixture
def sample_nostr_event() -> dict:
    """Sample Nostr event dictionary."""
    return {
        "id": "b" * 64,
        "pubkey": "a" * 64,
        "created_at": 1704067200,
        "kind": 1,
        "tags": [],
        "content": "Hello from Nostr!",
        "sig": "c" * 128,
    }


@pytest.fixture
def sample_text_note_event() -> dict:
    """Sample Nostr text note (kind 1) event."""
    return {
        "id": "d" * 64,
        "pubkey": "a" * 64,
        "created_at": 1704067200,
        "kind": 1,
        "tags": [
            ["e", "e" * 64],  # reply to
            ["p", "f" * 64],  # mention
        ],
        "content": "This is a reply with a #hashtag",
        "sig": "c" * 128,
    }


@pytest.fixture
def sample_dm_event() -> dict:
    """Sample Nostr encrypted DM (kind 4) event."""
    return {
        "id": "g" * 64,
        "pubkey": "a" * 64,
        "created_at": 1704067200,
        "kind": 4,
        "tags": [["p", "h" * 64]],
        "content": "encrypted_content_here",
        "sig": "c" * 128,
    }


@pytest.fixture
def sample_reaction_event() -> dict:
    """Sample Nostr reaction (kind 7) event."""
    return {
        "id": "i" * 64,
        "pubkey": "a" * 64,
        "created_at": 1704067200,
        "kind": 7,
        "tags": [
            ["e", "j" * 64],
            ["p", "k" * 64],
        ],
        "content": "+",
        "sig": "c" * 128,
    }


@pytest.fixture
def sample_zap_request_event() -> dict:
    """Sample Nostr zap request (kind 9734) event."""
    return {
        "id": "l" * 64,
        "pubkey": "a" * 64,
        "created_at": 1704067200,
        "kind": 9734,
        "tags": [
            ["p", "m" * 64],
            ["amount", "1000000"],  # 1000 sats in millisats
            ["e", "n" * 64],  # target event
        ],
        "content": "Great post!",
        "sig": "c" * 128,
    }


@pytest.fixture
def sample_contacts_event() -> dict:
    """Sample Nostr contacts (kind 3) event."""
    return {
        "id": "o" * 64,
        "pubkey": "a" * 64,
        "created_at": 1704067200,
        "kind": 3,
        "tags": [
            ["p", "p" * 64, "wss://relay.example.com", "alice"],
            ["p", "q" * 64, "wss://relay.example.com", "bob"],
        ],
        "content": "",
        "sig": "c" * 128,
    }


@pytest.fixture
def sample_metadata_event() -> dict:
    """Sample Nostr metadata (kind 0) event."""
    return {
        "id": "r" * 64,
        "pubkey": "a" * 64,
        "created_at": 1704067200,
        "kind": 0,
        "tags": [],
        "content": '{"name":"alice","about":"Hello!","picture":"https://example.com/pic.jpg"}',
        "sig": "c" * 128,
    }
