"""Tests for configuration."""

import pytest

from botcash_telegram.config import (
    BridgeConfig,
    BotcashNodeConfig,
    TelegramConfig,
    FeeConfig,
    DatabaseConfig,
    PrivacyMode,
)


class TestPrivacyMode:
    """Tests for PrivacyMode enum."""

    def test_privacy_mode_values(self):
        """Test all privacy mode values exist."""
        assert PrivacyMode.FULL_MIRROR.value == "full_mirror"
        assert PrivacyMode.SELECTIVE.value == "selective"
        assert PrivacyMode.READ_ONLY.value == "read_only"
        assert PrivacyMode.PRIVATE.value == "private"


class TestBotcashNodeConfig:
    """Tests for BotcashNodeConfig."""

    def test_default_values(self):
        """Test default configuration values."""
        config = BotcashNodeConfig()

        assert config.rpc_url == "http://localhost:8532"
        assert config.rpc_user == ""
        assert config.rpc_password == ""
        assert config.indexer_url == "http://localhost:9067"
        assert config.bridge_address == ""


class TestTelegramConfig:
    """Tests for TelegramConfig."""

    def test_valid_bot_token(self):
        """Test valid bot token is accepted."""
        config = TelegramConfig(bot_token="123456789:ABCdefGHIjklMNOpqrsTUVwxyz")

        assert ":" in config.bot_token

    def test_invalid_bot_token_rejected(self):
        """Test invalid bot token is rejected."""
        with pytest.raises(ValueError, match="Invalid bot token"):
            TelegramConfig(bot_token="invalid_token_no_colon")

    def test_empty_bot_token_rejected(self):
        """Test empty bot token is rejected."""
        with pytest.raises(ValueError, match="Invalid bot token"):
            TelegramConfig(bot_token="")

    def test_default_rate_limit(self):
        """Test default rate limit."""
        config = TelegramConfig(bot_token="123:ABC")

        assert config.rate_limit_messages_per_minute == 10

    def test_rate_limit_bounds(self):
        """Test rate limit bounds."""
        # Valid lower bound
        config = TelegramConfig(bot_token="123:ABC", rate_limit_messages_per_minute=1)
        assert config.rate_limit_messages_per_minute == 1

        # Valid upper bound
        config = TelegramConfig(bot_token="123:ABC", rate_limit_messages_per_minute=60)
        assert config.rate_limit_messages_per_minute == 60


class TestFeeConfig:
    """Tests for FeeConfig."""

    def test_default_values(self):
        """Test default fee configuration."""
        config = FeeConfig()

        assert config.sponsor_new_users is True
        assert config.max_sponsored_per_day == 100
        assert config.require_link_deposit_bcash == 0.0
        assert config.min_balance_for_relay == 0.0

    def test_non_negative_values(self):
        """Test non-negative constraints."""
        # Zero is valid
        config = FeeConfig(max_sponsored_per_day=0)
        assert config.max_sponsored_per_day == 0


class TestDatabaseConfig:
    """Tests for DatabaseConfig."""

    def test_default_sqlite(self):
        """Test default SQLite database URL."""
        config = DatabaseConfig()

        assert "sqlite" in config.url
        assert "aiosqlite" in config.url


class TestBridgeConfig:
    """Tests for BridgeConfig."""

    def test_nested_configs(self, monkeypatch):
        """Test nested configuration loading."""
        monkeypatch.setenv("TELEGRAM_BOT_TOKEN", "123:ABC")

        config = BridgeConfig()

        assert config.telegram.bot_token == "123:ABC"
        assert config.default_privacy_mode == PrivacyMode.SELECTIVE
        assert config.log_level == "INFO"

    def test_default_privacy_mode(self, monkeypatch):
        """Test default privacy mode setting."""
        monkeypatch.setenv("TELEGRAM_BOT_TOKEN", "123:ABC")

        config = BridgeConfig(default_privacy_mode=PrivacyMode.READ_ONLY)

        assert config.default_privacy_mode == PrivacyMode.READ_ONLY
