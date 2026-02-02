"""Rich embed formatting for Discord messages."""

from datetime import datetime
from typing import Any, Optional

import discord

# Botcash brand colors
BOTCASH_COLOR_PRIMARY = 0x7B68EE  # Medium slate blue
BOTCASH_COLOR_SUCCESS = 0x2ECC71  # Emerald green
BOTCASH_COLOR_ERROR = 0xE74C3C    # Alizarin red
BOTCASH_COLOR_WARNING = 0xF39C12  # Orange
BOTCASH_COLOR_INFO = 0x3498DB     # Peter river blue

# Emoji icons
EMOJI_BOTCASH = "\U0001F4B8"  # Money with wings
EMOJI_LINK = "\U0001F517"      # Link
EMOJI_POST = "\U0001F4DD"      # Memo
EMOJI_DM = "\U0001F4E8"        # Incoming envelope
EMOJI_BALANCE = "\U0001F4B0"   # Money bag
EMOJI_FEED = "\U0001F4F0"      # Newspaper
EMOJI_CHECK = "\u2705"         # White check mark
EMOJI_CROSS = "\u274C"         # Cross mark
EMOJI_CLOCK = "\U0001F552"     # Clock


def create_welcome_embed() -> discord.Embed:
    """Create welcome embed for /bcash_help command."""
    embed = discord.Embed(
        title=f"{EMOJI_BOTCASH} Botcash Discord Bridge",
        description=(
            "Connect your Discord account to the Botcash decentralized social network.\n\n"
            "Botcash combines privacy-preserving cryptocurrency with social features, "
            "enabling censorship-resistant communication with economic incentives."
        ),
        color=BOTCASH_COLOR_PRIMARY,
    )

    embed.add_field(
        name=f"{EMOJI_LINK} Identity",
        value=(
            "`/bcash_link` - Link your Botcash address\n"
            "`/bcash_verify` - Complete linking with signature\n"
            "`/bcash_unlink` - Remove link\n"
            "`/bcash_status` - Check link status"
        ),
        inline=True,
    )

    embed.add_field(
        name=f"{EMOJI_POST} Social",
        value=(
            "`/bcash_post` - Post to Botcash\n"
            "`/bcash_dm` - Send encrypted DM\n"
            "`/bcash_feed` - Show recent posts\n"
            "`/bcash_follow` - Follow a user"
        ),
        inline=True,
    )

    embed.add_field(
        name=f"{EMOJI_BALANCE} Account",
        value=(
            "`/bcash_balance` - Check BCASH balance\n"
            "`/bcash_privacy` - Set privacy mode"
        ),
        inline=True,
    )

    embed.set_footer(text="Start by linking your address with /bcash_link")
    return embed


def create_link_initiated_embed(challenge: str, address: str) -> discord.Embed:
    """Create embed for link initiation."""
    embed = discord.Embed(
        title=f"{EMOJI_LINK} Link Initiated",
        description="Sign the message below with your Botcash wallet to verify ownership.",
        color=BOTCASH_COLOR_INFO,
    )

    embed.add_field(
        name="Address",
        value=f"`{address[:24]}...`",
        inline=False,
    )

    embed.add_field(
        name="Message to Sign",
        value=f"```\n{challenge[:64]}...\n```",
        inline=False,
    )

    embed.add_field(
        name="Next Step",
        value="Use `/bcash_verify signature:<your_signature>` to complete linking.",
        inline=False,
    )

    embed.set_footer(text=f"{EMOJI_CLOCK} Challenge expires in 10 minutes")
    return embed


def create_link_complete_embed(
    discord_user: discord.User,
    botcash_address: str,
    tx_id: str,
) -> discord.Embed:
    """Create embed for successful link completion."""
    embed = discord.Embed(
        title=f"{EMOJI_CHECK} Link Complete!",
        description="Your Discord account is now linked to your Botcash address.",
        color=BOTCASH_COLOR_SUCCESS,
    )

    embed.add_field(
        name="Discord",
        value=f"{discord_user.mention}",
        inline=True,
    )

    embed.add_field(
        name="Botcash",
        value=f"`{botcash_address[:20]}...`",
        inline=True,
    )

    embed.add_field(
        name="Transaction",
        value=f"[`{tx_id[:12]}...`](https://botcash.network/tx/{tx_id})",
        inline=False,
    )

    embed.set_footer(text="You can now use /bcash_post to post to Botcash!")
    return embed


def create_unlink_embed() -> discord.Embed:
    """Create embed for successful unlink."""
    embed = discord.Embed(
        title=f"{EMOJI_CHECK} Account Unlinked",
        description="Your account has been unlinked from Botcash.",
        color=BOTCASH_COLOR_SUCCESS,
    )
    embed.set_footer(text="Use /bcash_link to link a new address.")
    return embed


