"""Telegram bot command handlers for Botcash bridge."""

import re
from datetime import datetime, timedelta, timezone

import structlog
from sqlalchemy import func, select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker
from telegram import Update
from telegram.constants import ParseMode
from telegram.ext import ContextTypes

from .botcash_client import BotcashClient
from .config import BridgeConfig
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


class BotHandlers:
    """Handler class for Telegram bot commands."""

    def __init__(
        self,
        config: BridgeConfig,
        session_maker: async_sessionmaker[AsyncSession],
        botcash_client: BotcashClient,
        identity_service: IdentityService,
    ):
        """Initialize handlers.

        Args:
            config: Bridge configuration
            session_maker: Database session factory
            botcash_client: Botcash RPC client
            identity_service: Identity linking service
        """
        self.config = config
        self.session_maker = session_maker
        self.botcash = botcash_client
        self.identity = identity_service

    async def _check_rate_limit(
        self,
        session: AsyncSession,
        telegram_user_id: int,
    ) -> bool:
        """Check if user is within rate limit.

        Args:
            session: Database session
            telegram_user_id: Telegram user ID

        Returns:
            True if within limit, False if exceeded
        """
        window_start = datetime.now(timezone.utc).replace(second=0, microsecond=0)

        result = await session.execute(
            select(RateLimitEntry).where(
                RateLimitEntry.telegram_user_id == telegram_user_id,
                RateLimitEntry.window_start == window_start,
            )
        )
        entry = result.scalar_one_or_none()

        if entry:
            if entry.message_count >= self.config.telegram.rate_limit_messages_per_minute:
                return False
            entry.message_count += 1
        else:
            entry = RateLimitEntry(
                telegram_user_id=telegram_user_id,
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

    async def start(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /start command."""
        if not update.effective_user or not update.message:
            return

        welcome_msg = (
            "*Welcome to the Botcash Bridge!*\n\n"
            "This bot connects your Telegram account to the Botcash decentralized social network.\n\n"
            "*Commands:*\n"
            "`/link <address>` - Link your Botcash address\n"
            "`/unlink` - Remove link\n"
            "`/post <content>` - Post to Botcash\n"
            "`/dm @user <message>` - Send DM via Botcash\n"
            "`/balance` - Check your BCASH balance\n"
            "`/feed` - Show recent posts\n"
            "`/privacy <mode>` - Set privacy mode\n"
            "`/status` - Check link status\n\n"
            "_Start by linking your address with_ `/link bs1...`"
        )
        await update.message.reply_text(welcome_msg, parse_mode=ParseMode.MARKDOWN)

    async def help_command(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /help command."""
        if not update.message:
            return

        help_msg = (
            "*Botcash Bridge Commands*\n\n"
            "*Identity:*\n"
            "`/link <address>` - Start linking process\n"
            "`/verify <signature>` - Complete linking with signature\n"
            "`/unlink` - Unlink your account\n"
            "`/status` - Check link status\n\n"
            "*Social:*\n"
            "`/post <content>` - Post to Botcash\n"
            "`/dm @user <msg>` - Send encrypted DM\n"
            "`/feed` - Show recent posts\n\n"
            "*Account:*\n"
            "`/balance` - Check BCASH balance\n"
            "`/privacy <mode>` - Set privacy mode\n\n"
            "*Privacy Modes:*\n"
            "- `full_mirror` - All messages synced\n"
            "- `selective` - Only /post commands\n"
            "- `read_only` - View only, no posting\n"
            "- `private` - DMs only"
        )
        await update.message.reply_text(help_msg, parse_mode=ParseMode.MARKDOWN)

    async def link(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /link command to start identity linking."""
        if not update.effective_user or not update.message:
            return

        if not context.args or len(context.args) != 1:
            await update.message.reply_text(
                "Usage: `/link <botcash_address>`\n\n"
                "Example: `/link bs1qz7s5p...`",
                parse_mode=ParseMode.MARKDOWN,
            )
            return

        address = context.args[0]

        # Validate address format
        if not BOTCASH_ADDRESS_PATTERN.match(address):
            await update.message.reply_text(
                "Invalid address format.\n\n"
                "Supported formats:\n"
                "- Shielded: `bs1...` (Sapling) or `bu1...` (Unified)\n"
                "- Transparent: `B1...` or `B3...`",
                parse_mode=ParseMode.MARKDOWN,
            )
            return

        async with self.session_maker() as session:
            try:
                challenge, verification_msg = await self.identity.initiate_link(
                    session,
                    telegram_user_id=update.effective_user.id,
                    telegram_username=update.effective_user.username,
                    botcash_address=address,
                )

                response = (
                    "*Link Initiated*\n\n"
                    "To complete linking, sign this message with your Botcash wallet:\n\n"
                    f"`{verification_msg}`\n\n"
                    "Then send the signature using:\n"
                    "`/verify <signature>`\n\n"
                    f"_Challenge expires in 10 minutes._"
                )
                await update.message.reply_text(response, parse_mode=ParseMode.MARKDOWN)

            except IdentityLinkError as e:
                await update.message.reply_text(f"Link failed: {e}")

    async def verify(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /verify command to complete identity linking."""
        if not update.effective_user or not update.message:
            return

        if not context.args or len(context.args) != 1:
            await update.message.reply_text(
                "Usage: `/verify <signature>`\n\n"
                "Provide the signature from your Botcash wallet.",
                parse_mode=ParseMode.MARKDOWN,
            )
            return

        signature = context.args[0]

        async with self.session_maker() as session:
            try:
                identity = await self.identity.complete_link(
                    session,
                    telegram_user_id=update.effective_user.id,
                    signature=signature,
                )

                response = (
                    "*Link Complete!*\n\n"
                    f"Telegram: @{update.effective_user.username or update.effective_user.id}\n"
                    f"Botcash: `{identity.botcash_address[:16]}...`\n"
                    f"TX: `{identity.link_tx_id[:16]}...`\n\n"
                    "You can now use `/post` to post to Botcash!"
                )
                await update.message.reply_text(response, parse_mode=ParseMode.MARKDOWN)

            except IdentityLinkError as e:
                await update.message.reply_text(f"Verification failed: {e}")

    async def unlink(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /unlink command."""
        if not update.effective_user or not update.message:
            return

        async with self.session_maker() as session:
            success = await self.identity.unlink(session, update.effective_user.id)

        if success:
            await update.message.reply_text(
                "Your account has been unlinked. Use `/link` to link a new address.",
                parse_mode=ParseMode.MARKDOWN,
            )
        else:
            await update.message.reply_text("No linked account found.")

    async def status(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /status command."""
        if not update.effective_user or not update.message:
            return

        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, update.effective_user.id)

        if not identity:
            await update.message.reply_text(
                "No linked account.\nUse `/link <address>` to get started.",
                parse_mode=ParseMode.MARKDOWN,
            )
            return

        # Get balance
        try:
            balance = await self.botcash.get_balance(identity.botcash_address)
            balance_str = f"{balance.confirmed_bcash:.8f} BCASH"
        except Exception:
            balance_str = "Unknown"

        response = (
            "*Link Status*\n\n"
            f"Telegram: @{identity.telegram_username or identity.telegram_user_id}\n"
            f"Botcash: `{identity.botcash_address[:24]}...`\n"
            f"Balance: {balance_str}\n"
            f"Privacy: {identity.privacy_mode.value}\n"
            f"Linked: {identity.linked_at.strftime('%Y-%m-%d') if identity.linked_at else 'N/A'}"
        )
        await update.message.reply_text(response, parse_mode=ParseMode.MARKDOWN)

    async def balance(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /balance command."""
        if not update.effective_user or not update.message:
            return

        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, update.effective_user.id)

        if not identity:
            await update.message.reply_text(
                "No linked account. Use `/link` first.",
                parse_mode=ParseMode.MARKDOWN,
            )
            return

        try:
            balance = await self.botcash.get_balance(identity.botcash_address)
            response = (
                f"*Balance for* `{identity.botcash_address[:16]}...`\n\n"
                f"Confirmed: {balance.confirmed_bcash:.8f} BCASH\n"
                f"Pending: {balance.pending / 100_000_000:.8f} BCASH"
            )
            await update.message.reply_text(response, parse_mode=ParseMode.MARKDOWN)
        except Exception as e:
            logger.error("Failed to get balance", error=str(e))
            await update.message.reply_text("Failed to retrieve balance. Please try again.")

    async def post(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /post command to create a Botcash post."""
        if not update.effective_user or not update.message:
            return

        if not context.args:
            await update.message.reply_text(
                "Usage: `/post <content>`\n\n"
                "Example: `/post Hello Botcash!`",
                parse_mode=ParseMode.MARKDOWN,
            )
            return

        content = " ".join(context.args)
        if len(content) > 450:
            await update.message.reply_text(
                f"Post too long ({len(content)} chars). Max 450 characters."
            )
            return

        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, update.effective_user.id)

            if not identity:
                await update.message.reply_text(
                    "No linked account. Use `/link` first.",
                    parse_mode=ParseMode.MARKDOWN,
                )
                return

            # Check rate limit
            if not await self._check_rate_limit(session, update.effective_user.id):
                await update.message.reply_text(
                    f"Rate limit exceeded. Max {self.config.telegram.rate_limit_messages_per_minute} messages/minute."
                )
                return

            # Check privacy mode
            if identity.privacy_mode == PrivacyMode.READ_ONLY:
                await update.message.reply_text(
                    "Your privacy mode is set to read-only. Use `/privacy selective` to enable posting."
                )
                return

            # Create post
            result = await self.botcash.create_post(identity.botcash_address, content)

            if result.success:
                # Record relayed message
                import hashlib
                content_hash = hashlib.sha256(content.encode()).hexdigest()

                relay_record = RelayedMessage(
                    identity_id=identity.id,
                    direction="tg_to_bc",
                    telegram_message_id=update.message.message_id,
                    telegram_chat_id=update.message.chat_id,
                    botcash_tx_id=result.tx_id,
                    message_type="post",
                    content_hash=content_hash,
                    fee_sponsored=self.config.fees.sponsor_new_users,
                )
                session.add(relay_record)
                await session.commit()

                response = (
                    "*Posted to Botcash!*\n\n"
                    f"TX: `{result.tx_id[:16]}...`\n"
                    f"View: botcash.network/tx/{result.tx_id}"
                )
                await update.message.reply_text(response, parse_mode=ParseMode.MARKDOWN)
            else:
                await update.message.reply_text(f"Failed to post: {result.error}")

    async def dm(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /dm command to send encrypted DM."""
        if not update.effective_user or not update.message:
            return

        if not context.args or len(context.args) < 2:
            await update.message.reply_text(
                "Usage: `/dm <address> <message>`\n\n"
                "Example: `/dm bs1qz7s5p... Hello there!`",
                parse_mode=ParseMode.MARKDOWN,
            )
            return

        to_address = context.args[0]
        content = " ".join(context.args[1:])

        if not BOTCASH_ADDRESS_PATTERN.match(to_address):
            await update.message.reply_text("Invalid recipient address.")
            return

        if len(content) > 450:
            await update.message.reply_text(
                f"Message too long ({len(content)} chars). Max 450 characters."
            )
            return

        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, update.effective_user.id)

            if not identity:
                await update.message.reply_text(
                    "No linked account. Use `/link` first.",
                    parse_mode=ParseMode.MARKDOWN,
                )
                return

            # Check rate limit
            if not await self._check_rate_limit(session, update.effective_user.id):
                await update.message.reply_text(
                    f"Rate limit exceeded. Max {self.config.telegram.rate_limit_messages_per_minute} messages/minute."
                )
                return

            # Send DM
            result = await self.botcash.send_dm(identity.botcash_address, to_address, content)

            if result.success:
                import hashlib
                content_hash = hashlib.sha256(content.encode()).hexdigest()

                relay_record = RelayedMessage(
                    identity_id=identity.id,
                    direction="tg_to_bc",
                    telegram_message_id=update.message.message_id,
                    telegram_chat_id=update.message.chat_id,
                    botcash_tx_id=result.tx_id,
                    message_type="dm",
                    content_hash=content_hash,
                )
                session.add(relay_record)
                await session.commit()

                response = (
                    "*DM Sent!*\n\n"
                    f"To: `{to_address[:16]}...`\n"
                    f"TX: `{result.tx_id[:16]}...`"
                )
                await update.message.reply_text(response, parse_mode=ParseMode.MARKDOWN)
            else:
                await update.message.reply_text(f"Failed to send DM: {result.error}")

    async def feed(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /feed command to show recent posts."""
        if not update.effective_user or not update.message:
            return

        async with self.session_maker() as session:
            identity = await self.identity.get_linked_identity(session, update.effective_user.id)

        addresses = []
        if identity:
            addresses.append(identity.botcash_address)

        posts = await self.botcash.get_feed(addresses, limit=5)

        if not posts:
            await update.message.reply_text(
                "No posts found. Link your address and follow users to see their posts."
            )
            return

        response = "*Recent Posts*\n\n"
        for post in posts[:5]:
            author = post.get("author", "Unknown")[:12]
            content = post.get("content", "")[:100]
            tx_id = post.get("tx_id", "")[:8]
            response += f"`{author}...`: {content}\n_TX: {tx_id}..._\n\n"

        await update.message.reply_text(response, parse_mode=ParseMode.MARKDOWN)

    async def privacy(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /privacy command to set privacy mode."""
        if not update.effective_user or not update.message:
            return

        if not context.args or len(context.args) != 1:
            await update.message.reply_text(
                "Usage: `/privacy <mode>`\n\n"
                "Modes:\n"
                "- `full_mirror` - All messages synced\n"
                "- `selective` - Only /post commands\n"
                "- `read_only` - View only, no posting\n"
                "- `private` - DMs only",
                parse_mode=ParseMode.MARKDOWN,
            )
            return

        mode_str = context.args[0].lower()
        try:
            mode = PrivacyMode(mode_str)
        except ValueError:
            await update.message.reply_text(
                f"Invalid mode: `{mode_str}`\n\n"
                "Valid modes: full_mirror, selective, read_only, private",
                parse_mode=ParseMode.MARKDOWN,
            )
            return

        async with self.session_maker() as session:
            success = await self.identity.set_privacy_mode(
                session, update.effective_user.id, mode
            )

        if success:
            await update.message.reply_text(f"Privacy mode set to: *{mode.value}*", parse_mode=ParseMode.MARKDOWN)
        else:
            await update.message.reply_text(
                "No linked account. Use `/link` first.",
                parse_mode=ParseMode.MARKDOWN,
            )
