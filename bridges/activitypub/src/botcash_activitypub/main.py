"""Main entry point for Botcash ActivityPub Bridge server.

Implements an aiohttp-based HTTP server with:
- WebFinger endpoint (/.well-known/webfinger)
- Actor endpoints (/users/{username})
- Inbox/Outbox endpoints
- Federation protocol handling
"""

import asyncio
import json
import logging
import signal
import sys
from typing import Any

import structlog
from aiohttp import web

from .activitypub_types import AP_ACCEPT_HEADER, AP_CONTENT_TYPE
from .botcash_client import BotcashClient
from .config import BridgeConfig, load_config
from .federation import FederationService
from .identity import IdentityService
from .models import init_db
from .protocol_mapper import ProtocolMapper

# Configure structured logging
structlog.configure(
    processors=[
        structlog.stdlib.filter_by_level,
        structlog.stdlib.add_logger_name,
        structlog.stdlib.add_log_level,
        structlog.stdlib.PositionalArgumentsFormatter(),
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.StackInfoRenderer(),
        structlog.processors.format_exc_info,
        structlog.processors.UnicodeDecoder(),
        structlog.processors.JSONRenderer(),
    ],
    wrapper_class=structlog.stdlib.BoundLogger,
    context_class=dict,
    logger_factory=structlog.stdlib.LoggerFactory(),
    cache_logger_on_first_use=True,
)

logger = structlog.get_logger()


class ActivityPubServer:
    """ActivityPub/Fediverse bridge server."""

    def __init__(self, config: BridgeConfig):
        """Initialize server.

        Args:
            config: Bridge configuration
        """
        self.config = config
        self.app = web.Application()
        self.session_maker = None
        self.botcash_client = None
        self.identity_service = None
        self.federation_service = None
        self.protocol_mapper = None

    async def setup(self) -> None:
        """Set up server components."""
        # Initialize database
        self.session_maker = await init_db(self.config.database.url)

        # Initialize services
        self.botcash_client = BotcashClient(
            rpc_url=self.config.botcash.rpc_url,
            rpc_user=self.config.botcash.rpc_user,
            rpc_password=self.config.botcash.rpc_password,
            bridge_address=self.config.botcash.bridge_address,
        )

        self.identity_service = IdentityService(
            botcash_client=self.botcash_client,
            base_url=self.config.activitypub.base_url,
            domain=self.config.activitypub.domain,
        )

        self.protocol_mapper = ProtocolMapper(
            base_url=self.config.activitypub.base_url,
            domain=self.config.activitypub.domain,
        )

        self.federation_service = FederationService(
            identity_service=self.identity_service,
            protocol_mapper=self.protocol_mapper,
            botcash_client=self.botcash_client,
            base_url=self.config.activitypub.base_url,
            domain=self.config.activitypub.domain,
        )

        # Set up routes
        self._setup_routes()

        # Store services in app for handlers
        self.app["config"] = self.config
        self.app["session_maker"] = self.session_maker
        self.app["identity"] = self.identity_service
        self.app["federation"] = self.federation_service
        self.app["mapper"] = self.protocol_mapper

        logger.info(
            "Server setup complete",
            domain=self.config.activitypub.domain,
            base_url=self.config.activitypub.base_url,
        )

    async def cleanup(self) -> None:
        """Clean up server resources."""
        if self.botcash_client:
            await self.botcash_client.close()
        if self.federation_service:
            await self.federation_service.close()

    def _setup_routes(self) -> None:
        """Set up HTTP routes."""
        self.app.router.add_get("/.well-known/webfinger", handle_webfinger)
        self.app.router.add_get("/.well-known/nodeinfo", handle_nodeinfo_wellknown)
        self.app.router.add_get("/nodeinfo/2.0", handle_nodeinfo)

        self.app.router.add_get("/users/{username}", handle_actor)
        self.app.router.add_post("/users/{username}/inbox", handle_inbox)
        self.app.router.add_get("/users/{username}/outbox", handle_outbox)
        self.app.router.add_get("/users/{username}/followers", handle_followers)
        self.app.router.add_get("/users/{username}/following", handle_following)

        # Health check
        self.app.router.add_get("/health", handle_health)

    async def run(self) -> None:
        """Run the server."""
        await self.setup()

        runner = web.AppRunner(self.app)
        await runner.setup()

        site = web.TCPSite(
            runner,
            self.config.activitypub.host,
            self.config.activitypub.port,
        )

        await site.start()

        logger.info(
            "Server started",
            host=self.config.activitypub.host,
            port=self.config.activitypub.port,
        )

        # Wait for shutdown signal
        stop_event = asyncio.Event()

        def signal_handler():
            stop_event.set()

        loop = asyncio.get_event_loop()
        for sig in (signal.SIGTERM, signal.SIGINT):
            loop.add_signal_handler(sig, signal_handler)

        await stop_event.wait()

        logger.info("Shutting down...")
        await self.cleanup()
        await runner.cleanup()


# === Route Handlers ===

