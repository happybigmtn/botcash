"""Database models for Botcash Telegram Bridge."""

from datetime import datetime, timezone
from enum import Enum
from typing import Optional

from sqlalchemy import (
    Boolean,
    DateTime,
    Enum as SQLEnum,
    ForeignKey,
    Index,
    Integer,
    String,
    Text,
    UniqueConstraint,
)
from sqlalchemy.ext.asyncio import AsyncAttrs, async_sessionmaker, create_async_engine
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column, relationship


class LinkStatus(str, Enum):
    """Status of identity link."""
    PENDING = "pending"      # Challenge issued, awaiting signature
    ACTIVE = "active"        # Successfully linked
    UNLINKED = "unlinked"    # User unlinked
    SUSPENDED = "suspended"  # Admin suspended


class PrivacyMode(str, Enum):
    """User's privacy mode preference."""
    FULL_MIRROR = "full_mirror"
    SELECTIVE = "selective"
    READ_ONLY = "read_only"
    PRIVATE = "private"


class Base(AsyncAttrs, DeclarativeBase):
    """Base class for all models."""
    pass


class LinkedIdentity(Base):
    """Links a Telegram user to a Botcash address."""
    __tablename__ = "linked_identities"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    telegram_user_id: Mapped[int] = mapped_column(Integer, unique=True, nullable=False, index=True)
    telegram_username: Mapped[Optional[str]] = mapped_column(String(64), nullable=True)
    botcash_address: Mapped[str] = mapped_column(String(128), nullable=False, index=True)

    # Link verification
    status: Mapped[LinkStatus] = mapped_column(
        SQLEnum(LinkStatus), default=LinkStatus.PENDING, nullable=False
    )
    challenge: Mapped[Optional[str]] = mapped_column(String(64), nullable=True)
    challenge_expires_at: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)
    link_tx_id: Mapped[Optional[str]] = mapped_column(String(64), nullable=True)

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

    # Relationships
    relayed_messages: Mapped[list["RelayedMessage"]] = relationship(
        back_populates="identity", cascade="all, delete-orphan"
    )

    __table_args__ = (
        Index("ix_linked_identities_status", "status"),
    )


class RelayedMessage(Base):
    """Record of a relayed message (either direction)."""
    __tablename__ = "relayed_messages"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    identity_id: Mapped[int] = mapped_column(Integer, ForeignKey("linked_identities.id"), nullable=False)

    # Direction: "tg_to_bc" or "bc_to_tg"
    direction: Mapped[str] = mapped_column(String(10), nullable=False)

    # Source identifiers
    telegram_message_id: Mapped[Optional[int]] = mapped_column(Integer, nullable=True)
    telegram_chat_id: Mapped[Optional[int]] = mapped_column(Integer, nullable=True)
    botcash_tx_id: Mapped[Optional[str]] = mapped_column(String(64), nullable=True, index=True)

    # Message type (post, dm, follow, etc.)
    message_type: Mapped[str] = mapped_column(String(32), nullable=False)

    # Content hash (for deduplication)
    content_hash: Mapped[str] = mapped_column(String(64), nullable=False, index=True)

    # Fee info
    fee_sponsored: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    fee_amount_zatoshis: Mapped[Optional[int]] = mapped_column(Integer, nullable=True)

    # Timestamps
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )

    # Relationships
    identity: Mapped[LinkedIdentity] = relationship(back_populates="relayed_messages")

    __table_args__ = (
        UniqueConstraint("telegram_message_id", "telegram_chat_id", name="uq_telegram_message"),
        Index("ix_relayed_direction_created", "direction", "created_at"),
    )


class RateLimitEntry(Base):
    """Rate limiting tracker."""
    __tablename__ = "rate_limits"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    telegram_user_id: Mapped[int] = mapped_column(Integer, nullable=False, index=True)
    window_start: Mapped[datetime] = mapped_column(DateTime, nullable=False)
    message_count: Mapped[int] = mapped_column(Integer, default=1, nullable=False)

    __table_args__ = (
        UniqueConstraint("telegram_user_id", "window_start", name="uq_user_window"),
    )


class SponsoredTransaction(Base):
    """Track sponsored transactions for daily limits."""
    __tablename__ = "sponsored_transactions"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    telegram_user_id: Mapped[int] = mapped_column(Integer, nullable=False, index=True)
    tx_id: Mapped[str] = mapped_column(String(64), nullable=False, unique=True)
    fee_zatoshis: Mapped[int] = mapped_column(Integer, nullable=False)
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )

    __table_args__ = (
        Index("ix_sponsored_date", "created_at"),
    )


async def init_db(database_url: str) -> async_sessionmaker:
    """Initialize database and return session maker."""
    engine = create_async_engine(database_url, echo=False)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    return async_sessionmaker(engine, expire_on_commit=False)
