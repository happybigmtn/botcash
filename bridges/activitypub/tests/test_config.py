"""Tests for ActivityPub bridge configuration."""

import os
import tempfile
from pathlib import Path

import pytest
import yaml

from botcash_activitypub.config import (
    ActivityPubConfig,
    BotcashNodeConfig,
    BridgeConfig,
    DatabaseConfig,
    FeeConfig,
    PrivacyMode,
    load_config,
)


class TestPrivacyMode:
    """Tests for PrivacyMode enum."""

    def test_privacy_modes(self):
        """Test all privacy modes exist."""
        assert PrivacyMode.FULL_MIRROR.value == "full_mirror"
        assert PrivacyMode.SELECTIVE.value == "selective"
        assert PrivacyMode.READ_ONLY.value == "read_only"
        assert PrivacyMode.PRIVATE.value == "private"


class TestBotcashNodeConfig:
    """Tests for BotcashNodeConfig."""

    def test_defaults(self):
        """Test default values."""
        config = BotcashNodeConfig()
        assert config.rpc_url == "http://localhost:8532"
        assert config.rpc_user == ""
        assert config.rpc_password == ""

    def test_custom_values(self):
        """Test custom configuration."""
        config = BotcashNodeConfig(
            rpc_url="http://localhost:9000",
            rpc_user="admin",
            rpc_password="secret",
        )
        assert config.rpc_url == "http://localhost:9000"
        assert config.rpc_user == "admin"
        assert config.rpc_password == "secret"


class TestActivityPubConfig:
    """Tests for ActivityPubConfig."""

    def test_defaults(self):
        """Test default values."""
        config = ActivityPubConfig()
        assert config.domain == "botcash.social"
        assert config.host == "0.0.0.0"
        assert config.port == 8080
        assert config.base_url == "https://botcash.social"

    def test_base_url_stripped(self):
        """Test base_url trailing slash is stripped."""
        config = ActivityPubConfig(base_url="https://example.com/")
        assert config.base_url == "https://example.com"


class TestFeeConfig:
    """Tests for FeeConfig."""

    def test_defaults(self):
        """Test default fee settings."""
        config = FeeConfig()
        assert config.sponsor_new_users is True
        assert config.max_sponsored_per_day == 100


class TestDatabaseConfig:
    """Tests for DatabaseConfig."""

    def test_defaults(self):
        """Test default database settings."""
        config = DatabaseConfig()
        assert "sqlite+aiosqlite" in config.url


class TestBridgeConfig:
    """Tests for full BridgeConfig."""

    def test_defaults(self):
        """Test default configuration loads."""
        config = BridgeConfig()
        assert config.activitypub.domain == "botcash.social"
        assert config.botcash.rpc_url == "http://localhost:8532"
        assert config.fees.sponsor_new_users is True

    def test_from_yaml(self, tmp_path):
        """Test loading config from YAML file."""
        yaml_content = """
activitypub:
  domain: test.example.com
  port: 9000
botcash:
  rpc_url: http://localhost:8532
fees:
  sponsor_new_users: false
  max_sponsored_per_day: 50
"""
        config_file = tmp_path / "config.yaml"
        config_file.write_text(yaml_content)

        config = BridgeConfig.from_yaml(str(config_file))
        assert config.activitypub.domain == "test.example.com"
        assert config.activitypub.port == 9000
        assert config.fees.sponsor_new_users is False
        assert config.fees.max_sponsored_per_day == 50

    def test_from_yaml_missing_file(self):
        """Test loading from non-existent file raises error."""
        with pytest.raises(FileNotFoundError):
            BridgeConfig.from_yaml("/nonexistent/config.yaml")


class TestLoadConfig:
    """Tests for load_config function."""

    def test_load_default(self):
        """Test loading default configuration."""
        config = load_config()
        assert isinstance(config, BridgeConfig)
