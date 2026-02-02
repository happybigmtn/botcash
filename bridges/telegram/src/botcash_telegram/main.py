"""Main entry point for Botcash Telegram Bridge."""

import asyncio
import logging
import sys

import structlog
from telegram.ext import Application, CommandHandler

from .botcash_client import BotcashClient
from .config import load_config
from .handlers import BotHandlers
from .identity import IdentityService
from .models import init_db


def configure_logging(level: str) -> None:
    """Configure structured logging."""
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
            structlog.dev.ConsoleRenderer() if sys.stdout.isatty() else structlog.processors.JSONRenderer(),
        ],
        wrapper_class=structlog.stdlib.BoundLogger,
        context_class=dict,
        logger_factory=structlog.stdlib.LoggerFactory(),
        cache_logger_on_first_use=True,
    )

    logging.basicConfig(
        format="%(message)s",
        stream=sys.stdout,
        level=getattr(logging, level.upper()),
    )


async def run_bot() -> None:
    """Run the Telegram bot."""
    logger = structlog.get_logger()

    # Load configuration
    try:
        config = load_config()
    except Exception as e:
        logger.error("Failed to load configuration", error=str(e))
        sys.exit(1)

    configure_logging(config.log_level)
    logger.info("Starting Botcash Telegram Bridge", version="0.1.0")

    # Initialize database
    logger.info("Initializing database", url=config.database.url.split("@")[-1])
    session_maker = await init_db(config.database.url)

    # Initialize Botcash client
    botcash_client = BotcashClient(
        rpc_url=config.botcash.rpc_url,
        rpc_user=config.botcash.rpc_user,
        rpc_password=config.botcash.rpc_password,
        bridge_address=config.botcash.bridge_address,
    )

    # Verify connection to Botcash node
    try:
        info = await botcash_client.get_blockchain_info()
        logger.info(
            "Connected to Botcash node",
            chain=info.get("chain", "unknown"),
            blocks=info.get("blocks", 0),
        )
    except Exception as e:
        logger.warning("Could not connect to Botcash node", error=str(e))
        logger.info("Bridge will start but Botcash operations may fail")

    # Initialize services
    identity_service = IdentityService(botcash_client)
    handlers = BotHandlers(config, session_maker, botcash_client, identity_service)

    # Build Telegram application
    application = Application.builder().token(config.telegram.bot_token).build()

    # Register handlers
    application.add_handler(CommandHandler("start", handlers.start))
    application.add_handler(CommandHandler("help", handlers.help_command))
    application.add_handler(CommandHandler("link", handlers.link))
    application.add_handler(CommandHandler("verify", handlers.verify))
    application.add_handler(CommandHandler("unlink", handlers.unlink))
    application.add_handler(CommandHandler("status", handlers.status))
    application.add_handler(CommandHandler("balance", handlers.balance))
    application.add_handler(CommandHandler("post", handlers.post))
    application.add_handler(CommandHandler("dm", handlers.dm))
    application.add_handler(CommandHandler("feed", handlers.feed))
    application.add_handler(CommandHandler("privacy", handlers.privacy))

    logger.info("Bot handlers registered")

    # Run the bot
    logger.info("Starting Telegram bot polling")
    await application.initialize()
    await application.start()
    await application.updater.start_polling()

    # Keep running until interrupted
    try:
        while True:
            await asyncio.sleep(1)
    except asyncio.CancelledError:
        pass
    finally:
        logger.info("Shutting down...")
        await application.updater.stop()
        await application.stop()
        await application.shutdown()
        await botcash_client.close()


def main() -> None:
    """Entry point."""
    try:
        asyncio.run(run_bot())
    except KeyboardInterrupt:
        print("\nShutdown requested.")


if __name__ == "__main__":
    main()
