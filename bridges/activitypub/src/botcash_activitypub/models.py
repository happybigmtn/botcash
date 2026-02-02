"""Database models for Botcash ActivityPub Bridge."""

from datetime import datetime, timezone
from enum import Enum
from typing import Optional

from sqlalchemy import (
    BigInteger,
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
    PENDING = "pending"      # Challenge issued, awaiting verification
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
    """Links an ActivityPub actor to a Botcash address.

    Actor format: @{local_part}@{domain}
    Example: @bs1abc123@botcash.social

    The local_part is derived from the Botcash address to create
    a unique ActivityPub actor ID.
    """
    __tablename__ = "linked_identities"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)

    # ActivityPub actor identification
    # Actor ID is the full URL: https://botcash.social/users/bs1abc123
    actor_id: Mapped[str] = mapped_column(String(512), unique=True, nullable=False, index=True)
    # Local part of the actor handle (e.g., "bs1abc123" from @bs1abc123@botcash.social)
    actor_local_part: Mapped[str] = mapped_column(String(128), unique=True, nullable=False, index=True)
    # Preferred username for display
    actor_preferred_username: Mapped[Optional[str]] = mapped_column(String(128), nullable=True)

    # Botcash address this actor represents
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

    # RSA key pair for HTTP signatures (actor's key)
    public_key_pem: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    private_key_pem: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

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
    followers: Mapped[list["Follower"]] = relationship(
        back_populates="identity", cascade="all, delete-orphan", foreign_keys="Follower.identity_id"
    )
    following: Mapped[list["Following"]] = relationship(
        back_populates="identity", cascade="all, delete-orphan", foreign_keys="Following.identity_id"
    )

    __table_args__ = (
        Index("ix_linked_identities_status", "status"),
        UniqueConstraint("botcash_address", name="uq_botcash_address"),
    )


class RemoteActor(Base):
    """Cached information about remote ActivityPub actors (Mastodon users, etc.)."""
    __tablename__ = "remote_actors"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    # Full actor ID URL (e.g., https://mastodon.social/users/alice)
    actor_id: Mapped[str] = mapped_column(String(512), unique=True, nullable=False, index=True)
    # Instance domain (e.g., mastodon.social)
    instance_domain: Mapped[str] = mapped_column(String(256), nullable=False, index=True)
    # Actor handle (e.g., @alice@mastodon.social)
    handle: Mapped[str] = mapped_column(String(256), nullable=False)
    # Preferred username
    preferred_username: Mapped[str] = mapped_column(String(128), nullable=False)
    # Display name
    display_name: Mapped[Optional[str]] = mapped_column(String(256), nullable=True)
    # Profile summary/bio
    summary: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    # Avatar URL
    avatar_url: Mapped[Optional[str]] = mapped_column(String(512), nullable=True)

    # Endpoints
    inbox_url: Mapped[str] = mapped_column(String(512), nullable=False)
    outbox_url: Mapped[Optional[str]] = mapped_column(String(512), nullable=True)
    shared_inbox_url: Mapped[Optional[str]] = mapped_column(String(512), nullable=True)

    # Public key for signature verification
    public_key_id: Mapped[str] = mapped_column(String(512), nullable=False)
    public_key_pem: Mapped[str] = mapped_column(Text, nullable=False)

    # Linked Botcash address (if this remote actor has linked via bridge)
    linked_botcash_address: Mapped[Optional[str]] = mapped_column(String(128), nullable=True)

    # Cache management
    fetched_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc),
        onupdate=lambda: datetime.now(timezone.utc), nullable=False
    )

    __table_args__ = (
        Index("ix_remote_actors_instance", "instance_domain"),
        Index("ix_remote_actors_handle", "handle"),
    )


class Follower(Base):
    """Tracks followers of Botcash actors (remote actors following local actors)."""
    __tablename__ = "followers"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    # Local Botcash actor being followed
    identity_id: Mapped[int] = mapped_column(Integer, ForeignKey("linked_identities.id"), nullable=False)
    # Remote actor doing the following
    remote_actor_id: Mapped[int] = mapped_column(Integer, ForeignKey("remote_actors.id"), nullable=False)

    # Follow activity ID for Accept/Reject
    follow_activity_id: Mapped[str] = mapped_column(String(512), nullable=False, unique=True)

    # Status: pending (awaiting accept), accepted, rejected
    status: Mapped[str] = mapped_column(String(20), default="pending", nullable=False)

    # Timestamps
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )
    accepted_at: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)

    # Relationships
    identity: Mapped[LinkedIdentity] = relationship(back_populates="followers", foreign_keys=[identity_id])
    remote_actor: Mapped[RemoteActor] = relationship()

    __table_args__ = (
        Index("ix_followers_identity", "identity_id"),
        UniqueConstraint("identity_id", "remote_actor_id", name="uq_follower"),
    )


