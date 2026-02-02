"""Tests for Twitter API client."""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from botcash_twitter.twitter_client import (
    OAuthTokenResponse,
    RateLimitError,
    Tweet,
    TweetResult,
    TwitterApiError,
    TwitterClient,
    TwitterUser,
    generate_code_challenge,
    generate_code_verifier,
    generate_state,
    truncate_for_tweet,
)


class TestGenerateCodeVerifier:
    """Tests for generate_code_verifier function."""

    def test_returns_string(self):
        verifier = generate_code_verifier()
        assert isinstance(verifier, str)

    def test_appropriate_length(self):
        verifier = generate_code_verifier()
        # URL-safe base64 encoded 32 bytes = ~43 characters
        assert 40 <= len(verifier) <= 50

    def test_unique_each_call(self):
        v1 = generate_code_verifier()
        v2 = generate_code_verifier()
        assert v1 != v2


class TestGenerateCodeChallenge:
    """Tests for generate_code_challenge function."""

    def test_returns_string(self):
        verifier = "test_verifier"
        challenge = generate_code_challenge(verifier)
        assert isinstance(challenge, str)

    def test_deterministic(self):
        verifier = "test_verifier"
        c1 = generate_code_challenge(verifier)
        c2 = generate_code_challenge(verifier)
        assert c1 == c2

    def test_different_for_different_verifiers(self):
        c1 = generate_code_challenge("verifier1")
        c2 = generate_code_challenge("verifier2")
        assert c1 != c2

    def test_no_padding(self):
        verifier = generate_code_verifier()
        challenge = generate_code_challenge(verifier)
        assert "=" not in challenge


class TestGenerateState:
    """Tests for generate_state function."""

    def test_returns_string(self):
        state = generate_state()
        assert isinstance(state, str)

    def test_hex_format(self):
        state = generate_state()
        # Should be 64 hex characters (32 bytes)
        assert len(state) == 64
        assert all(c in "0123456789abcdef" for c in state)

    def test_unique_each_call(self):
        s1 = generate_state()
        s2 = generate_state()
        assert s1 != s2


class TestTruncateForTweet:
    """Tests for truncate_for_tweet function."""

    def test_short_content_unchanged(self):
        content = "Short content"
        result = truncate_for_tweet(content, max_length=280)
        assert result == content

    def test_with_suffix(self):
        content = "Test content"
        suffix = "\n#Botcash"
        result = truncate_for_tweet(content, suffix=suffix)
        assert result == f"{content}{suffix}"

    def test_with_link(self):
        content = "Test content"
        link = "https://bcash.network/post/123"
        result = truncate_for_tweet(content, link=link)
        assert link in result

    def test_truncates_long_content(self):
        content = "A" * 300  # Longer than max
        result = truncate_for_tweet(content, max_length=280)
        assert len(result) <= 280
        assert result.endswith("...")

    def test_truncates_with_suffix_and_link(self):
        content = "A" * 300
        suffix = "\n#Botcash"
        link = "https://bcash.network/post/123"
        result = truncate_for_tweet(content, max_length=280, suffix=suffix, link=link)
        assert len(result) <= 280
        assert suffix in result

    def test_preserves_content_when_possible(self):
        content = "Test content"
        suffix = "\n#Botcash"
        link = "https://bcash.network/post/123"
        result = truncate_for_tweet(content, suffix=suffix, link=link)
        assert "Test content" in result


class TestTwitterUser:
    """Tests for TwitterUser dataclass."""

    def test_create_user(self):
        user = TwitterUser(
            id="12345678",
            username="testuser",
            name="Test User",
        )
        assert user.id == "12345678"
        assert user.username == "testuser"
        assert user.name == "Test User"

    def test_profile_image_optional(self):
        user = TwitterUser(
            id="12345678",
            username="testuser",
            name="Test User",
        )
        assert user.profile_image_url is None

    def test_with_profile_image(self):
        user = TwitterUser(
            id="12345678",
            username="testuser",
            name="Test User",
            profile_image_url="https://pbs.twimg.com/test.jpg",
        )
        assert user.profile_image_url == "https://pbs.twimg.com/test.jpg"


class TestTweet:
    """Tests for Tweet dataclass."""

    def test_create_tweet(self):
        tweet = Tweet(
            id="tweet123",
            text="Hello world",
            author_id="12345678",
        )
        assert tweet.id == "tweet123"
        assert tweet.text == "Hello world"
        assert tweet.author_id == "12345678"

    def test_optional_fields(self):
        tweet = Tweet(
            id="tweet123",
            text="Hello",
            author_id="12345678",
        )
        assert tweet.created_at is None
        assert tweet.conversation_id is None


