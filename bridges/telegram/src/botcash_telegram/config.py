"""Configuration for Botcash Telegram Bridge."""

from enum import Enum
from typing import Any

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class PrivacyMode(str, Enum):
    """Privacy modes for bridge operation.

    - FULL_MIRROR: All Telegram messages -> Botcash, all Botcash posts -> Telegram
    - SELECTIVE: Only /post commands -> Botcash, only opted-in posts -> Telegram
    - READ_ONLY: No Telegram -> Botcash, all Botcash posts -> Telegram
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


class TelegramConfig(BaseSettings):
    """Telegram bot settings."""

    model_config = SettingsConfigDict(env_prefix="TELEGRAM_")

    bot_token: str = Field(
        description="Telegram Bot API token from @BotFather"
    )
    allowed_group_ids: list[int] = Field(
        default_factory=list,
        description="List of group IDs where bridge is allowed (empty = all groups)"
    )
    admin_user_ids: list[int] = Field(
        default_factory=list,
        description="Telegram user IDs with admin privileges"
    )
    rate_limit_messages_per_minute: int = Field(
        default=10,
        ge=1,
        le=60,
        description="Max messages per user per minute"
    )

    @field_validator("bot_token")
    @classmethod
    def validate_bot_token(cls, v: str) -> str:
        """Validate bot token format."""
        if not v or ":" not in v:
            raise ValueError("Invalid bot token format. Expected: 123456789:ABC...")
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


class DatabaseConfig(BaseSettings):
    """Database settings."""

    model_config = SettingsConfigDict(env_prefix="DB_")

    url: str = Field(
        default="sqlite+aiosqlite:///botcash_bridge.db",
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
    telegram: TelegramConfig = Field(default_factory=TelegramConfig)
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
