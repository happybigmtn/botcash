"""Configuration for Botcash ActivityPub/Fediverse Bridge."""

from enum import Enum
from typing import Any

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class PrivacyMode(str, Enum):
    """Privacy modes for bridge operation.

    - FULL_MIRROR: All ActivityPub activities -> Botcash, all Botcash posts -> Fediverse
    - SELECTIVE: Only explicitly shared posts are relayed
    - READ_ONLY: No AP -> Botcash, all Botcash posts -> Fediverse
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


class ActivityPubConfig(BaseSettings):
    """ActivityPub server settings."""

    model_config = SettingsConfigDict(env_prefix="AP_")

    domain: str = Field(
        default="botcash.social",
        description="Domain for ActivityPub actors (e.g., @bs1abc@botcash.social)"
    )
    host: str = Field(
        default="0.0.0.0",
        description="Host address for HTTP server"
    )
    port: int = Field(
        default=8080,
        ge=1,
        le=65535,
        description="Port for HTTP server"
    )
    base_url: str = Field(
        default="https://botcash.social",
        description="Public URL of this server (must be HTTPS for federation)"
    )
    private_key_path: str = Field(
        default="",
        description="Path to RSA private key for HTTP signatures. Generated if not provided."
    )
    max_inbox_queue_size: int = Field(
        default=10000,
        ge=100,
        le=100000,
        description="Maximum size of inbox processing queue"
    )
    rate_limit_requests_per_minute: int = Field(
        default=60,
        ge=1,
        le=300,
        description="Max requests per remote server per minute"
    )
    # Allowed instance domains (empty = allow all)
    allowed_instances: list[str] = Field(
        default_factory=list,
        description="Allowed instance domains for federation (empty = allow all)"
    )
    # Blocked instance domains
    blocked_instances: list[str] = Field(
        default_factory=list,
        description="Blocked instance domains for federation"
    )

    @field_validator("base_url")
    @classmethod
    def validate_base_url(cls, v: str) -> str:
        """Ensure base URL uses HTTPS (required for ActivityPub)."""
        if v and not v.startswith("https://"):
            # Allow http for development
            import warnings
            warnings.warn("ActivityPub base URL should use HTTPS for production")
        return v.rstrip("/")


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
    min_balance_for_federation: float = Field(
        default=0.0,
        ge=0,
        description="Minimum BCASH balance required for federation (0 = no minimum)"
    )


class DatabaseConfig(BaseSettings):
    """Database settings."""

    model_config = SettingsConfigDict(env_prefix="DB_")

    url: str = Field(
        default="sqlite+aiosqlite:///botcash_activitypub_bridge.db",
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
    activitypub: ActivityPubConfig = Field(default_factory=ActivityPubConfig)
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