class TestTweetResult:
    """Tests for TweetResult dataclass."""

    def test_successful_result(self):
        result = TweetResult(tweet_id="tweet123", success=True)
        assert result.tweet_id == "tweet123"
        assert result.success is True
        assert result.error is None

    def test_failed_result(self):
        result = TweetResult(tweet_id="", success=False, error="Rate limited")
        assert result.tweet_id == ""
        assert result.success is False
        assert result.error == "Rate limited"


class TestOAuthTokenResponse:
    """Tests for OAuthTokenResponse dataclass."""

    def test_create_response(self):
        response = OAuthTokenResponse(
            access_token="token123",
            refresh_token="refresh123",
            token_type="Bearer",
            expires_in=7200,
            scope="tweet.read tweet.write",
        )
        assert response.access_token == "token123"
        assert response.refresh_token == "refresh123"
        assert response.expires_in == 7200


class TestTwitterApiError:
    """Tests for TwitterApiError exception."""

    def test_create_error(self):
        error = TwitterApiError(400, "Bad Request")
        assert error.status_code == 400
        assert error.message == "Bad Request"
        assert "400" in str(error)

    def test_with_detail(self):
        error = TwitterApiError(400, "Bad Request", "Invalid parameter")
        assert error.detail == "Invalid parameter"


class TestRateLimitError:
    """Tests for RateLimitError exception."""

    def test_create_error(self):
        error = RateLimitError()
        assert error.status_code == 429
        assert error.reset_at is None

    def test_with_reset_time(self):
        error = RateLimitError(reset_at=1704067200)
        assert error.reset_at == 1704067200


class TestTwitterClientInit:
    """Tests for TwitterClient initialization."""

    def test_init_with_credentials(self):
        client = TwitterClient(
            client_id="test_id",
            client_secret="test_secret",
            callback_url="http://localhost/callback",
            bearer_token="test_bearer",
        )
        assert client.client_id == "test_id"
        assert client.client_secret == "test_secret"
        assert client.callback_url == "http://localhost/callback"
        assert client.bearer_token == "test_bearer"

    def test_init_minimal(self):
        client = TwitterClient(client_id="test_id")
        assert client.client_id == "test_id"
        assert client.client_secret == ""


class TestTwitterClientAuthorizationUrl:
    """Tests for get_authorization_url method."""

    def test_generates_valid_url(self):
        client = TwitterClient(
            client_id="test_id",
            callback_url="http://localhost/callback",
        )
        url = client.get_authorization_url(
            state="test_state",
            code_challenge="test_challenge",
        )
        assert "twitter.com" in url
        assert "oauth2/authorize" in url
        assert "test_id" in url
        assert "test_state" in url
        assert "test_challenge" in url

    def test_default_scopes(self):
        client = TwitterClient(
            client_id="test_id",
            callback_url="http://localhost/callback",
        )
        url = client.get_authorization_url(
            state="test_state",
            code_challenge="test_challenge",
        )
        assert "tweet.read" in url or "tweet.read" in url.replace("%20", " ")

    def test_custom_scopes(self):
        client = TwitterClient(
            client_id="test_id",
            callback_url="http://localhost/callback",
        )
        url = client.get_authorization_url(
            state="test_state",
            code_challenge="test_challenge",
            scopes=["tweet.read"],
        )
        assert "tweet.read" in url


class TestTwitterClientMocked:
    """Tests for TwitterClient with mocked HTTP."""

    @pytest.fixture
    def client(self):
        return TwitterClient(
            client_id="test_id",
            client_secret="test_secret",
            callback_url="http://localhost/callback",
        )

    async def test_close(self, client):
        # Should not raise even if no session
        await client.close()

    async def test_get_me_parses_response(self, client):
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {
                "id": "12345678",
                "username": "testuser",
                "name": "Test User",
                "profile_image_url": "https://pbs.twimg.com/test.jpg",
            }
        }

        with patch.object(client, "_api_call", return_value=mock_response.json.return_value):
            user = await client.get_me("test_token")
            assert user.id == "12345678"
            assert user.username == "testuser"

    async def test_post_tweet_success(self, client):
        with patch.object(client, "_api_call") as mock_call:
            mock_call.return_value = {
                "data": {"id": "tweet123"}
            }
            result = await client.post_tweet("test_token", "Hello world")
            assert result.success is True
            assert result.tweet_id == "tweet123"

    async def test_post_tweet_rate_limited(self, client):
        with patch.object(client, "_api_call") as mock_call:
            mock_call.side_effect = RateLimitError(reset_at=1704067200)
            result = await client.post_tweet("test_token", "Hello world")
            assert result.success is False
            assert "Rate limited" in result.error

    async def test_post_tweet_api_error(self, client):
        with patch.object(client, "_api_call") as mock_call:
            mock_call.side_effect = TwitterApiError(403, "Forbidden")
            result = await client.post_tweet("test_token", "Hello world")
            assert result.success is False
            assert result.error is not None
