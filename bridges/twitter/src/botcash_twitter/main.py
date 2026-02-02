"""Main entry point for Botcash X/Twitter Bridge.

Runs an HTTP server for OAuth callbacks and a background task for cross-posting.
"""

import asyncio
import signal
from datetime import datetime, timezone

import structlog
from aiohttp import web

from .botcash_client import BotcashClient
from .config import BridgeConfig, load_config
from .crosspost import CrossPostService
from .identity import IdentityService
from .models import init_db
from .twitter_client import TwitterClient

logger = structlog.get_logger()


class TwitterBridge:
    """Main bridge application."""

    def __init__(self, config: BridgeConfig):
        """Initialize bridge.

        Args:
            config: Bridge configuration
        """
        self.config = config
        self.session_maker = None
        self.botcash_client = None
        self.twitter_client = None
        self.identity_service = None
        self.crosspost_service = None
        self.app = None
        self._running = False
        self._poll_task = None

    async def setup(self) -> None:
        """Initialize database and clients."""
        # Initialize database
        self.session_maker = await init_db(self.config.database.url)

        # Initialize clients
        self.botcash_client = BotcashClient(
            rpc_url=self.config.botcash.rpc_url,
            rpc_user=self.config.botcash.rpc_user,
            rpc_password=self.config.botcash.rpc_password,
            bridge_address=self.config.botcash.bridge_address,
        )

        self.twitter_client = TwitterClient(
            client_id=self.config.twitter.client_id,
            client_secret=self.config.twitter.client_secret,
            callback_url=self.config.twitter.callback_url,
            bearer_token=self.config.twitter.bearer_token,
        )

        # Initialize services
        self.identity_service = IdentityService(
            botcash_client=self.botcash_client,
            twitter_client=self.twitter_client,
            default_privacy_mode=self.config.default_privacy_mode,
        )

        self.crosspost_service = CrossPostService(
            botcash_client=self.botcash_client,
            twitter_client=self.twitter_client,
            identity_service=self.identity_service,
            max_tweet_length=self.config.twitter.max_tweet_length,
            attribution_suffix=self.config.twitter.attribution_suffix,
            link_base_url=self.config.twitter.link_base_url,
            include_link=self.config.twitter.include_link,
        )

        logger.info("Bridge initialized")

    async def cleanup(self) -> None:
        """Clean up resources."""
        self._running = False

        if self._poll_task:
            self._poll_task.cancel()
            try:
                await self._poll_task
            except asyncio.CancelledError:
                pass

        if self.botcash_client:
            await self.botcash_client.close()
        if self.twitter_client:
            await self.twitter_client.close()

        logger.info("Bridge cleaned up")

    # === HTTP Handlers ===

    async def handle_link(self, request: web.Request) -> web.Response:
        """Start OAuth flow to link Twitter account.

        POST /link
        Body: {"botcash_address": "bs1..."}
        """
        try:
            data = await request.json()
            botcash_address = data.get("botcash_address")

            if not botcash_address:
                return web.json_response(
                    {"error": "botcash_address is required"},
                    status=400,
                )

            async with self.session_maker() as session:
                oauth_state = await self.identity_service.initiate_link(
                    session, botcash_address
                )

            return web.json_response({
                "authorization_url": oauth_state.authorization_url,
                "state": oauth_state.state,
            })

        except Exception as e:
            logger.error("Link initiation failed", error=str(e))
            return web.json_response({"error": str(e)}, status=400)

    async def handle_callback(self, request: web.Request) -> web.Response:
        """Handle OAuth callback from Twitter.

        GET /callback?code=xxx&state=xxx
        """
        code = request.query.get("code")
        state = request.query.get("state")
        error = request.query.get("error")

        if error:
            return web.json_response(
                {"error": f"Twitter authorization denied: {error}"},
                status=400,
            )

        if not code or not state:
            return web.json_response(
                {"error": "Missing code or state parameter"},
                status=400,
            )

        try:
            async with self.session_maker() as session:
                identity = await self.identity_service.complete_link(
                    session, state, code
                )

            # Redirect to success page or return JSON
            return web.json_response({
                "success": True,
                "twitter_username": identity.twitter_username,
                "botcash_address": identity.botcash_address,
                "message": f"Successfully linked @{identity.twitter_username}",
            })

        except Exception as e:
            logger.error("OAuth callback failed", error=str(e))
            return web.json_response({"error": str(e)}, status=400)

    async def handle_unlink(self, request: web.Request) -> web.Response:
        """Unlink Twitter account.

        POST /unlink
        Body: {"botcash_address": "bs1..."}
        """
        try:
            data = await request.json()
            botcash_address = data.get("botcash_address")

            if not botcash_address:
                return web.json_response(
                    {"error": "botcash_address is required"},
                    status=400,
                )

            async with self.session_maker() as session:
                success = await self.identity_service.unlink(session, botcash_address)

            if success:
                return web.json_response({
                    "success": True,
                    "message": "Twitter account unlinked",
                })
            else:
                return web.json_response(
                    {"error": "No active link found"},
                    status=404,
                )

        except Exception as e:
            logger.error("Unlink failed", error=str(e))
            return web.json_response({"error": str(e)}, status=500)

    async def handle_status(self, request: web.Request) -> web.Response:
        """Get link status.

        GET /status?botcash_address=bs1...
        """
        botcash_address = request.query.get("botcash_address")

        if not botcash_address:
            return web.json_response(
                {"error": "botcash_address is required"},
                status=400,
            )

        try:
            async with self.session_maker() as session:
                status = await self.identity_service.get_status(
                    session, botcash_address
                )

            if status:
                return web.json_response(status)
            else:
                return web.json_response(
                    {"error": "No link found"},
                    status=404,
                )

        except Exception as e:
            logger.error("Status check failed", error=str(e))
            return web.json_response({"error": str(e)}, status=500)

    async def handle_crosspost(self, request: web.Request) -> web.Response:
        """Manually trigger cross-post.

        POST /crosspost
        Body: {"botcash_address": "bs1...", "botcash_tx_id": "...", "content": "..."}
        """
        try:
            data = await request.json()
            botcash_address = data.get("botcash_address")
            botcash_tx_id = data.get("botcash_tx_id")
            content = data.get("content")

            if not all([botcash_address, botcash_tx_id, content]):
                return web.json_response(
                    {"error": "botcash_address, botcash_tx_id, and content are required"},
                    status=400,
                )

            async with self.session_maker() as session:
                record = await self.crosspost_service.cross_post(
                    session=session,
                    botcash_tx_id=botcash_tx_id,
                    content=content,
                    botcash_address=botcash_address,
                    force=True,  # Manual trigger = opt-in
                )

            if record.success:
                return web.json_response({
                    "success": True,
                    "tweet_id": record.tweet_id,
                })
            else:
                return web.json_response({
                    "success": False,
                    "error": record.error,
                })

        except ValueError as e:
            return web.json_response({"error": str(e)}, status=400)
        except Exception as e:
            logger.error("Cross-post failed", error=str(e))
            return web.json_response({"error": str(e)}, status=500)

    async def handle_privacy(self, request: web.Request) -> web.Response:
        """Update privacy mode.

        POST /privacy
        Body: {"botcash_address": "bs1...", "mode": "full_mirror|selective|disabled"}
        """
        try:
            data = await request.json()
            botcash_address = data.get("botcash_address")
            mode = data.get("mode")

            if not botcash_address or not mode:
                return web.json_response(
                    {"error": "botcash_address and mode are required"},
                    status=400,
                )

            from .models import PrivacyMode
            try:
                privacy_mode = PrivacyMode(mode)
            except ValueError:
                return web.json_response(
                    {"error": f"Invalid mode: {mode}. Use: full_mirror, selective, disabled"},
                    status=400,
                )

            async with self.session_maker() as session:
                success = await self.identity_service.set_privacy_mode(
                    session, botcash_address, privacy_mode
                )

            if success:
                return web.json_response({
                    "success": True,
                    "privacy_mode": mode,
                })
            else:
                return web.json_response(
                    {"error": "No active link found"},
                    status=404,
                )

        except Exception as e:
            logger.error("Privacy update failed", error=str(e))
            return web.json_response({"error": str(e)}, status=500)

    async def handle_health(self, request: web.Request) -> web.Response:
        """Health check endpoint.

        GET /health
        """
        try:
            # Check Botcash connection
            info = await self.botcash_client.get_blockchain_info()
            botcash_ok = bool(info)
        except Exception:
            botcash_ok = False

        return web.json_response({
            "status": "healthy" if botcash_ok else "degraded",
            "botcash_connected": botcash_ok,
            "timestamp": datetime.now(timezone.utc).isoformat(),
        })

    # === Background Polling ===

    async def _poll_loop(self) -> None:
        """Background task to poll for new Botcash posts."""
        logger.info("Starting cross-post polling loop")

        while self._running:
            try:
                async with self.session_maker() as session:
                    results = await self.crosspost_service.process_new_posts(session)
                    if results:
                        logger.info(
                            "Processed new posts",
                            count=len(results),
                            successful=sum(1 for r in results if r.success),
                        )
            except Exception as e:
                logger.error("Polling error", error=str(e))

            await asyncio.sleep(self.config.poll_interval_seconds)

    # === Server Setup ===

    def create_app(self) -> web.Application:
        """Create aiohttp web application."""
        app = web.Application()

        # Routes
        app.router.add_post("/link", self.handle_link)
        app.router.add_get("/callback", self.handle_callback)
        app.router.add_post("/unlink", self.handle_unlink)
        app.router.add_get("/status", self.handle_status)
        app.router.add_post("/crosspost", self.handle_crosspost)
        app.router.add_post("/privacy", self.handle_privacy)
        app.router.add_get("/health", self.handle_health)

        return app

    async def run(self) -> None:
        """Run the bridge server."""
        await self.setup()
        self._running = True

        # Create app
        self.app = self.create_app()

        # Start background polling
        self._poll_task = asyncio.create_task(self._poll_loop())

        # Start server
        runner = web.AppRunner(self.app)
        await runner.setup()
        site = web.TCPSite(
            runner,
            self.config.server.host,
            self.config.server.port,
        )
        await site.start()

        logger.info(
            "Bridge server started",
            host=self.config.server.host,
            port=self.config.server.port,
        )

        # Wait for shutdown signal
        try:
            while self._running:
                await asyncio.sleep(1)
        finally:
            await self.cleanup()
            await runner.cleanup()


async def async_main(config: BridgeConfig | None = None) -> None:
    """Async main entry point."""
    if config is None:
        config = load_config()

    # Configure logging
    structlog.configure(
        wrapper_class=structlog.make_filtering_bound_logger(
            getattr(structlog, config.log_level.upper(), structlog.INFO)
        ),
    )

    bridge = TwitterBridge(config)

    # Handle shutdown signals
    loop = asyncio.get_running_loop()

    def shutdown():
        bridge._running = False

    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, shutdown)

    await bridge.run()


def main() -> None:
    """Main entry point."""
    asyncio.run(async_main())


if __name__ == "__main__":
    main()
