"""Database models for Botcash X/Twitter Bridge."""

from datetime import datetime, timezone
from enum import Enum
from typing import Optional

from sqlalchemy import (
    BigInteger,
    Boolean,
    DateTime,
    Enum as SQLEnum,
    Index,
    Integer,
    String,
    Text,
    UniqueConstraint,
)
from sqlalchemy.ext.asyncio import AsyncAttrs, async_sessionmaker, create_async_engine
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column


class LinkStatus(str, Enum):
    """Status of Twitter identity link."""
    PENDING = "pending"      # OAuth flow initiated
    ACTIVE = "active"        # Successfully linked
    UNLINKED = "unlinked"    # User unlinked
    SUSPENDED = "suspended"  # Admin suspended
    EXPIRED = "expired"      # Token expired, needs refresh


class PrivacyMode(str, Enum):
    """User's cross-posting preference."""
    FULL_MIRROR = "full_mirror"
    SELECTIVE = "selective"
    DISABLED = "disabled"


class Base(AsyncAttrs, DeclarativeBase):
    """Base class for all models."""
    pass


class LinkedIdentity(Base):
    """Links a Twitter account to a Botcash address.

    Due to X API restrictions, this link is:
    - Initiated via OAuth (user authorizes the bridge app)
    - Used for Botcash → Twitter cross-posting only
    - NOT used for Twitter → Botcash verification (identity concerns)
    """
    __tablename__ = "linked_identities"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)

    # Twitter account identification
    # Twitter user IDs are 64-bit integers
    twitter_user_id: Mapped[str] = mapped_column(String(32), unique=True, nullable=False, index=True)
    twitter_username: Mapped[str] = mapped_column(String(64), nullable=False, index=True)
    twitter_display_name: Mapped[Optional[str]] = mapped_column(String(128), nullable=True)

    # Botcash address this Twitter account is linked to
    botcash_address: Mapped[str] = mapped_column(String(128), nullable=False, index=True)

    # Link status
    status: Mapped[LinkStatus] = mapped_column(
        SQLEnum(LinkStatus), default=LinkStatus.PENDING, nullable=False
    )

    # User preferences
    privacy_mode: Mapped[PrivacyMode] = mapped_column(
        SQLEnum(PrivacyMode), default=PrivacyMode.SELECTIVE, nullable=False
    )

    # Timestamps
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc),
        onupdate=lambda: datetime.now(timezone.utc), nullable=False
    )
    linked_at: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)
    unlinked_at: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)

    __table_args__ = (
        Index("ix_linked_identities_status", "status"),
        UniqueConstraint("botcash_address", name="uq_botcash_address"),
    )


class OAuthToken(Base):
    """OAuth 2.0 access/refresh tokens for Twitter API.

    Stored separately for security and to handle token refresh.
    """
    __tablename__ = "oauth_tokens"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    twitter_user_id: Mapped[str] = mapped_column(String(32), unique=True, nullable=False, index=True)

    # OAuth 2.0 tokens
    access_token: Mapped[str] = mapped_column(Text, nullable=False)
    refresh_token: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    token_type: Mapped[str] = mapped_column(String(32), default="Bearer", nullable=False)

    # Token metadata
    scope: Mapped[str] = mapped_column(String(512), nullable=False)
    expires_at: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)

    # Timestamps
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc),
        onupdate=lambda: datetime.now(timezone.utc), nullable=False
    )


class CrossPostRecord(Base):
    """Record of a Botcash post cross-posted to Twitter."""
    __tablename__ = "crosspost_records"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)

    # Identity link
    twitter_user_id: Mapped[str] = mapped_column(String(32), nullable=False, index=True)

    # Source (Botcash)
    botcash_tx_id: Mapped[str] = mapped_column(String(64), nullable=False, unique=True, index=True)

    # Target (Twitter)
    tweet_id: Mapped[Optional[str]] = mapped_column(String(32), nullable=True, index=True)

    # Status
    success: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    error: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    retry_count: Mapped[int] = mapped_column(Integer, default=0, nullable=False)

    # Content (for retry/audit)
    content_hash: Mapped[str] = mapped_column(String(64), nullable=False)
    tweet_content: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    # Timestamps
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )
    posted_at: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)

    __table_args__ = (
        Index("ix_crosspost_user_created", "twitter_user_id", "created_at"),
        Index("ix_crosspost_success", "success"),
    )


class RateLimitEntry(Base):
    """Rate limiting tracker per Twitter user."""
    __tablename__ = "rate_limits"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    twitter_user_id: Mapped[str] = mapped_column(String(32), nullable=False, index=True)
    window_start: Mapped[datetime] = mapped_column(DateTime, nullable=False)
    request_count: Mapped[int] = mapped_column(Integer, default=1, nullable=False)

    __table_args__ = (
        UniqueConstraint("twitter_user_id", "window_start", name="uq_user_window"),
    )


class SponsoredTransaction(Base):
    """Track sponsored transactions for daily limits."""
    __tablename__ = "sponsored_transactions"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    botcash_address: Mapped[str] = mapped_column(String(128), nullable=False, index=True)
    tx_id: Mapped[str] = mapped_column(String(64), nullable=False, unique=True)
    fee_zatoshis: Mapped[int] = mapped_column(Integer, nullable=False)
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )

    __table_args__ = (
        Index("ix_sponsored_date", "created_at"),
    )


class OAuthPendingState(Base):
    """Temporary storage for OAuth state during authorization flow."""
    __tablename__ = "oauth_pending_states"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    state: Mapped[str] = mapped_column(String(64), unique=True, nullable=False, index=True)
    code_verifier: Mapped[str] = mapped_column(String(128), nullable=False)
    botcash_address: Mapped[str] = mapped_column(String(128), nullable=False)
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )
    expires_at: Mapped[datetime] = mapped_column(DateTime, nullable=False)


async def init_db(database_url: str) -> async_sessionmaker:
    """Initialize database and return session maker."""
    engine = create_async_engine(database_url, echo=False)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    return async_sessionmaker(engine, expire_on_commit=False)
