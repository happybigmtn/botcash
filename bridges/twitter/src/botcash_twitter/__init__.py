"""Botcash X/Twitter Bridge.

This package implements a cross-posting bridge from Botcash to X/Twitter.
Due to X API restrictions, this is a read-only bridge (Botcash → Twitter only).

Key components:
- config: Pydantic configuration management
- models: SQLAlchemy database models
- botcash_client: JSON-RPC client for Botcash node
- twitter_client: Twitter API v2 client
- identity: Account linking (OAuth-based)
- crosspost: Botcash → Twitter posting service
- main: HTTP server entry point
"""

from .botcash_client import Balance, BotcashClient, BotcashRpcError, PostResult
from .config import (
    BotcashNodeConfig,
    BridgeConfig,
    DatabaseConfig,
    FeeConfig,
    PrivacyMode,
    TwitterConfig,
    load_config,
)
from .identity import IdentityLinkError, IdentityService, OAuthState
from .models import (
    CrossPostRecord,
    LinkedIdentity,
    LinkStatus,
    OAuthToken,
    RateLimitEntry,
    SponsoredTransaction,
    init_db,
)
from .twitter_client import (
    RateLimitError,
    Tweet,
    TweetResult,
    TwitterApiError,
    TwitterClient,
    TwitterUser,
)

__version__ = "0.1.0"

__all__ = [
    # Client
    "Balance",
    "BotcashClient",
    "BotcashRpcError",
    "PostResult",
    # Config
    "BotcashNodeConfig",
    "BridgeConfig",
    "DatabaseConfig",
    "FeeConfig",
    "PrivacyMode",
    "TwitterConfig",
    "load_config",
    # Identity
    "IdentityLinkError",
    "IdentityService",
    "OAuthState",
    # Models
    "CrossPostRecord",
    "LinkedIdentity",
    "LinkStatus",
    "OAuthToken",
    "RateLimitEntry",
    "SponsoredTransaction",
    "init_db",
    # Twitter
    "RateLimitError",
    "Tweet",
    "TweetResult",
    "TwitterApiError",
    "TwitterClient",
    "TwitterUser",
]