class Following(Base):
    """Tracks who Botcash actors follow (local actors following remote actors)."""
    __tablename__ = "following"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    # Local Botcash actor doing the following
    identity_id: Mapped[int] = mapped_column(Integer, ForeignKey("linked_identities.id"), nullable=False)
    # Remote actor being followed
    remote_actor_id: Mapped[int] = mapped_column(Integer, ForeignKey("remote_actors.id"), nullable=False)

    # Status: pending, accepted, rejected
    status: Mapped[str] = mapped_column(String(20), default="pending", nullable=False)

    # Timestamps
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )
    accepted_at: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)

    # Relationships
    identity: Mapped[LinkedIdentity] = relationship(back_populates="following", foreign_keys=[identity_id])
    remote_actor: Mapped[RemoteActor] = relationship()

    __table_args__ = (
        Index("ix_following_identity", "identity_id"),
        UniqueConstraint("identity_id", "remote_actor_id", name="uq_following"),
    )


class RelayedMessage(Base):
    """Record of a relayed message (either direction)."""
    __tablename__ = "relayed_messages"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    identity_id: Mapped[int] = mapped_column(Integer, ForeignKey("linked_identities.id"), nullable=False)

    # Direction: "ap_to_bc" (ActivityPub to Botcash) or "bc_to_ap" (Botcash to ActivityPub)
    direction: Mapped[str] = mapped_column(String(12), nullable=False)

    # Source identifiers
    ap_activity_id: Mapped[Optional[str]] = mapped_column(String(512), nullable=True, index=True)
    ap_object_id: Mapped[Optional[str]] = mapped_column(String(512), nullable=True, index=True)
    botcash_tx_id: Mapped[Optional[str]] = mapped_column(String(64), nullable=True, index=True)

    # Message type (post, reply, like, follow, etc.)
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
        UniqueConstraint("ap_activity_id", name="uq_ap_activity"),
        Index("ix_relayed_direction_created", "direction", "created_at"),
    )


class StoredActivity(Base):
    """Cached ActivityPub activity for delivery retry and audit."""
    __tablename__ = "stored_activities"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    activity_id: Mapped[str] = mapped_column(String(512), unique=True, nullable=False, index=True)
    activity_type: Mapped[str] = mapped_column(String(32), nullable=False, index=True)
    actor_id: Mapped[str] = mapped_column(String(512), nullable=False, index=True)

    # Full activity JSON
    activity_json: Mapped[str] = mapped_column(Text, nullable=False)

    # Object ID if activity wraps an object
    object_id: Mapped[Optional[str]] = mapped_column(String(512), nullable=True, index=True)

    # Delivery status
    delivered: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    delivery_attempts: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    last_delivery_attempt: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)
    delivery_error: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    # Origin
    from_botcash: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    botcash_tx_id: Mapped[Optional[str]] = mapped_column(String(64), nullable=True)

    # Timestamps
    received_at: Mapped[datetime] = mapped_column(
        DateTime, default=lambda: datetime.now(timezone.utc), nullable=False
    )

    __table_args__ = (
        Index("ix_stored_activities_type", "activity_type"),
        Index("ix_stored_activities_delivered", "delivered"),
    )


class RateLimitEntry(Base):
    """Rate limiting tracker by instance domain."""
    __tablename__ = "rate_limits"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    instance_domain: Mapped[str] = mapped_column(String(256), nullable=False, index=True)
    window_start: Mapped[datetime] = mapped_column(DateTime, nullable=False)
    request_count: Mapped[int] = mapped_column(Integer, default=1, nullable=False)

    __table_args__ = (
        UniqueConstraint("instance_domain", "window_start", name="uq_domain_window"),
    )


class SponsoredTransaction(Base):
    """Track sponsored transactions for daily limits."""
    __tablename__ = "sponsored_transactions"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    actor_id: Mapped[str] = mapped_column(String(512), nullable=False, index=True)
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
