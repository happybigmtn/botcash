"""Twitter API v2 client for Botcash bridge.

Handles OAuth 2.0 PKCE authentication and tweet posting.
"""

import base64
import hashlib
import secrets
from dataclasses import dataclass
from typing import Any
from urllib.parse import urlencode

import httpx
import structlog

logger = structlog.get_logger()

# Twitter API v2 endpoints
TWITTER_API_BASE = "https://api.twitter.com/2"
TWITTER_OAUTH2_AUTHORIZE = "https://twitter.com/i/oauth2/authorize"
TWITTER_OAUTH2_TOKEN = "https://api.twitter.com/2/oauth2/token"
TWITTER_OAUTH2_REVOKE = "https://api.twitter.com/2/oauth2/revoke"


@dataclass
class TwitterUser:
    """Twitter user information."""
    id: str
    username: str
    name: str
    profile_image_url: str | None = None


@dataclass
class Tweet:
    """A tweet/post on Twitter."""
    id: str
    text: str
    author_id: str
    created_at: str | None = None
    conversation_id: str | None = None


@dataclass
class TweetResult:
    """Result of posting a tweet."""
    tweet_id: str
    success: bool
    error: str | None = None


@dataclass
class OAuthTokenResponse:
    """OAuth 2.0 token response."""
    access_token: str
    refresh_token: str | None
    token_type: str
    expires_in: int
    scope: str


class TwitterApiError(Exception):
    """Error from Twitter API."""

    def __init__(self, status_code: int, message: str, detail: str | None = None):
        self.status_code = status_code
        self.message = message
        self.detail = detail
        super().__init__(f"Twitter API Error {status_code}: {message}")


class RateLimitError(TwitterApiError):
    """Rate limit exceeded error."""

    def __init__(self, reset_at: int | None = None):
        self.reset_at = reset_at
        super().__init__(429, "Rate limit exceeded")


def generate_code_verifier() -> str:
    """Generate a PKCE code verifier.

    Returns:
        Random URL-safe string (43-128 characters)
    """
    return secrets.token_urlsafe(32)


def generate_code_challenge(verifier: str) -> str:
    """Generate PKCE code challenge from verifier.

    Args:
        verifier: Code verifier string

    Returns:
        Base64-URL encoded SHA256 hash of verifier
    """
    digest = hashlib.sha256(verifier.encode()).digest()
    return base64.urlsafe_b64encode(digest).decode().rstrip("=")


def generate_state() -> str:
    """Generate random state for OAuth flow.

    Returns:
        Random hex string
    """
    return secrets.token_hex(32)


