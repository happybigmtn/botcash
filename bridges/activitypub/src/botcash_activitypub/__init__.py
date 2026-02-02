"""Botcash ActivityPub/Fediverse Bridge.

This package implements an ActivityPub bridge for the Botcash cryptocurrency
social network, enabling federation with Mastodon and other Fediverse servers.

Key components:
- activitypub_types: ActivityPub/ActivityStreams protocol types
- config: Pydantic configuration management
- models: SQLAlchemy database models
- botcash_client: JSON-RPC client for Botcash node
- identity: WebFinger and actor management
- protocol_mapper: Bidirectional message translation
- federation: Inbox/Outbox handlers
- main: HTTP server entry point
"""

from .activitypub_types import (
    Activity,
    ActivityType,
    Actor,
    Note,
    ObjectType,
    OrderedCollection,
    OrderedCollectionPage,
    PublicKey,
)
from .botcash_client import Balance, BotcashClient, BotcashRpcError, PostResult
from .config import (
    ActivityPubConfig,
    BotcashNodeConfig,
    BridgeConfig,
    DatabaseConfig,
    FeeConfig,
    PrivacyMode,
    load_config,
)
from .federation import FederationError, FederationService
from .identity import ActorNotFoundError, IdentityLinkError, IdentityService
from .models import (
    Follower,
    Following,
    LinkedIdentity,
    LinkStatus,
    RelayedMessage,
    RemoteActor,
    StoredActivity,
    init_db,
)
from .protocol_mapper import MappedActivity, MappedMessage, ProtocolMapper

__version__ = "0.1.0"

__all__ = [
    # Types
    "Activity",
    "ActivityType",
    "Actor",
    "Note",
    "ObjectType",
    "OrderedCollection",
    "OrderedCollectionPage",
    "PublicKey",
    # Client
    "Balance",
    "BotcashClient",
    "BotcashRpcError",
    "PostResult",
    # Config
    "ActivityPubConfig",
    "BotcashNodeConfig",
    "BridgeConfig",
    "DatabaseConfig",
    "FeeConfig",
    "PrivacyMode",
    "load_config",
    # Federation
    "FederationError",
    "FederationService",
    # Identity
    "ActorNotFoundError",
    "IdentityLinkError",
    "IdentityService",
    # Models
    "Follower",
    "Following",
    "LinkedIdentity",
    "LinkStatus",
    "RelayedMessage",
    "RemoteActor",
    "StoredActivity",
    "init_db",
    # Mapper
    "MappedActivity",
    "MappedMessage",
    "ProtocolMapper",
]
