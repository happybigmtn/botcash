"""Tests for Nostr bridge configuration."""

import pytest
from pydantic import ValidationError

from botcash_nostr.config import (
    BotcashNodeConfig,
    BridgeConfig,
    DatabaseConfig,
    FeeConfig,
    NostrConfig,
    PrivacyMode,
)


class TestBotcashNodeConfig:
    """Tests for BotcashNodeConfig."""

    def test_defaults(self):
        """Test default values."""
        config = BotcashNodeConfig()

        assert config.rpc_url == "http://localhost:8532"
        assert config.rpc_user == ""
        assert config.rpc_password == ""
        assert config.indexer_url == "http://localhost:9067"
        assert config.bridge_address == ""

    def test_custom_values(self):
        """Test custom values."""
        config = BotcashNodeConfig(
            rpc_url="http://custom:8532",
            rpc_user="user",
            rpc_password="pass",
        )

        assert config.rpc_url == "http://custom:8532"
        assert config.rpc_user == "user"
        assert config.rpc_password == "pass"


class TestNostrConfig:
    """Tests for NostrConfig."""

    def test_defaults(self):
        """Test default values."""
        config = NostrConfig()

        assert config.relay_host == "0.0.0.0"
        assert config.relay_port == 8080
        assert config.relay_url == "wss://nostr.botcash.network"
        assert config.private_key == ""
        assert 1 in config.allowed_kinds  # text notes
        assert 4 in config.allowed_kinds  # DMs
        assert config.max_message_size == 65536
        assert config.rate_limit_events_per_minute == 30

    def test_port_validation(self):
        """Test port number validation."""
        # Valid ports
        config = NostrConfig(relay_port=8080)
        assert config.relay_port == 8080

        config = NostrConfig(relay_port=65535)
        assert config.relay_port == 65535

        # Invalid ports
        with pytest.raises(ValidationError):
            NostrConfig(relay_port=0)

        with pytest.raises(ValidationError):
            NostrConfig(relay_port=70000)

    def test_private_key_validation(self):
        """Test private key validation."""
        # Valid hex key
        config = NostrConfig(private_key="a" * 64)
        assert config.private_key == "a" * 64

        # Valid nsec key
        config = NostrConfig(private_key="nsec1test123")
        assert config.private_key == "nsec1test123"

        # Empty is allowed
        config = NostrConfig(private_key="")
        assert config.private_key == ""

        # Invalid format
        with pytest.raises(ValidationError):
            NostrConfig(private_key="invalid_key")


class TestFeeConfig:
    """Tests for FeeConfig."""

    def test_defaults(self):
        """Test default values."""
        config = FeeConfig()

        assert config.sponsor_new_users is True
        assert config.max_sponsored_per_day == 100
        assert config.require_link_deposit_bcash == 0.0
        assert config.min_balance_for_relay == 0.0
        assert config.zap_conversion_rate == 0.00000001

    def test_zap_conversion_rate(self):
        """Test zap conversion rate validation."""
        config = FeeConfig(zap_conversion_rate=0.0000001)
        assert config.zap_conversion_rate == 0.0000001

        # Must be positive
        with pytest.raises(ValidationError):
            FeeConfig(zap_conversion_rate=0)

        with pytest.raises(ValidationError):
            FeeConfig(zap_conversion_rate=-1)


class TestDatabaseConfig:
    """Tests for DatabaseConfig."""

    def test_defaults(self):
        """Test default values."""
        config = DatabaseConfig()
        assert config.url == "sqlite+aiosqlite:///botcash_nostr_bridge.db"

    def test_custom_url(self):
        """Test custom database URL."""
        config = DatabaseConfig(url="postgresql+asyncpg://user:pass@localhost/db")
        assert "postgresql" in config.url


class TestBridgeConfig:
    """Tests for main BridgeConfig."""

    def test_defaults(self):
        """Test default values."""
        config = BridgeConfig()

        assert isinstance(config.botcash, BotcashNodeConfig)
        assert isinstance(config.nostr, NostrConfig)
        assert isinstance(config.fees, FeeConfig)
        assert isinstance(config.database, DatabaseConfig)
        assert config.default_privacy_mode == PrivacyMode.SELECTIVE
        assert config.log_level == "INFO"

    def test_nested_config(self):
        """Test nested configuration."""
        config = BridgeConfig(
            botcash=BotcashNodeConfig(rpc_url="http://custom:8532"),
            nostr=NostrConfig(relay_port=9000),
        )

        assert config.botcash.rpc_url == "http://custom:8532"
        assert config.nostr.relay_port == 9000


class TestPrivacyMode:
    """Tests for PrivacyMode enum."""

    def test_all_modes(self):
        """Test all privacy modes exist."""
        assert PrivacyMode.FULL_MIRROR == "full_mirror"
        assert PrivacyMode.SELECTIVE == "selective"
        assert PrivacyMode.READ_ONLY == "read_only"
        assert PrivacyMode.PRIVATE == "private"

    def test_mode_values(self):
        """Test privacy mode string values."""
        assert PrivacyMode.FULL_MIRROR.value == "full_mirror"
        assert PrivacyMode.SELECTIVE.value == "selective"
        assert PrivacyMode.READ_ONLY.value == "read_only"
        assert PrivacyMode.PRIVATE.value == "private"
