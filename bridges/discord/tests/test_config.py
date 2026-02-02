"""Tests for Discord bridge configuration."""

import pytest
from pydantic import ValidationError

from botcash_discord.config import (
    BotcashNodeConfig,
    BridgeConfig,
    DatabaseConfig,
    DiscordConfig,
    FeeConfig,
    PrivacyMode,
)


class TestPrivacyMode:
    """Tests for PrivacyMode enum."""

    def test_privacy_mode_values(self):
        """Test privacy mode enum values."""
        assert PrivacyMode.FULL_MIRROR.value == "full_mirror"
        assert PrivacyMode.SELECTIVE.value == "selective"
        assert PrivacyMode.READ_ONLY.value == "read_only"
        assert PrivacyMode.PRIVATE.value == "private"

    def test_privacy_mode_from_string(self):
        """Test creating privacy mode from string."""
        assert PrivacyMode("full_mirror") == PrivacyMode.FULL_MIRROR
        assert PrivacyMode("selective") == PrivacyMode.SELECTIVE


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

    def test_custom_values(self):
        """Test custom configuration values."""
        config = BotcashNodeConfig(
            rpc_url="http://custom:8532",
            rpc_user="user",
            rpc_password="pass",
        )
        assert config.rpc_url == "http://custom:8532"
        assert config.rpc_user == "user"
        assert config.rpc_password == "pass"


class TestDiscordConfig:
    """Tests for DiscordConfig."""

    def test_valid_bot_token(self):
        """Test valid bot token."""
        config = DiscordConfig(
            bot_token="MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0.GhIj12.abcdefghijklmnopqrstuvwxyz12345678",
            application_id=123456789012345678,
        )
        assert config.bot_token.startswith("MTIz")

    def test_invalid_bot_token_empty(self):
        """Test that empty bot token raises error."""
        with pytest.raises(ValidationError):
            DiscordConfig(
                bot_token="",
                application_id=123456789012345678,
            )

    def test_invalid_bot_token_short(self):
        """Test that short bot token raises error."""
        with pytest.raises(ValidationError):
            DiscordConfig(
                bot_token="short_token",
                application_id=123456789012345678,
            )

    def test_default_rate_limit(self):
        """Test default rate limit."""
        config = DiscordConfig(
            bot_token="MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0.GhIj12.abcdefghijklmnopqrstuvwxyz12345678",
            application_id=123456789012345678,
        )
        assert config.rate_limit_messages_per_minute == 10

    def test_rate_limit_bounds(self):
        """Test rate limit boundary validation."""
        # Valid minimum
        config = DiscordConfig(
            bot_token="MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0.GhIj12.abcdefghijklmnopqrstuvwxyz12345678",
            application_id=123456789012345678,
            rate_limit_messages_per_minute=1,
        )
        assert config.rate_limit_messages_per_minute == 1

        # Valid maximum
        config = DiscordConfig(
            bot_token="MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0.GhIj12.abcdefghijklmnopqrstuvwxyz12345678",
            application_id=123456789012345678,
            rate_limit_messages_per_minute=60,
        )
        assert config.rate_limit_messages_per_minute == 60

        # Invalid: too low
        with pytest.raises(ValidationError):
            DiscordConfig(
                bot_token="MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0.GhIj12.abcdefghijklmnopqrstuvwxyz12345678",
                application_id=123456789012345678,
                rate_limit_messages_per_minute=0,
            )

        # Invalid: too high
        with pytest.raises(ValidationError):
            DiscordConfig(
                bot_token="MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0.GhIj12.abcdefghijklmnopqrstuvwxyz12345678",
                application_id=123456789012345678,
                rate_limit_messages_per_minute=61,
            )


class TestFeeConfig:
    """Tests for FeeConfig."""

    def test_default_values(self):
        """Test default fee configuration."""
        config = FeeConfig()
        assert config.sponsor_new_users is True
        assert config.max_sponsored_per_day == 100
        assert config.require_link_deposit_bcash == 0.0
        assert config.min_balance_for_relay == 0.0

    def test_negative_values_rejected(self):
        """Test that negative values are rejected."""
        with pytest.raises(ValidationError):
            FeeConfig(max_sponsored_per_day=-1)

        with pytest.raises(ValidationError):
            FeeConfig(require_link_deposit_bcash=-0.1)


class TestDatabaseConfig:
    """Tests for DatabaseConfig."""

    def test_default_database_url(self):
        """Test default database URL."""
        config = DatabaseConfig()
        assert "sqlite+aiosqlite" in config.url
        assert "botcash_discord_bridge.db" in config.url


class TestBridgeConfig:
    """Tests for BridgeConfig."""

    def test_full_config(self, mock_config):
        """Test full configuration."""
        assert mock_config.botcash.rpc_url == "http://localhost:8532"
        assert mock_config.discord.application_id == 123456789012345678
        assert mock_config.fees.sponsor_new_users is True
        assert mock_config.default_privacy_mode == PrivacyMode.SELECTIVE

    def test_default_privacy_mode(self):
        """Test default privacy mode."""
        config = BridgeConfig(
            discord=DiscordConfig(
                bot_token="MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0.GhIj12.abcdefghijklmnopqrstuvwxyz12345678",
                application_id=123456789012345678,
            ),
        )
        assert config.default_privacy_mode == PrivacyMode.SELECTIVE
