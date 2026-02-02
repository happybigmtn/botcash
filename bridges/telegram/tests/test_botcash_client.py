"""Tests for Botcash RPC client."""

import pytest

from botcash_telegram.botcash_client import BotcashClient, Balance


class TestBotcashClient:
    """Tests for BotcashClient."""

    def test_generate_challenge_length(self):
        """Test that generated challenges are 64 hex chars (32 bytes)."""
        client = BotcashClient(rpc_url="http://localhost:8532")
        challenge = client.generate_challenge()

        assert len(challenge) == 64
        assert all(c in "0123456789abcdef" for c in challenge)

    def test_generate_challenge_uniqueness(self):
        """Test that challenges are unique."""
        client = BotcashClient(rpc_url="http://localhost:8532")

        challenges = [client.generate_challenge() for _ in range(100)]
        assert len(set(challenges)) == 100  # All unique

    def test_compute_challenge_hash(self):
        """Test challenge hash computation."""
        client = BotcashClient(rpc_url="http://localhost:8532")

        hash1 = client.compute_challenge_hash("challenge1", 12345)
        hash2 = client.compute_challenge_hash("challenge1", 12345)
        hash3 = client.compute_challenge_hash("challenge1", 67890)

        # Same inputs -> same hash
        assert hash1 == hash2
        # Different user -> different hash
        assert hash1 != hash3
        # Hash is 64 hex chars
        assert len(hash1) == 64


class TestBalance:
    """Tests for Balance dataclass."""

    def test_balance_conversion(self):
        """Test zatoshi to BCASH conversion."""
        balance = Balance(
            address="bs1test",
            confirmed=312_500_000,  # 3.125 BCASH
            pending=100_000_000,    # 1 BCASH
        )

        assert balance.confirmed_bcash == 3.125
        assert balance.total_bcash == 4.125

    def test_balance_zero(self):
        """Test zero balance handling."""
        balance = Balance(
            address="bs1test",
            confirmed=0,
            pending=0,
        )

        assert balance.confirmed_bcash == 0.0
        assert balance.total_bcash == 0.0

    def test_balance_small_amounts(self):
        """Test small amount precision."""
        balance = Balance(
            address="bs1test",
            confirmed=1,  # 1 zatoshi
            pending=0,
        )

        assert balance.confirmed_bcash == 0.00000001
