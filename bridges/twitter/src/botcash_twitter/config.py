"""Configuration for Botcash X/Twitter Bridge."""

from enum import Enum

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class PrivacyMode(str, Enum):
    """Privacy modes for cross-posting.

    - FULL_MIRROR: All Botcash posts -> Twitter
    - SELECTIVE: Only explicitly tagged posts
    - DISABLED: No cross-posting
    """
    FULL_MIRROR = "full_mirror"
    SELECTIVE = "selective"
    DISABLED = "disabled"


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


class TwitterConfig(BaseSettings):
    """Twitter API settings."""

    model_config = SettingsConfigDict(env_prefix="TWITTER_")

    # API v2 credentials
    api_key: str = Field(
        default="",
        description="Twitter API Key (Consumer Key)"
    )
    api_secret: str = Field(
        default="",
        description="Twitter API Secret (Consumer Secret)"
    )
    bearer_token: str = Field(
        default="",
        description="Twitter Bearer Token for app-only auth"
    )

    # OAuth 2.0 PKCE credentials (for user authentication)
    client_id: str = Field(
        default="",
        description="Twitter OAuth 2.0 Client ID"
    )
    client_secret: str = Field(
        default="",
        description="Twitter OAuth 2.0 Client Secret"
    )
    callback_url: str = Field(
        default="http://localhost:8080/callback",
        description="OAuth callback URL"
    )

    # Rate limiting
    tweets_per_user_per_day: int = Field(
        default=50,
        ge=1,
        le=300,
        description="Max tweets per user per day"
    )
    rate_limit_window_minutes: int = Field(
        default=15,
        ge=1,
        description="Rate limit window in minutes"
    )

    # Content
    max_tweet_length: int = Field(
        default=280,
        ge=1,
        le=280,
        description="Maximum tweet length"
    )
    attribution_suffix: str = Field(
        default="\n\n#Botcash",
        description="Attribution text appended to tweets"
    )
    include_link: bool = Field(
        default=True,
        description="Include link to Botcash post in tweet"
    )
    link_base_url: str = Field(
        default="https://bcash.network/post/",
        description="Base URL for post links"
    )

    @field_validator("api_key", "api_secret", "bearer_token", "client_id")
    @classmethod
    def warn_if_empty(cls, v: str, info) -> str:
        """Warn if critical credentials are empty."""
        if not v:
            import warnings
            warnings.warn(f"Twitter {info.field_name} is not set - Twitter features disabled")
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


class DatabaseConfig(BaseSettings):
    """Database settings."""

    model_config = SettingsConfigDict(env_prefix="DB_")

    url: str = Field(
        default="sqlite+aiosqlite:///botcash_twitter_bridge.db",
        description="SQLAlchemy database URL"
    )


class ServerConfig(BaseSettings):
    """HTTP server settings."""

    model_config = SettingsConfigDict(env_prefix="SERVER_")

    host: str = Field(
        default="0.0.0.0",
        description="Server host address"
    )
    port: int = Field(
        default=8080,
        ge=1,
        le=65535,
        description="Server port"
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
    twitter: TwitterConfig = Field(default_factory=TwitterConfig)
    fees: FeeConfig = Field(default_factory=FeeConfig)
    database: DatabaseConfig = Field(default_factory=DatabaseConfig)
    server: ServerConfig = Field(default_factory=ServerConfig)

    # Bridge behavior
    default_privacy_mode: PrivacyMode = Field(
        default=PrivacyMode.SELECTIVE,
        description="Default privacy mode for new users"
    )
    log_level: str = Field(
        default="INFO",
        description="Logging level"
    )

    # Polling interval for new Botcash posts (seconds)
    poll_interval_seconds: int = Field(
        default=60,
        ge=10,
        le=3600,
        description="How often to check for new Botcash posts"
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
