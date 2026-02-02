"""Main entry point for Botcash Nostr Bridge."""

import asyncio
import sys

import structlog

from .botcash_client import BotcashClient
from .config import BridgeConfig, load_config
from .identity import IdentityService
from .models import init_db
from .protocol_mapper import ProtocolMapper
from .relay import start_relay

# Configure structlog
structlog.configure(
    processors=[
        structlog.stdlib.filter_by_level,
        structlog.stdlib.add_logger_name,
        structlog.stdlib.add_log_level,
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.StackInfoRenderer(),
        structlog.processors.format_exc_info,
        structlog.dev.ConsoleRenderer(),
    ],
    wrapper_class=structlog.stdlib.BoundLogger,
    context_class=dict,
    logger_factory=structlog.stdlib.LoggerFactory(),
    cache_logger_on_first_use=True,
)

logger = structlog.get_logger()


async def run_bridge(config: BridgeConfig) -> None:
    """Run the Nostr bridge.

    Args:
        config: Bridge configuration
    """
    logger.info("Starting Botcash Nostr Bridge")

    # Initialize database
    logger.info("Initializing database", url=config.database.url)
    session_maker = await init_db(config.database.url)

    # Initialize Botcash client
    logger.info("Connecting to Botcash node", url=config.botcash.rpc_url)
    botcash_client = BotcashClient(
        rpc_url=config.botcash.rpc_url,
        rpc_user=config.botcash.rpc_user,
        rpc_password=config.botcash.rpc_password,
        bridge_address=config.botcash.bridge_address,
    )

    # Verify Botcash connection
    try:
        blockchain_info = await botcash_client.get_blockchain_info()
        logger.info(
            "Connected to Botcash node",
            chain=blockchain_info.get("chain"),
            blocks=blockchain_info.get("blocks"),
        )
    except Exception as e:
        logger.error("Failed to connect to Botcash node", error=str(e))
        logger.warning("Continuing without Botcash connection (relay-only mode)")

    # Initialize services
    identity_service = IdentityService(botcash_client)
    protocol_mapper = ProtocolMapper(
        zap_conversion_rate=config.fees.zap_conversion_rate,
    )

    # Start relay
    try:
        await start_relay(
            host=config.nostr.relay_host,
            port=config.nostr.relay_port,
            session_maker=session_maker,
            botcash_client=botcash_client,
            identity_service=identity_service,
            protocol_mapper=protocol_mapper,
            allowed_kinds=config.nostr.allowed_kinds,
            rate_limit_per_minute=config.nostr.rate_limit_events_per_minute,
        )
    finally:
        await botcash_client.close()


def main() -> None:
    """Main entry point."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Botcash Nostr Bridge - WebSocket relay for Botcash/Nostr interoperability"
    )
    parser.add_argument(
        "--config",
        "-c",
        type=str,
        help="Path to YAML configuration file",
    )
    parser.add_argument(
        "--host",
        type=str,
        help="Override relay host",
    )
    parser.add_argument(
        "--port",
        "-p",
        type=int,
        help="Override relay port",
    )

    args = parser.parse_args()

    # Load configuration
    if args.config:
        config = BridgeConfig.from_yaml(args.config)
    else:
        config = load_config()

    # Apply command-line overrides
    if args.host:
        config.nostr.relay_host = args.host
    if args.port:
        config.nostr.relay_port = args.port

    # Run the bridge
    try:
        asyncio.run(run_bridge(config))
    except KeyboardInterrupt:
        logger.info("Shutting down...")
        sys.exit(0)


if __name__ == "__main__":
    main()
