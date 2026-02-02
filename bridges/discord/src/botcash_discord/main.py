"""Main entry point for Botcash Discord Bridge."""

import asyncio
import logging
import sys
from typing import Optional

import discord
import structlog
from discord.ext import commands

from .botcash_client import BotcashClient
from .commands import BotcashCommands
from .config import BridgeConfig, load_config
from .identity import IdentityService
from .models import init_db


def setup_logging(log_level: str) -> None:
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
            structlog.dev.ConsoleRenderer(),
        ],
        context_class=dict,
        logger_factory=structlog.stdlib.LoggerFactory(),
        wrapper_class=structlog.stdlib.BoundLogger,
        cache_logger_on_first_use=True,
    )

    logging.basicConfig(
        format="%(message)s",
        stream=sys.stdout,
        level=getattr(logging, log_level.upper()),
    )


class BotcashDiscordBot(commands.Bot):
    """Custom Discord bot class for Botcash Bridge."""

    def __init__(
        self,
        config: BridgeConfig,
        **kwargs,
    ):
        """Initialize the bot.

        Args:
            config: Bridge configuration
            **kwargs: Additional arguments for commands.Bot
        """
        intents = discord.Intents.default()
        intents.message_content = True  # Required for message content access

        super().__init__(
            command_prefix="!",  # Not used since we use slash commands
            intents=intents,
            **kwargs,
        )

        self.config = config
        self.botcash_client: Optional[BotcashClient] = None
        self.identity_service: Optional[IdentityService] = None
        self.session_maker = None
        self.logger = structlog.get_logger()

    async def setup_hook(self) -> None:
        """Called when the bot is starting up."""
        self.logger.info("Initializing Botcash Discord Bridge...")

        # Initialize database
        self.session_maker = await init_db(self.config.database.url)
        self.logger.info("Database initialized", url=self.config.database.url)

        # Initialize Botcash client
        self.botcash_client = BotcashClient(
            rpc_url=self.config.botcash.rpc_url,
            rpc_user=self.config.botcash.rpc_user,
            rpc_password=self.config.botcash.rpc_password,
            bridge_address=self.config.botcash.bridge_address,
        )

        # Test connection to Botcash node
        try:
            info = await self.botcash_client.get_blockchain_info()
            self.logger.info(
                "Connected to Botcash node",
                chain=info.get("chain"),
                blocks=info.get("blocks"),
            )
        except Exception as e:
            self.logger.warning(
                "Could not connect to Botcash node",
                error=str(e),
                url=self.config.botcash.rpc_url,
            )

        # Initialize identity service
        self.identity_service = IdentityService(self.botcash_client)

        # Add command cog
        cog = BotcashCommands(
            bot=self,
            config=self.config,
            session_maker=self.session_maker,
            botcash_client=self.botcash_client,
            identity_service=self.identity_service,
        )
        await self.add_cog(cog)

        # Sync commands with Discord
        if self.config.discord.allowed_guild_ids:
            # Sync to specific guilds (faster for development)
            for guild_id in self.config.discord.allowed_guild_ids:
                guild = discord.Object(id=guild_id)
                self.tree.copy_global_to(guild=guild)
                await self.tree.sync(guild=guild)
                self.logger.info("Synced commands to guild", guild_id=guild_id)
        else:
            # Sync globally (takes up to an hour to propagate)
            await self.tree.sync()
            self.logger.info("Synced commands globally")

    async def on_ready(self) -> None:
        """Called when the bot is ready."""
        self.logger.info(
            "Bot ready",
            user=str(self.user),
            guilds=len(self.guilds),
        )

        # Set bot status
        await self.change_presence(
            activity=discord.Activity(
                type=discord.ActivityType.watching,
                name="Botcash Network | /bcash_help",
            )
        )

    async def on_guild_join(self, guild: discord.Guild) -> None:
        """Called when the bot joins a new guild."""
        self.logger.info("Joined guild", guild=guild.name, guild_id=guild.id)

        # Sync commands to the new guild if using guild-specific commands
        if self.config.discord.allowed_guild_ids:
            if guild.id not in self.config.discord.allowed_guild_ids:
                self.logger.warning(
                    "Guild not in allowed list",
                    guild_id=guild.id,
                )
                return

            self.tree.copy_global_to(guild=guild)
            await self.tree.sync(guild=guild)
            self.logger.info("Synced commands to new guild", guild_id=guild.id)

    async def close(self) -> None:
        """Clean up resources before shutdown."""
        self.logger.info("Shutting down...")

        if self.botcash_client:
            await self.botcash_client.close()

        await super().close()


def main() -> None:
    """Main entry point."""
    # Load configuration
    try:
        config = load_config()
    except Exception as e:
        print(f"Failed to load configuration: {e}")
        sys.exit(1)

    # Setup logging
    setup_logging(config.log_level)

    logger = structlog.get_logger()
    logger.info("Starting Botcash Discord Bridge", version="0.1.0")

    # Create and run bot
    bot = BotcashDiscordBot(config)

    try:
        bot.run(config.discord.bot_token)
    except discord.LoginFailure:
        logger.error("Invalid bot token")
        sys.exit(1)
    except KeyboardInterrupt:
        logger.info("Received keyboard interrupt")
    except Exception as e:
        logger.exception("Unexpected error", error=str(e))
        sys.exit(1)


if __name__ == "__main__":
    main()