async def handle_webfinger(request: web.Request) -> web.Response:
    """Handle WebFinger discovery requests."""
    resource = request.query.get("resource", "")
    if not resource:
        return web.json_response(
            {"error": "Missing resource parameter"},
            status=400,
        )

    async with request.app["session_maker"]() as session:
        identity_service = request.app["identity"]
        result = await identity_service.webfinger_lookup(session, resource)

    if not result:
        return web.json_response(
            {"error": "Resource not found"},
            status=404,
        )

    return web.json_response(
        result,
        content_type="application/jrd+json",
    )


async def handle_nodeinfo_wellknown(request: web.Request) -> web.Response:
    """Handle NodeInfo well-known endpoint."""
    config = request.app["config"]
    return web.json_response({
        "links": [
            {
                "rel": "http://nodeinfo.diaspora.software/ns/schema/2.0",
                "href": f"{config.activitypub.base_url}/nodeinfo/2.0",
            }
        ]
    })


async def handle_nodeinfo(request: web.Request) -> web.Response:
    """Handle NodeInfo endpoint."""
    config = request.app["config"]
    return web.json_response({
        "version": "2.0",
        "software": {
            "name": "botcash-activitypub-bridge",
            "version": "0.1.0",
        },
        "protocols": ["activitypub"],
        "usage": {
            "users": {
                "total": 0,  # TODO: Count from database
                "activeMonth": 0,
                "activeHalfyear": 0,
            },
            "localPosts": 0,
        },
        "openRegistrations": True,
        "metadata": {
            "nodeName": f"Botcash Bridge ({config.activitypub.domain})",
            "nodeDescription": "ActivityPub bridge for Botcash cryptocurrency social network",
        },
    })


async def handle_actor(request: web.Request) -> web.Response:
    """Handle actor profile request."""
    username = request.match_info["username"]

    # Check Accept header for ActivityPub
    accept = request.headers.get("Accept", "")
    if "application/activity+json" not in accept and "application/ld+json" not in accept:
        # Return HTML profile page for browsers
        return web.Response(
            text=f"<html><body><h1>@{username}@{request.app['config'].activitypub.domain}</h1></body></html>",
            content_type="text/html",
        )

    async with request.app["session_maker"]() as session:
        identity_service = request.app["identity"]
        identity = await identity_service.get_actor_by_local_part(session, username)

        if not identity:
            return web.json_response(
                {"error": "Actor not found"},
                status=404,
            )

        actor = identity_service.build_actor_object(identity)

    return web.json_response(
        actor.to_dict(),
        content_type=AP_CONTENT_TYPE,
    )


async def handle_inbox(request: web.Request) -> web.Response:
    """Handle incoming ActivityPub activities."""
    username = request.match_info["username"]

    # Parse activity
    try:
        activity_data = await request.json()
    except json.JSONDecodeError:
        return web.json_response(
            {"error": "Invalid JSON"},
            status=400,
        )

    logger.info(
        "Received inbox activity",
        username=username,
        activity_type=activity_data.get("type"),
        activity_id=activity_data.get("id"),
    )

    # TODO: Verify HTTP signature

    async with request.app["session_maker"]() as session:
        federation_service = request.app["federation"]
        try:
            result = await federation_service.handle_inbox(
                session,
                username,
                activity_data,
                signature_verified=False,  # TODO: Implement signature verification
            )
            return web.json_response(result, status=202)
        except Exception as e:
            logger.error("Inbox processing error", error=str(e))
            return web.json_response(
                {"error": str(e)},
                status=500,
            )


async def handle_outbox(request: web.Request) -> web.Response:
    """Handle outbox collection request."""
    username = request.match_info["username"]
    page = request.query.get("page")

    page_num = int(page) if page else None

    async with request.app["session_maker"]() as session:
        federation_service = request.app["federation"]
        try:
            result = await federation_service.get_outbox(session, username, page_num)
            return web.json_response(result, content_type=AP_CONTENT_TYPE)
        except Exception as e:
            return web.json_response({"error": str(e)}, status=404)


async def handle_followers(request: web.Request) -> web.Response:
    """Handle followers collection request."""
    username = request.match_info["username"]
    page = request.query.get("page")

    page_num = int(page) if page else None

    async with request.app["session_maker"]() as session:
        federation_service = request.app["federation"]
        try:
            result = await federation_service.get_followers(session, username, page_num)
            return web.json_response(result, content_type=AP_CONTENT_TYPE)
        except Exception as e:
            return web.json_response({"error": str(e)}, status=404)


async def handle_following(request: web.Request) -> web.Response:
    """Handle following collection request."""
    username = request.match_info["username"]
    page = request.query.get("page")

    page_num = int(page) if page else None

    async with request.app["session_maker"]() as session:
        federation_service = request.app["federation"]
        try:
            result = await federation_service.get_following(session, username, page_num)
            return web.json_response(result, content_type=AP_CONTENT_TYPE)
        except Exception as e:
            return web.json_response({"error": str(e)}, status=404)


async def handle_health(request: web.Request) -> web.Response:
    """Health check endpoint."""
    return web.json_response({"status": "ok"})


def main() -> None:
    """Main entry point."""
    # Configure standard logging
    logging.basicConfig(
        level=logging.INFO,
        format="%(message)s",
    )

    config = load_config()

    # Set log level from config
    log_level = getattr(logging, config.log_level.upper(), logging.INFO)
    logging.getLogger().setLevel(log_level)

    server = ActivityPubServer(config)

    try:
        asyncio.run(server.run())
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
