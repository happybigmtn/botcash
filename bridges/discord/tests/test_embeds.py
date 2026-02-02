"""Tests for Discord bridge embed formatting."""

from datetime import datetime, timezone
from unittest.mock import MagicMock

import discord

from botcash_discord.embeds import (
    BOTCASH_COLOR_ERROR,
    BOTCASH_COLOR_INFO,
    BOTCASH_COLOR_PRIMARY,
    BOTCASH_COLOR_SUCCESS,
    BOTCASH_COLOR_WARNING,
    create_balance_embed,
    create_bridged_post_embed,
    create_dm_success_embed,
    create_error_embed,
    create_feed_embed,
    create_info_embed,
    create_link_complete_embed,
    create_link_initiated_embed,
    create_post_success_embed,
    create_status_embed,
    create_unlink_embed,
    create_warning_embed,
    create_welcome_embed,
)


class TestColors:
    """Tests for embed colors."""

    def test_colors_are_valid_hex(self):
        """Test that all colors are valid hex values."""
        colors = [
            BOTCASH_COLOR_PRIMARY,
            BOTCASH_COLOR_SUCCESS,
            BOTCASH_COLOR_ERROR,
            BOTCASH_COLOR_WARNING,
            BOTCASH_COLOR_INFO,
        ]

        for color in colors:
            assert isinstance(color, int)
            assert 0 <= color <= 0xFFFFFF


class TestWelcomeEmbed:
    """Tests for welcome embed."""

    def test_welcome_embed_structure(self):
        """Test welcome embed has correct structure."""
        embed = create_welcome_embed()

        assert isinstance(embed, discord.Embed)
        assert embed.title is not None
        assert "Botcash" in embed.title
        assert embed.description is not None
        assert len(embed.fields) >= 3  # Identity, Social, Account

    def test_welcome_embed_has_commands(self):
        """Test welcome embed lists commands."""
        embed = create_welcome_embed()

        # Check fields contain command references
        field_values = " ".join(f.value for f in embed.fields)
        assert "/bcash_link" in field_values
        assert "/bcash_post" in field_values
        assert "/bcash_balance" in field_values


class TestLinkEmbeds:
    """Tests for link-related embeds."""

    def test_link_initiated_embed(self):
        """Test link initiated embed."""
        embed = create_link_initiated_embed(
            challenge="a" * 64,
            address="bs1" + "a" * 59,
        )

        assert "Link" in embed.title or "Link" in str(embed.description)
        assert embed.color.value == BOTCASH_COLOR_INFO

    def test_link_complete_embed(self):
        """Test link complete embed."""
        mock_user = MagicMock(spec=discord.User)
        mock_user.mention = "<@123456789>"

        embed = create_link_complete_embed(
            discord_user=mock_user,
            botcash_address="bs1" + "a" * 59,
            tx_id="abc123" * 10,
        )

        assert "Complete" in embed.title
        assert embed.color.value == BOTCASH_COLOR_SUCCESS
        assert len(embed.fields) >= 2  # Discord, Botcash

    def test_unlink_embed(self):
        """Test unlink embed."""
        embed = create_unlink_embed()

        assert "Unlinked" in embed.title
        assert embed.color.value == BOTCASH_COLOR_SUCCESS


class TestStatusEmbed:
    """Tests for status embed."""

    def test_status_embed_with_linked_at(self):
        """Test status embed with linked_at timestamp."""
        mock_user = MagicMock(spec=discord.User)
        mock_user.mention = "<@123456789>"

        embed = create_status_embed(
            discord_user=mock_user,
            botcash_address="bs1" + "a" * 59,
            balance=1.5,
            privacy_mode="selective",
            linked_at=datetime.now(timezone.utc),
        )

        assert "Status" in embed.title
        assert embed.color.value == BOTCASH_COLOR_PRIMARY
        assert len(embed.fields) >= 4  # Discord, Botcash, Balance, Privacy

    def test_status_embed_without_linked_at(self):
        """Test status embed without linked_at timestamp."""
        mock_user = MagicMock(spec=discord.User)
        mock_user.mention = "<@123456789>"

        embed = create_status_embed(
            discord_user=mock_user,
            botcash_address="bs1" + "a" * 59,
            balance=0.0,
            privacy_mode="read_only",
            linked_at=None,
        )

        assert isinstance(embed, discord.Embed)


