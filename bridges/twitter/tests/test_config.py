"""Tests for configuration module."""

import pytest
from pydantic import ValidationError

from botcash_twitter.config import (
    BotcashNodeConfig,
    BridgeConfig,
    DatabaseConfig,
    FeeConfig,
    PrivacyMode,
    ServerConfig,
    TwitterConfig,
    load_config,
)


class TestPrivacyMode:
    """Tests for PrivacyMode enum."""

    def test_full_mirror_value(self):
        assert PrivacyMode.FULL_MIRROR.value == "full_mirror"

    def test_selective_value(self):
        assert PrivacyMode.SELECTIVE.value == "selective"

    def test_disabled_value(self):
        assert PrivacyMode.DISABLED.value == "disabled"

    def test_from_string(self):
        assert PrivacyMode("full_mirror") == PrivacyMode.FULL_MIRROR
        assert PrivacyMode("selective") == PrivacyMode.SELECTIVE
        assert PrivacyMode("disabled") == PrivacyMode.DISABLED


class TestBotcashNodeConfig:
    """Tests for BotcashNodeConfig."""

    def test_default_values(self):
        config = BotcashNodeConfig()
        assert config.rpc_url == "http://localhost:8532"
        assert config.rpc_user == ""
        assert config.rpc_password == ""
        assert config.indexer_url == "http://localhost:9067"
        assert config.bridge_address == ""

    def test_custom_values(self):
        config = BotcashNodeConfig(
            rpc_url="http://custom:8532",
            rpc_user="user",
            rpc_password="pass",
            bridge_address="bs1testaddress",
        )
        assert config.rpc_url == "http://custom:8532"
        assert config.rpc_user == "user"


class TestTwitterConfig:
    """Tests for TwitterConfig."""

    def test_default_values(self):
        config = TwitterConfig()
        assert config.api_key == ""
        assert config.client_id == ""
        assert config.max_tweet_length == 280
        assert config.tweets_per_user_per_day == 50

    def test_custom_values(self):
        config = TwitterConfig(
            api_key="test_key",
            client_id="test_client",
            max_tweet_length=240,
        )
        assert config.api_key == "test_key"
        assert config.max_tweet_length == 240

    def test_rate_limit_bounds(self):
        config = TwitterConfig(tweets_per_user_per_day=1)
        assert config.tweets_per_user_per_day == 1

        config = TwitterConfig(tweets_per_user_per_day=300)
        assert config.tweets_per_user_per_day == 300

    def test_attribution_suffix(self):
        config = TwitterConfig(attribution_suffix="\n#Custom")
        assert config.attribution_suffix == "\n#Custom"

    def test_include_link_default(self):
        config = TwitterConfig()
        assert config.include_link is True


class TestFeeConfig:
    """Tests for FeeConfig."""

    def test_default_values(self):
        config = FeeConfig()
        assert config.sponsor_new_users is True
        assert config.max_sponsored_per_day == 100
        assert config.require_link_deposit_bcash == 0.0

    def test_custom_values(self):
        config = FeeConfig(
            sponsor_new_users=False,
            max_sponsored_per_day=50,
            require_link_deposit_bcash=0.01,
        )
        assert config.sponsor_new_users is False
        assert config.max_sponsored_per_day == 50
        assert config.require_link_deposit_bcash == 0.01


class TestDatabaseConfig:
    """Tests for DatabaseConfig."""

    def test_default_url(self):
        config = DatabaseConfig()
        assert "sqlite+aiosqlite" in config.url
        assert "botcash_twitter_bridge.db" in config.url

    def test_custom_url(self):
        config = DatabaseConfig(url="postgresql+asyncpg://localhost/test")
        assert "postgresql" in config.url


class TestServerConfig:
    """Tests for ServerConfig."""

    def test_default_values(self):
        config = ServerConfig()
        assert config.host == "0.0.0.0"
        assert config.port == 8080

    def test_custom_port(self):
        config = ServerConfig(port=9000)
        assert config.port == 9000

    def test_port_bounds(self):
        config = ServerConfig(port=1)
        assert config.port == 1

        config = ServerConfig(port=65535)
        assert config.port == 65535


class TestBridgeConfig:
    """Tests for BridgeConfig."""

    def test_default_config(self):
        config = BridgeConfig()
        assert config.default_privacy_mode == PrivacyMode.SELECTIVE
        assert config.log_level == "INFO"
        assert config.poll_interval_seconds == 60

    def test_nested_configs(self):
        config = BridgeConfig()
        assert isinstance(config.botcash, BotcashNodeConfig)
        assert isinstance(config.twitter, TwitterConfig)
        assert isinstance(config.fees, FeeConfig)
        assert isinstance(config.database, DatabaseConfig)
        assert isinstance(config.server, ServerConfig)

    def test_custom_privacy_mode(self):
        config = BridgeConfig(default_privacy_mode=PrivacyMode.FULL_MIRROR)
        assert config.default_privacy_mode == PrivacyMode.FULL_MIRROR

    def test_custom_log_level(self):
        config = BridgeConfig(log_level="DEBUG")
        assert config.log_level == "DEBUG"

    def test_poll_interval_bounds(self):
        config = BridgeConfig(poll_interval_seconds=10)
        assert config.poll_interval_seconds == 10

        config = BridgeConfig(poll_interval_seconds=3600)
        assert config.poll_interval_seconds == 3600

    def test_from_dict(self):
        config = BridgeConfig(
            botcash={"rpc_url": "http://custom:8532"},
            twitter={"client_id": "test_id"},
        )
        assert config.botcash.rpc_url == "http://custom:8532"
        assert config.twitter.client_id == "test_id"


class TestLoadConfig:
    """Tests for load_config function."""

    def test_load_config_returns_bridge_config(self):
        config = load_config()
        assert isinstance(config, BridgeConfig)

    def test_load_config_has_defaults(self):
        config = load_config()
        assert config.botcash.rpc_url == "http://localhost:8532"