class TwitterClient:
    """Client for Twitter API v2 with OAuth 2.0 PKCE support."""

    def __init__(
        self,
        client_id: str,
        client_secret: str = "",
        callback_url: str = "",
        bearer_token: str = "",
    ):
        """Initialize Twitter client.

        Args:
            client_id: OAuth 2.0 Client ID
            client_secret: OAuth 2.0 Client Secret
            callback_url: OAuth callback URL
            bearer_token: Bearer token for app-only auth (optional)
        """
        self.client_id = client_id
        self.client_secret = client_secret
        self.callback_url = callback_url
        self.bearer_token = bearer_token
        self._http_client: httpx.AsyncClient | None = None

    async def _get_client(self) -> httpx.AsyncClient:
        """Get or create HTTP client."""
        if self._http_client is None or self._http_client.is_closed:
            self._http_client = httpx.AsyncClient(
                timeout=30.0,
                headers={"User-Agent": "BotcashTwitterBridge/1.0"},
            )
        return self._http_client

    async def close(self) -> None:
        """Close HTTP client."""
        if self._http_client and not self._http_client.is_closed:
            await self._http_client.aclose()

    # === OAuth 2.0 PKCE Flow ===

    def get_authorization_url(
        self,
        state: str,
        code_challenge: str,
        scopes: list[str] | None = None,
    ) -> str:
        """Generate OAuth 2.0 authorization URL.

        Args:
            state: Random state for CSRF protection
            code_challenge: PKCE code challenge
            scopes: Requested OAuth scopes

        Returns:
            Authorization URL to redirect user to
        """
        if scopes is None:
            # Minimum scopes for posting tweets and reading user info
            scopes = ["tweet.read", "tweet.write", "users.read", "offline.access"]

        params = {
            "response_type": "code",
            "client_id": self.client_id,
            "redirect_uri": self.callback_url,
            "scope": " ".join(scopes),
            "state": state,
            "code_challenge": code_challenge,
            "code_challenge_method": "S256",
        }
        return f"{TWITTER_OAUTH2_AUTHORIZE}?{urlencode(params)}"

    async def exchange_code_for_token(
        self,
        code: str,
        code_verifier: str,
    ) -> OAuthTokenResponse:
        """Exchange authorization code for access token.

        Args:
            code: Authorization code from callback
            code_verifier: PKCE code verifier

        Returns:
            OAuth token response

        Raises:
            TwitterApiError: If token exchange fails
        """
        client = await self._get_client()

        data = {
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": self.callback_url,
            "code_verifier": code_verifier,
        }

        # Use client credentials for authentication
        auth = (self.client_id, self.client_secret) if self.client_secret else None
        if not auth:
            # For public clients, include client_id in body
            data["client_id"] = self.client_id

        response = await client.post(
            TWITTER_OAUTH2_TOKEN,
            data=data,
            auth=auth,
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )

        if response.status_code != 200:
            error = response.json()
            raise TwitterApiError(
                response.status_code,
                error.get("error", "Token exchange failed"),
                error.get("error_description"),
            )

        result = response.json()
        return OAuthTokenResponse(
            access_token=result["access_token"],
            refresh_token=result.get("refresh_token"),
            token_type=result.get("token_type", "Bearer"),
            expires_in=result.get("expires_in", 7200),
            scope=result.get("scope", ""),
        )

    async def refresh_access_token(
        self,
        refresh_token: str,
    ) -> OAuthTokenResponse:
        """Refresh an expired access token.

        Args:
            refresh_token: Refresh token

        Returns:
            New OAuth token response

        Raises:
            TwitterApiError: If refresh fails
        """
        client = await self._get_client()

        data = {
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        }

        auth = (self.client_id, self.client_secret) if self.client_secret else None
        if not auth:
            data["client_id"] = self.client_id

        response = await client.post(
            TWITTER_OAUTH2_TOKEN,
            data=data,
            auth=auth,
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )

        if response.status_code != 200:
            error = response.json()
            raise TwitterApiError(
                response.status_code,
                error.get("error", "Token refresh failed"),
                error.get("error_description"),
            )

        result = response.json()
        return OAuthTokenResponse(
            access_token=result["access_token"],
            refresh_token=result.get("refresh_token", refresh_token),
            token_type=result.get("token_type", "Bearer"),
            expires_in=result.get("expires_in", 7200),
            scope=result.get("scope", ""),
        )

    async def revoke_token(self, token: str) -> bool:
        """Revoke an access or refresh token.

        Args:
            token: Token to revoke

        Returns:
            True if revoked successfully
        """
        client = await self._get_client()

        data = {
            "token": token,
            "client_id": self.client_id,
        }

        response = await client.post(
            TWITTER_OAUTH2_REVOKE,
            data=data,
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )

        return response.status_code == 200

    # === API Calls ===

    async def _api_call(
        self,
        method: str,
        endpoint: str,
        access_token: str,
        json_data: dict[str, Any] | None = None,
        params: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """Make authenticated API call.

        Args:
            method: HTTP method
            endpoint: API endpoint (relative to base URL)
            access_token: User access token
            json_data: JSON body (for POST/PUT)
            params: Query parameters

        Returns:
            API response data

        Raises:
            TwitterApiError: On API error
            RateLimitError: On rate limit
        """
        client = await self._get_client()
        url = f"{TWITTER_API_BASE}/{endpoint.lstrip('/')}"

        headers = {
            "Authorization": f"Bearer {access_token}",
            "Content-Type": "application/json",
        }

        response = await client.request(
            method,
            url,
            headers=headers,
            json=json_data,
            params=params,
        )

        # Handle rate limiting
        if response.status_code == 429:
            reset_at = response.headers.get("x-rate-limit-reset")
            raise RateLimitError(int(reset_at) if reset_at else None)

        # Handle errors
        if response.status_code >= 400:
            try:
                error = response.json()
                detail = error.get("detail") or error.get("error_description")
                message = error.get("title") or error.get("error", "Unknown error")
            except Exception:
                message = response.text
                detail = None
            raise TwitterApiError(response.status_code, message, detail)

        return response.json()

    async def get_me(self, access_token: str) -> TwitterUser:
        """Get authenticated user's profile.

        Args:
            access_token: User access token

        Returns:
            TwitterUser object
        """
        result = await self._api_call(
            "GET",
            "/users/me",
            access_token,
            params={"user.fields": "id,username,name,profile_image_url"},
        )

        data = result["data"]
        return TwitterUser(
            id=data["id"],
            username=data["username"],
            name=data["name"],
            profile_image_url=data.get("profile_image_url"),
        )

    async def get_user_by_id(self, user_id: str, access_token: str) -> TwitterUser | None:
        """Get user by ID.

        Args:
            user_id: Twitter user ID
            access_token: Access token

        Returns:
            TwitterUser or None if not found
        """
        try:
            result = await self._api_call(
                "GET",
                f"/users/{user_id}",
                access_token,
                params={"user.fields": "id,username,name,profile_image_url"},
            )
            data = result["data"]
            return TwitterUser(
                id=data["id"],
                username=data["username"],
                name=data["name"],
                profile_image_url=data.get("profile_image_url"),
            )
        except TwitterApiError as e:
            if e.status_code == 404:
                return None
            raise

    async def post_tweet(
        self,
        access_token: str,
        text: str,
        reply_to: str | None = None,
        quote_tweet_id: str | None = None,
    ) -> TweetResult:
        """Post a tweet.

        Args:
            access_token: User access token
            text: Tweet text (max 280 characters)
            reply_to: Tweet ID to reply to (optional)
            quote_tweet_id: Tweet ID to quote (optional)

        Returns:
            TweetResult with tweet ID if successful
        """
        json_data: dict[str, Any] = {"text": text}

        if reply_to:
            json_data["reply"] = {"in_reply_to_tweet_id": reply_to}

        if quote_tweet_id:
            json_data["quote_tweet_id"] = quote_tweet_id

        try:
            result = await self._api_call(
                "POST",
                "/tweets",
                access_token,
                json_data=json_data,
            )

            tweet_data = result.get("data", {})
            return TweetResult(
                tweet_id=tweet_data.get("id", ""),
                success=True,
            )
        except RateLimitError as e:
            logger.warning("Rate limited posting tweet", reset_at=e.reset_at)
            return TweetResult(
                tweet_id="",
                success=False,
                error=f"Rate limited. Reset at: {e.reset_at}",
            )
        except TwitterApiError as e:
            logger.error("Failed to post tweet", error=str(e))
            return TweetResult(
                tweet_id="",
                success=False,
                error=str(e),
            )

    async def delete_tweet(self, access_token: str, tweet_id: str) -> bool:
        """Delete a tweet.

        Args:
            access_token: User access token
            tweet_id: Tweet ID to delete

        Returns:
            True if deleted successfully
        """
        try:
            result = await self._api_call(
                "DELETE",
                f"/tweets/{tweet_id}",
                access_token,
            )
            return result.get("data", {}).get("deleted", False)
        except TwitterApiError:
            return False

    async def get_tweet(self, tweet_id: str, access_token: str) -> Tweet | None:
        """Get a tweet by ID.

        Args:
            tweet_id: Tweet ID
            access_token: Access token

        Returns:
            Tweet or None if not found
        """
        try:
            result = await self._api_call(
                "GET",
                f"/tweets/{tweet_id}",
                access_token,
                params={"tweet.fields": "id,text,author_id,created_at,conversation_id"},
            )
            data = result["data"]
            return Tweet(
                id=data["id"],
                text=data["text"],
                author_id=data["author_id"],
                created_at=data.get("created_at"),
                conversation_id=data.get("conversation_id"),
            )
        except TwitterApiError as e:
            if e.status_code == 404:
                return None
            raise


def truncate_for_tweet(
    content: str,
    max_length: int = 280,
    suffix: str = "",
    link: str = "",
) -> str:
    """Truncate content to fit in a tweet.

    Args:
        content: Original content
        max_length: Maximum tweet length (default 280)
        suffix: Attribution suffix to append
        link: Link to include

    Returns:
        Truncated content that fits in max_length with suffix and link
    """
    # Calculate available space for content
    # We need to account for: suffix length + link + newlines before link
    available = max_length - len(suffix)
    if link:
        # Account for link and the newlines before it (\n\n)
        available -= len(link) + 2

    if len(content) <= available:
        result = content
    else:
        # Truncate and add ellipsis
        result = content[:available - 3].rstrip() + "..."

    # Build final tweet
    if link:
        result = f"{result}\n\n{link}"
    result = f"{result}{suffix}"

    return result
