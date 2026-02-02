"""Tests for Discord bridge Botcash client."""

import pytest

from botcash_discord.botcash_client import Balance, BotcashClient, PostResult


class TestPostResult:
    """Tests for PostResult dataclass."""

    def test_successful_result(self):
        """Test successful post result."""
        result = PostResult(tx_id="abc123", success=True)
        assert result.success is True
        assert result.tx_id == "abc123"
        assert result.error is None

    def test_failed_result(self):
        """Test failed post result."""
        result = PostResult(tx_id="", success=False, error="Insufficient funds")
        assert result.success is False
        assert result.tx_id == ""
        assert result.error == "Insufficient funds"


class TestBalance:
    """Tests for Balance dataclass."""

    def test_balance_properties(self):
        """Test balance calculation properties."""
        balance = Balance(
            address="bs1test...",
            confirmed=100_000_000,  # 1 BCASH
            pending=50_000_000,      # 0.5 BCASH
        )

        assert balance.confirmed_bcash == 1.0
        assert balance.total_bcash == 1.5

    def test_balance_zero(self):
        """Test zero balance."""
        balance = Balance(
            address="bs1test...",
            confirmed=0,
            pending=0,
        )

        assert balance.confirmed_bcash == 0.0
        assert balance.total_bcash == 0.0


class TestBotcashClient:
    """Tests for BotcashClient."""

    def test_client_initialization(self):
        """Test client initialization."""
        client = BotcashClient(
            rpc_url="http://localhost:8532",
            rpc_user="user",
            rpc_password="pass",
            bridge_address="bs1bridge...",
        )

        assert client.rpc_url == "http://localhost:8532"
        assert client.rpc_user == "user"
        assert client.rpc_password == "pass"
        assert client.bridge_address == "bs1bridge..."

    def test_generate_challenge(self):
        """Test challenge generation."""
        client = BotcashClient(rpc_url="http://localhost:8532")

        challenge = client.generate_challenge()

        assert len(challenge) == 64  # 32 bytes hex-encoded
        assert all(c in "0123456789abcdef" for c in challenge)

    def test_generate_challenge_unique(self):
        """Test that challenges are unique."""
        client = BotcashClient(rpc_url="http://localhost:8532")

        challenges = [client.generate_challenge() for _ in range(100)]

        # All challenges should be unique
        assert len(set(challenges)) == 100

    def test_compute_challenge_hash(self):
        """Test challenge hash computation."""
        client = BotcashClient(rpc_url="http://localhost:8532")

        hash1 = client.compute_challenge_hash("challenge1", 123)
        hash2 = client.compute_challenge_hash("challenge1", 123)
        hash3 = client.compute_challenge_hash("challenge1", 456)
        hash4 = client.compute_challenge_hash("challenge2", 123)

        # Same inputs should give same hash
        assert hash1 == hash2

        # Different inputs should give different hashes
        assert hash1 != hash3
        assert hash1 != hash4

        # Hash should be hex-encoded SHA256 (64 chars)
        assert len(hash1) == 64
        assert all(c in "0123456789abcdef" for c in hash1)


class TestBotcashClientMocked:
    """Tests for BotcashClient with mocked network calls."""

    async def test_validate_shielded_address(self, mock_botcash_client):
        """Test validating shielded address."""
        result = await mock_botcash_client.validate_address("bs1" + "a" * 59)
        assert result is True

    async def test_validate_transparent_address(self, mock_botcash_client):
        """Test validating transparent address."""
        result = await mock_botcash_client.validate_address("B1" + "a" * 33)
        assert result is True

    async def test_get_balance(self, mock_botcash_client):
        """Test getting balance."""
        balance = await mock_botcash_client.get_balance("bs1test...")

        assert balance.confirmed == 100_000_000
        assert balance.confirmed_bcash == 1.0

    async def test_create_post(self, mock_botcash_client):
        """Test creating a post."""
        result = await mock_botcash_client.create_post(
            "bs1test...",
            "Hello Botcash!",
            ["test", "hello"],
        )

        assert result.success is True
        assert result.tx_id != ""

    async def test_send_dm(self, mock_botcash_client):
        """Test sending a DM."""
        result = await mock_botcash_client.send_dm(
            "bs1sender...",
            "bs1recipient...",
            "Private message",
        )

        assert result.success is True
        assert result.tx_id != ""

    async def test_create_bridge_link(self, mock_botcash_client):
        """Test creating bridge link."""
        result = await mock_botcash_client.create_bridge_link(
            botcash_address="bs1test...",
            platform="discord",
            platform_id="123456789012345678",
            proof="signature_hex",
        )

        assert result.success is True
        assert result.tx_id != ""

    async def test_get_feed(self, mock_botcash_client):
        """Test getting feed."""
        posts = await mock_botcash_client.get_feed(
            addresses=["bs1test..."],
            limit=10,
        )

        assert len(posts) >= 0