class TestBalanceEmbed:
    """Tests for balance embed."""

    def test_balance_embed(self):
        """Test balance embed structure."""
        embed = create_balance_embed(
            botcash_address="bs1" + "a" * 59,
            confirmed=1.5,
            pending=0.25,
        )

        assert "Balance" in embed.title
        assert embed.color.value == BOTCASH_COLOR_PRIMARY
        assert len(embed.fields) >= 3  # Confirmed, Pending, Total

        # Check values are displayed
        field_values = " ".join(f.value for f in embed.fields)
        assert "1.5" in field_values or "1.50" in field_values
        assert "0.25" in field_values

    def test_balance_embed_zero(self):
        """Test balance embed with zero balance."""
        embed = create_balance_embed(
            botcash_address="bs1" + "a" * 59,
            confirmed=0.0,
            pending=0.0,
        )

        assert isinstance(embed, discord.Embed)
        field_values = " ".join(f.value for f in embed.fields)
        assert "0" in field_values


class TestPostEmbeds:
    """Tests for post-related embeds."""

    def test_post_success_embed(self):
        """Test post success embed."""
        embed = create_post_success_embed(
            tx_id="abc123" * 10,
            content="Hello Botcash!",
        )

        assert "Posted" in embed.title
        assert embed.color.value == BOTCASH_COLOR_SUCCESS
        assert "Hello Botcash!" in embed.description

    def test_post_success_embed_long_content(self):
        """Test post success embed truncates long content."""
        long_content = "A" * 500

        embed = create_post_success_embed(
            tx_id="abc123" * 10,
            content=long_content,
        )

        # Content should be truncated
        assert len(embed.description) < len(long_content)
        assert "..." in embed.description

    def test_dm_success_embed(self):
        """Test DM success embed."""
        embed = create_dm_success_embed(
            tx_id="def456" * 10,
            recipient="bs1" + "a" * 59,
        )

        assert "DM" in embed.title
        assert embed.color.value == BOTCASH_COLOR_SUCCESS


class TestFeedEmbed:
    """Tests for feed embed."""

    def test_feed_embed_with_posts(self):
        """Test feed embed with posts."""
        posts = [
            {"author": "bs1author1...", "content": "First post", "tx_id": "tx1"},
            {"author": "bs1author2...", "content": "Second post", "tx_id": "tx2"},
        ]

        embed = create_feed_embed(posts)

        assert "Posts" in embed.title
        assert len(embed.fields) == 2

    def test_feed_embed_empty(self):
        """Test feed embed with no posts."""
        embed = create_feed_embed([])

        assert "No posts found" in embed.description

    def test_feed_embed_limits_posts(self):
        """Test feed embed limits to 5 posts."""
        posts = [
            {"author": f"author{i}", "content": f"Post {i}", "tx_id": f"tx{i}"}
            for i in range(10)
        ]

        embed = create_feed_embed(posts)

        assert len(embed.fields) == 5


class TestErrorEmbeds:
    """Tests for error and warning embeds."""

    def test_error_embed(self):
        """Test error embed."""
        embed = create_error_embed("Test Error", "Something went wrong")

        assert "Test Error" in embed.title
        assert "Something went wrong" in embed.description
        assert embed.color.value == BOTCASH_COLOR_ERROR

    def test_warning_embed(self):
        """Test warning embed."""
        embed = create_warning_embed("Test Warning", "Be careful")

        assert "Test Warning" in embed.title
        assert "Be careful" in embed.description
        assert embed.color.value == BOTCASH_COLOR_WARNING

    def test_info_embed(self):
        """Test info embed."""
        embed = create_info_embed("Test Info", "Here's some info")

        assert "Test Info" in embed.title
        assert "Here's some info" in embed.description
        assert embed.color.value == BOTCASH_COLOR_INFO


class TestBridgedPostEmbed:
    """Tests for bridged post embed."""

    def test_bridged_post_embed(self):
        """Test bridged post embed."""
        embed = create_bridged_post_embed(
            author_address="bs1" + "a" * 59,
            content="Hello from Botcash!",
            tx_id="abc123" * 10,
            timestamp=datetime.now(timezone.utc),
        )

        assert embed.description == "Hello from Botcash!"
        assert embed.color.value == BOTCASH_COLOR_PRIMARY
        assert "Bridged from Botcash" in embed.footer.text

    def test_bridged_post_embed_no_timestamp(self):
        """Test bridged post embed without timestamp."""
        embed = create_bridged_post_embed(
            author_address="bs1" + "a" * 59,
            content="Hello!",
            tx_id="abc123" * 10,
            timestamp=None,
        )

        assert isinstance(embed, discord.Embed)
        assert embed.timestamp is None
