"""Configuration for Botcash Nostr Bridge."""

from enum import Enum
from typing import Any

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class PrivacyMode(str, Enum):
    """Privacy modes for bridge operation.

    - FULL_MIRROR: All Nostr events -> Botcash, all Botcash posts -> Nostr
    - SELECTIVE: Only linked posts are relayed
    - READ_ONLY: No Nostr -> Botcash, all Botcash posts -> Nostr
    - PRIVATE: DMs only in both directions
    """
    FULL_MIRROR = "full_mirror"
    SELECTIVE = "selective"
    READ_ONLY = "read_only"
    PRIVATE = "private"


class BotcashNodeConfig(BaseSettings):
    """Botcash node connection settings."""

    model_config = SettingsConfigDict(env_prefix="BOTCASH_")

    rpc_url: str = Field(
        default="http://localhost:8532",
        description="Botcash node JSON-RPC URL"
    )
    rpc_user: str = Field(
        default="",
        description="RPC username (if authentication enabled)"
    )
    rpc_password: str = Field(
        default="",
        description="RPC password (if authentication enabled)"
    )
    indexer_url: str = Field(
        default="http://localhost:9067",
        description="Botcash indexer gRPC URL for watching new posts"
    )
    bridge_address: str = Field(
        default="",
        description="Botcash address used by the bridge for sponsored transactions"
    )


class NostrConfig(BaseSettings):
    """Nostr relay settings."""

    model_config = SettingsConfigDict(env_prefix="NOSTR_")

    relay_host: str = Field(
        default="0.0.0.0",
        description="Host address for WebSocket relay"
    )
    relay_port: int = Field(
        default=8080,
        ge=1,
        le=65535,
        description="Port for WebSocket relay"
    )
    relay_url: str = Field(
        default="wss://nostr.botcash.network",
        description="Public URL of this relay for NIP-05 verification"
    )
    private_key: str = Field(
        default="",
        description="Relay's Nostr private key (nsec or hex). Used for signing relay announcements."
    )
    allowed_kinds: list[int] = Field(
        default_factory=lambda: [0, 1, 3, 4, 7, 9734, 9735],
        description="Allowed Nostr event kinds (0=metadata, 1=note, 3=contacts, 4=dm, 7=reaction, 9734/9735=zaps)"
    )
    max_message_size: int = Field(
        default=65536,
        ge=1024,
        le=1048576,
        description="Maximum WebSocket message size in bytes"
    )
    rate_limit_events_per_minute: int = Field(
        default=30,
        ge=1,
        le=300,
        description="Max events per user per minute"
    )
    upstream_relays: list[str] = Field(
        default_factory=list,
        description="Upstream Nostr relays to connect to for event sync"
    )

    @field_validator("private_key")
    @classmethod
    def validate_private_key(cls, v: str) -> str:
        """Validate private key format."""
        if v and not (v.startswith("nsec") or len(v) == 64):
            raise ValueError("Invalid private key format. Expected: nsec... or 64-char hex")
        return v


class FeeConfig(BaseSettings):
    """Fee and sponsorship settings."""

    model_config = SettingsConfigDict(env_prefix="FEE_")

    sponsor_new_users: bool = Field(
        default=True,
        description="Sponsor transaction fees for new users"
    )
    max_sponsored_per_day: int = Field(
        default=100,
        ge=0,
        description="Maximum sponsored transactions per day (0 = unlimited)"
    )
    require_link_deposit_bcash: float = Field(
        default=0.0,
        ge=0,
        description="Required BCASH deposit to link account (0 = no deposit)"
    )
    min_balance_for_relay: float = Field(
        default=0.0,
        ge=0,
        description="Minimum BCASH balance required for relay (0 = no minimum)"
    )
    zap_conversion_rate: float = Field(
        default=0.00000001,
        gt=0,
        description="Conversion rate: 1 satoshi = X BCASH (default: 1 sat = 0.00000001 BCASH)"
    )


class DatabaseConfig(BaseSettings):
    """Database settings."""

    model_config = SettingsConfigDict(env_prefix="DB_")

    url: str = Field(
        default="sqlite+aiosqlite:///botcash_nostr_bridge.db",
        description="SQLAlchemy database URL"
    )


class BridgeConfig(BaseSettings):
    """Main bridge configuration combining all settings."""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # Sub-configs
    botcash: BotcashNodeConfig = Field(default_factory=BotcashNodeConfig)
    nostr: NostrConfig = Field(default_factory=NostrConfig)
    fees: FeeConfig = Field(default_factory=FeeConfig)
    database: DatabaseConfig = Field(default_factory=DatabaseConfig)

    # Bridge behavior
    default_privacy_mode: PrivacyMode = Field(
        default=PrivacyMode.SELECTIVE,
        description="Default privacy mode for new users"
    )
    log_level: str = Field(
        default="INFO",
        description="Logging level"
    )

    @classmethod
    def from_yaml(cls, path: str) -> "BridgeConfig":
        """Load configuration from YAML file."""
        import yaml
        with open(path) as f:
            data = yaml.safe_load(f)
        return cls.model_validate(data)


def load_config() -> BridgeConfig:
    """Load configuration from environment and .env file."""
    return BridgeConfig()