def create_status_embed(
    discord_user: discord.User,
    botcash_address: str,
    balance: float,
    privacy_mode: str,
    linked_at: Optional[datetime],
) -> discord.Embed:
    """Create embed for link status."""
    embed = discord.Embed(
        title=f"{EMOJI_LINK} Link Status",
        color=BOTCASH_COLOR_PRIMARY,
    )

    embed.add_field(
        name="Discord",
        value=f"{discord_user.mention}",
        inline=True,
    )

    embed.add_field(
        name="Botcash",
        value=f"`{botcash_address[:24]}...`",
        inline=True,
    )

    embed.add_field(
        name=f"{EMOJI_BALANCE} Balance",
        value=f"{balance:.8f} BCASH",
        inline=True,
    )

    embed.add_field(
        name="Privacy Mode",
        value=f"`{privacy_mode}`",
        inline=True,
    )

    if linked_at:
        embed.add_field(
            name="Linked Since",
            value=f"<t:{int(linked_at.timestamp())}:R>",
            inline=True,
        )

    return embed


def create_balance_embed(
    botcash_address: str,
    confirmed: float,
    pending: float,
) -> discord.Embed:
    """Create embed for balance display."""
    embed = discord.Embed(
        title=f"{EMOJI_BALANCE} Balance",
        description=f"Balance for `{botcash_address[:20]}...`",
        color=BOTCASH_COLOR_PRIMARY,
    )

    embed.add_field(
        name="Confirmed",
        value=f"**{confirmed:.8f}** BCASH",
        inline=True,
    )

    embed.add_field(
        name="Pending",
        value=f"{pending:.8f} BCASH",
        inline=True,
    )

    embed.add_field(
        name="Total",
        value=f"**{confirmed + pending:.8f}** BCASH",
        inline=True,
    )

    return embed


def create_post_success_embed(tx_id: str, content: str) -> discord.Embed:
    """Create embed for successful post."""
    # Truncate content if too long
    display_content = content[:200] + "..." if len(content) > 200 else content

    embed = discord.Embed(
        title=f"{EMOJI_CHECK} Posted to Botcash",
        description=display_content,
        color=BOTCASH_COLOR_SUCCESS,
    )

    embed.add_field(
        name="Transaction",
        value=f"[`{tx_id[:12]}...`](https://botcash.network/tx/{tx_id})",
        inline=False,
    )

    return embed


def create_dm_success_embed(tx_id: str, recipient: str) -> discord.Embed:
    """Create embed for successful DM."""
    embed = discord.Embed(
        title=f"{EMOJI_CHECK} DM Sent",
        description=f"Encrypted message sent to `{recipient[:20]}...`",
        color=BOTCASH_COLOR_SUCCESS,
    )

    embed.add_field(
        name="Transaction",
        value=f"[`{tx_id[:12]}...`](https://botcash.network/tx/{tx_id})",
        inline=False,
    )

    return embed


def create_feed_embed(posts: list[dict[str, Any]]) -> discord.Embed:
    """Create embed for feed display."""
    embed = discord.Embed(
        title=f"{EMOJI_FEED} Recent Posts",
        color=BOTCASH_COLOR_PRIMARY,
    )

    if not posts:
        embed.description = "No posts found. Link your address and follow users to see their posts."
        return embed

    for post in posts[:5]:
        author = post.get("author", "Unknown")[:16]
        content = post.get("content", "")[:100]
        tx_id = post.get("tx_id", "")[:8]

        embed.add_field(
            name=f"`{author}...`",
            value=f"{content}\n*TX: {tx_id}...*",
            inline=False,
        )

    return embed


def create_error_embed(title: str, description: str) -> discord.Embed:
    """Create error embed."""
    return discord.Embed(
        title=f"{EMOJI_CROSS} {title}",
        description=description,
        color=BOTCASH_COLOR_ERROR,
    )


def create_warning_embed(title: str, description: str) -> discord.Embed:
    """Create warning embed."""
    return discord.Embed(
        title=f"\u26A0\uFE0F {title}",
        description=description,
        color=BOTCASH_COLOR_WARNING,
    )


def create_info_embed(title: str, description: str) -> discord.Embed:
    """Create info embed."""
    return discord.Embed(
        title=title,
        description=description,
        color=BOTCASH_COLOR_INFO,
    )


def create_bridged_post_embed(
    author_address: str,
    content: str,
    tx_id: str,
    timestamp: Optional[datetime] = None,
) -> discord.Embed:
    """Create embed for a post bridged from Botcash to Discord."""
    embed = discord.Embed(
        description=content,
        color=BOTCASH_COLOR_PRIMARY,
    )

    embed.set_author(
        name=f"{author_address[:16]}...",
        url=f"https://botcash.network/address/{author_address}",
    )

    embed.add_field(
        name="Transaction",
        value=f"[View on Botcash](https://botcash.network/tx/{tx_id})",
        inline=True,
    )

    if timestamp:
        embed.timestamp = timestamp

    embed.set_footer(text="Bridged from Botcash Network")

    return embed
