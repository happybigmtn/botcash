"""Configuration for Botcash Discord Bridge."""

from enum import Enum
from typing import Any

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class PrivacyMode(str, Enum):
    """Privacy modes for bridge operation.

    - FULL_MIRROR: All Discord messages -> Botcash, all Botcash posts -> Discord
    - SELECTIVE: Only slash commands -> Botcash, only opted-in posts -> Discord
    - READ_ONLY: No Discord -> Botcash, all Botcash posts -> Discord
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


class DiscordConfig(BaseSettings):
    """Discord bot settings."""

    model_config = SettingsConfigDict(env_prefix="DISCORD_")

    bot_token: str = Field(
        description="Discord Bot token from Developer Portal"
    )
    application_id: int = Field(
        description="Discord Application ID for slash commands"
    )
    allowed_guild_ids: list[int] = Field(
        default_factory=list,
        description="List of guild IDs where bridge is allowed (empty = all guilds)"
    )
    allowed_channel_ids: list[int] = Field(
        default_factory=list,
        description="List of channel IDs where bridge is allowed (empty = all channels)"
    )
    admin_user_ids: list[int] = Field(
        default_factory=list,
        description="Discord user IDs with admin privileges"
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
        if not v or len(v) < 50:
            raise ValueError("Invalid bot token format. Expected Discord bot token.")
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
        default="sqlite+aiosqlite:///botcash_discord_bridge.db",
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
    discord: DiscordConfig = Field(default_factory=DiscordConfig)
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

    # Channel bridging configuration
    bridge_channels: dict[int, int] = Field(
        default_factory=dict,
        description="Mapping of Discord channel ID -> Botcash channel ID for bridging"
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
