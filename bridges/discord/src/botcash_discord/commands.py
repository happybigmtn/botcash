"""Discord slash command handlers for Botcash bridge."""

import hashlib
import re
from datetime import datetime, timezone
from typing import Optional

import discord
import structlog
from discord import app_commands
from discord.ext import commands
from sqlalchemy import func, select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker

from .botcash_client import BotcashClient
from .config import BridgeConfig
from .embeds import (
    create_balance_embed,
    create_dm_success_embed,
    create_error_embed,
    create_feed_embed,
    create_info_embed,
    create_link_complete_embed,
    create_link_initiated_embed,
    create_post_success_embed,
    create_status_embed,
    create_unlink_embed,
    create_welcome_embed,
)
from .identity import IdentityLinkError, IdentityService
from .models import (
    LinkedIdentity,
    LinkStatus,
    PrivacyMode,
    RateLimitEntry,
    RelayedMessage,
    SponsoredTransaction,
)

logger = structlog.get_logger()

# Botcash address regex patterns
BOTCASH_ADDRESS_PATTERN = re.compile(
    r"^(bs1[a-z0-9]{59}|bu1[a-z0-9]{59}|B1[a-zA-Z0-9]{33}|B3[a-zA-Z0-9]{33})$"
)


class BotcashCommands(commands.Cog):
    """Cog containing all Botcash slash commands."""

    def __init__(
        self,
        bot: commands.Bot,
        config: BridgeConfig,
        session_maker: async_sessionmaker[AsyncSession],
        botcash_client: BotcashClient,
        identity_service: IdentityService,
    ):
        """Initialize the commands cog.

        Args:
            bot: Discord bot instance
            config: Bridge configuration
            session_maker: Database session factory
            botcash_client: Botcash RPC client
            identity_service: Identity linking service
        """
        self.bot = bot
        self.config = config
        self.session_maker = session_maker
        self.botcash = botcash_client
        self.identity = identity_service

    async def _check_rate_limit(
        self,
        session: AsyncSession,
        discord_user_id: int,
    ) -> bool:
        """Check if user is within rate limit.

        Args:
            session: Database session
            discord_user_id: Discord user ID

        Returns:
            True if within limit, False if exceeded
        """
        window_start = datetime.now(timezone.utc).replace(second=0, microsecond=0)

        result = await session.execute(
            select(RateLimitEntry).where(
                RateLimitEntry.discord_user_id == discord_user_id,
                RateLimitEntry.window_start == window_start,
            )
        )
        entry = result.scalar_one_or_none()

        if entry:
            if entry.message_count >= self.config.discord.rate_limit_messages_per_minute:
                return False
            entry.message_count += 1
        else:
            entry = RateLimitEntry(
                discord_user_id=discord_user_id,
                window_start=window_start,
                message_count=1,
            )
            session.add(entry)

        await session.commit()
        return True

    async def _can_sponsor(self, session: AsyncSession) -> bool:
        """Check if bridge can sponsor another transaction today.

        Args:
            session: Database session

        Returns:
            True if within daily sponsorship limit
        """
        if self.config.fees.max_sponsored_per_day == 0:
            return True  # Unlimited

        today_start = datetime.now(timezone.utc).replace(hour=0, minute=0, second=0, microsecond=0)

        result = await session.execute(
            select(func.count(SponsoredTransaction.id)).where(
                SponsoredTransaction.created_at >= today_start
            )
        )
        count = result.scalar() or 0
        return count < self.config.fees.max_sponsored_per_day

    @app_commands.command(name="bcash_help", description="Show Botcash bridge help and commands")
    async def help_command(self, interaction: discord.Interaction) -> None:
        """Show help information."""
        await interaction.response.send_message(embed=create_welcome_embed())

    @app_commands.command(name="bcash_link", description="Link your Discord account to a Botcash address")
    @app_commands.describe(address="Your Botcash address (bs1..., bu1..., B1..., or B3...)")
    async def link(self, interaction: discord.Interaction, address: str) -> None:
        """Initiate identity linking process."""
        # Validate address format
        if not BOTCASH_ADDRESS_PATTERN.match(address):
            await interaction.response.send_message(
                embed=create_error_embed(
                    "Invalid Address",
                    "Supported formats:\n"
                    "- Shielded: `bs1...` (Sapling) or `bu1...` (Unified)\n"
                    "- Transparent: `B1...` or `B3...`"
                ),
                ephemeral=True,
            )
            return

        async with self.session_maker() as session:
            try:
                challenge, verification_msg = await self.identity.initiate_link(
                    session,
                    discord_user_id=interaction.user.id,
                    discord_username=interaction.user.name,
                    discord_discriminator=interaction.user.discriminator if hasattr(interaction.user, 'discriminator') else None,
                    botcash_address=address,
                )

                await interaction.response.send_message(
                    embed=create_link_initiated_embed(verification_msg, address),
                    ephemeral=True,
                )

            except IdentityLinkError as e:
                await interaction.response.send_message(
                    embed=create_error_embed("Link Failed", str(e)),
                    ephemeral=True,
                )

    @app_commands.command(name="bcash_verify", description="Complete linking with your signature")
    @app_commands.describe(signature="The signature from your Botcash wallet")
    async def verify(self, interaction: discord.Interaction, signature: str) -> None:
        """Complete identity linking by verifying signature."""
        async with self.session_maker() as session:
            try:
                identity = await self.identity.complete_link(
                    session,
                    discord_user_id=interaction.user.id,
                    signature=signature,
                )

                await interaction.response.send_message(
                    embed=create_link_complete_embed(
                        interaction.user,
                        identity.botcash_address,
                        identity.link_tx_id or "",
                    ),
                )

            except IdentityLinkError as e:
                await interaction.response.send_message(
                    embed=create_error_embed("Verification Failed", str(e)),
                    ephemeral=True,
                )

    @app_commands.command(name="bcash_unlink", description="Unlink your Discord account from Botcash")
    async def unlink(self, interaction: discord.Interaction) -> None:
        """Unlink account."""
        async with self.session_maker() as session:
            success = await self.identity.unlink(session, interaction.user.id)

        if success:
            await interaction.response.send_message(embed=create_unlink_embed())
        else:
            await interaction.response.send_message(
                embed=create_error_embed("Not Linked", "No linked account found."),
                ephemeral=True,
            )

    @app_commands.command(name="bcash_status", description="Check your link status")
    async def status(self, interaction: discord.Interaction) -> None:
        """Show link status."""
        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, interaction.user.id)

        if not identity:
            await interaction.response.send_message(
                embed=create_info_embed(
                    "Not Linked",
                    "No linked account.\nUse `/bcash_link` to get started."
                ),
                ephemeral=True,
            )
            return

        # Get balance
        try:
            balance = await self.botcash.get_balance(identity.botcash_address)
            balance_bcash = balance.confirmed_bcash
        except Exception:
            balance_bcash = 0.0

        await interaction.response.send_message(
            embed=create_status_embed(
                interaction.user,
                identity.botcash_address,
                balance_bcash,
                identity.privacy_mode.value,
                identity.linked_at,
            ),
        )

    @app_commands.command(name="bcash_balance", description="Check your BCASH balance")
    async def balance(self, interaction: discord.Interaction) -> None:
        """Show balance."""
        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, interaction.user.id)

        if not identity:
            await interaction.response.send_message(
                embed=create_error_embed("Not Linked", "Use `/bcash_link` first."),
                ephemeral=True,
            )
            return

        try:
            balance = await self.botcash.get_balance(identity.botcash_address)
            await interaction.response.send_message(
                embed=create_balance_embed(
                    identity.botcash_address,
                    balance.confirmed_bcash,
                    balance.pending / 100_000_000,
                ),
            )
        except Exception as e:
            logger.error("Failed to get balance", error=str(e))
            await interaction.response.send_message(
                embed=create_error_embed("Error", "Failed to retrieve balance. Please try again."),
                ephemeral=True,
            )

    @app_commands.command(name="bcash_post", description="Post to the Botcash network")
    @app_commands.describe(content="Your post content (max 450 characters)")
    async def post(self, interaction: discord.Interaction, content: str) -> None:
        """Create a post on Botcash."""
        if len(content) > 450:
            await interaction.response.send_message(
                embed=create_error_embed(
                    "Content Too Long",
                    f"Post is {len(content)} characters. Maximum is 450."
                ),
                ephemeral=True,
            )
            return

        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, interaction.user.id)

            if not identity:
                await interaction.response.send_message(
                    embed=create_error_embed("Not Linked", "Use `/bcash_link` first."),
                    ephemeral=True,
                )
                return

            # Check rate limit
            if not await self._check_rate_limit(session, interaction.user.id):
                await interaction.response.send_message(
                    embed=create_error_embed(
                        "Rate Limited",
                        f"Maximum {self.config.discord.rate_limit_messages_per_minute} messages per minute."
                    ),
                    ephemeral=True,
                )
                return

            # Check privacy mode
            if identity.privacy_mode == PrivacyMode.READ_ONLY:
                await interaction.response.send_message(
                    embed=create_error_embed(
                        "Read-Only Mode",
                        "Your privacy mode is set to read-only. Use `/bcash_privacy` to change."
                    ),
                    ephemeral=True,
                )
                return

            # Defer since this may take a moment
            await interaction.response.defer()

            # Create post
            result = await self.botcash.create_post(identity.botcash_address, content)

            if result.success:
                # Record relayed message
                content_hash = hashlib.sha256(content.encode()).hexdigest()

                relay_record = RelayedMessage(
                    identity_id=identity.id,
                    direction="discord_to_bc",
                    discord_message_id=interaction.id,
                    discord_channel_id=interaction.channel_id,
                    discord_guild_id=interaction.guild_id,
                    botcash_tx_id=result.tx_id,
                    message_type="post",
                    content_hash=content_hash,
                    fee_sponsored=self.config.fees.sponsor_new_users,
                )
                session.add(relay_record)
                await session.commit()

                await interaction.followup.send(
                    embed=create_post_success_embed(result.tx_id, content)
                )
            else:
                await interaction.followup.send(
                    embed=create_error_embed("Post Failed", result.error or "Unknown error")
                )

    @app_commands.command(name="bcash_dm", description="Send an encrypted DM via Botcash")
    @app_commands.describe(
        recipient="Recipient's Botcash address",
        message="Your message (max 450 characters)"
    )
    async def dm(self, interaction: discord.Interaction, recipient: str, message: str) -> None:
        """Send encrypted DM."""
        if not BOTCASH_ADDRESS_PATTERN.match(recipient):
            await interaction.response.send_message(
                embed=create_error_embed("Invalid Address", "Invalid recipient address."),
                ephemeral=True,
            )
            return

        if len(message) > 450:
            await interaction.response.send_message(
                embed=create_error_embed(
                    "Message Too Long",
                    f"Message is {len(message)} characters. Maximum is 450."
                ),
                ephemeral=True,
            )
            return

        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, interaction.user.id)

            if not identity:
                await interaction.response.send_message(
                    embed=create_error_embed("Not Linked", "Use `/bcash_link` first."),
                    ephemeral=True,
                )
                return

            # Check rate limit
            if not await self._check_rate_limit(session, interaction.user.id):
                await interaction.response.send_message(
                    embed=create_error_embed(
                        "Rate Limited",
                        f"Maximum {self.config.discord.rate_limit_messages_per_minute} messages per minute."
                    ),
                    ephemeral=True,
                )
                return

            # Defer since this may take a moment
            await interaction.response.defer(ephemeral=True)

            # Send DM
            result = await self.botcash.send_dm(identity.botcash_address, recipient, message)

            if result.success:
                content_hash = hashlib.sha256(message.encode()).hexdigest()

                relay_record = RelayedMessage(
                    identity_id=identity.id,
                    direction="discord_to_bc",
                    discord_message_id=interaction.id,
                    discord_channel_id=interaction.channel_id,
                    discord_guild_id=interaction.guild_id,
                    botcash_tx_id=result.tx_id,
                    message_type="dm",
                    content_hash=content_hash,
                )
                session.add(relay_record)
                await session.commit()

                await interaction.followup.send(
                    embed=create_dm_success_embed(result.tx_id, recipient),
                    ephemeral=True,
                )
            else:
                await interaction.followup.send(
                    embed=create_error_embed("DM Failed", result.error or "Unknown error"),
                    ephemeral=True,
                )

    @app_commands.command(name="bcash_feed", description="Show recent posts from the Botcash network")
    async def feed(self, interaction: discord.Interaction) -> None:
        """Show recent posts."""
        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, interaction.user.id)

        addresses = []
        if identity:
            addresses.append(identity.botcash_address)

        await interaction.response.defer()

        posts = await self.botcash.get_feed(addresses, limit=5)
        await interaction.followup.send(embed=create_feed_embed(posts))

    @app_commands.command(name="bcash_privacy", description="Set your privacy mode")
    @app_commands.describe(mode="Privacy mode to set")
    @app_commands.choices(mode=[
        app_commands.Choice(name="Full Mirror - All messages synced", value="full_mirror"),
        app_commands.Choice(name="Selective - Only slash commands", value="selective"),
        app_commands.Choice(name="Read Only - View only, no posting", value="read_only"),
        app_commands.Choice(name="Private - DMs only", value="private"),
    ])
    async def privacy(self, interaction: discord.Interaction, mode: str) -> None:
        """Set privacy mode."""
        try:
            privacy_mode = PrivacyMode(mode)
        except ValueError:
            await interaction.response.send_message(
                embed=create_error_embed(
                    "Invalid Mode",
                    "Valid modes: full_mirror, selective, read_only, private"
                ),
                ephemeral=True,
            )
            return

        async with self.session_maker() as session:
            success = await self.identity.set_privacy_mode(
                session, interaction.user.id, privacy_mode
            )

        if success:
            await interaction.response.send_message(
                embed=create_info_embed(
                    "Privacy Mode Updated",
                    f"Privacy mode set to: **{privacy_mode.value}**"
                ),
            )
        else:
            await interaction.response.send_message(
                embed=create_error_embed("Not Linked", "Use `/bcash_link` first."),
                ephemeral=True,
            )

    @app_commands.command(name="bcash_follow", description="Follow a Botcash user")
    @app_commands.describe(address="Botcash address to follow")
    async def follow(self, interaction: discord.Interaction, address: str) -> None:
        """Follow a user."""
        if not BOTCASH_ADDRESS_PATTERN.match(address):
            await interaction.response.send_message(
                embed=create_error_embed("Invalid Address", "Invalid Botcash address."),
                ephemeral=True,
            )
            return

        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, interaction.user.id)

            if not identity:
                await interaction.response.send_message(
                    embed=create_error_embed("Not Linked", "Use `/bcash_link` first."),
                    ephemeral=True,
                )
                return

            await interaction.response.defer()

            result = await self.botcash.follow(identity.botcash_address, address)

            if result.success:
                await interaction.followup.send(
                    embed=create_info_embed(
                        "Following",
                        f"You are now following `{address[:20]}...`\n\n"
                        f"Transaction: [`{result.tx_id[:12]}...`](https://botcash.network/tx/{result.tx_id})"
                    )
                )
            else:
                await interaction.followup.send(
                    embed=create_error_embed("Follow Failed", result.error or "Unknown error")
                )

    @app_commands.command(name="bcash_unfollow", description="Unfollow a Botcash user")
    @app_commands.describe(address="Botcash address to unfollow")
    async def unfollow(self, interaction: discord.Interaction, address: str) -> None:
        """Unfollow a user."""
        if not BOTCASH_ADDRESS_PATTERN.match(address):
            await interaction.response.send_message(
                embed=create_error_embed("Invalid Address", "Invalid Botcash address."),
                ephemeral=True,
            )
            return

        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, interaction.user.id)

            if not identity:
                await interaction.response.send_message(
                    embed=create_error_embed("Not Linked", "Use `/bcash_link` first."),
                    ephemeral=True,
                )
                return

            await interaction.response.defer()

            result = await self.botcash.unfollow(identity.botcash_address, address)

            if result.success:
                await interaction.followup.send(
                    embed=create_info_embed(
                        "Unfollowed",
                        f"You have unfollowed `{address[:20]}...`\n\n"
                        f"Transaction: [`{result.tx_id[:12]}...`](https://botcash.network/tx/{result.tx_id})"
                    )
                )
            else:
                await interaction.followup.send(
                    embed=create_error_embed("Unfollow Failed", result.error or "Unknown error")
                )


async def setup(bot: commands.Bot) -> None:
    """Setup function for the cog (called by bot.load_extension)."""
    # This is a placeholder - actual setup happens in main.py with full dependencies
    pass
