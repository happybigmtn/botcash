# Botcash Telegram Bridge

Bidirectional bridge between Telegram and the Botcash decentralized social network.

## Features

- **Identity Linking**: Link your Telegram account to your Botcash address with cryptographic verification
- **Post Relay**: Post to Botcash directly from Telegram using `/post`
- **DM Relay**: Send encrypted DMs via Botcash using `/dm`
- **Balance Check**: Check your BCASH balance
- **Feed Viewing**: View recent posts from your feed
- **Privacy Modes**: Control what gets synced between platforms

## Installation

```bash
# Install with pip
pip install -e .

# Or with uv (recommended)
uv pip install -e .

# For development
pip install -e ".[dev]"
```

## Configuration

Create a `.env` file or set environment variables:

```env
# Required
TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrsTUVwxyz

# Botcash node (optional, defaults shown)
BOTCASH_RPC_URL=http://localhost:8532
BOTCASH_RPC_USER=
BOTCASH_RPC_PASSWORD=
BOTCASH_INDEXER_URL=http://localhost:9067
BOTCASH_BRIDGE_ADDRESS=bs1...

# Database (optional)
DB_URL=sqlite+aiosqlite:///botcash_bridge.db

# Fees (optional)
FEE_SPONSOR_NEW_USERS=true
FEE_MAX_SPONSORED_PER_DAY=100
```

## Running

```bash
# Start the bot
botcash-telegram

# Or directly
python -m botcash_telegram.main
```

## Commands

| Command | Description |
|---------|-------------|
| `/start` | Welcome message and instructions |
| `/help` | Show all commands |
| `/link <address>` | Start linking your Botcash address |
| `/verify <signature>` | Complete linking with wallet signature |
| `/unlink` | Remove your linked account |
| `/status` | Check your link status and balance |
| `/balance` | Check your BCASH balance |
| `/post <content>` | Post to Botcash |
| `/dm <address> <message>` | Send encrypted DM |
| `/feed` | Show recent posts |
| `/privacy <mode>` | Set privacy mode |

## Privacy Modes

| Mode | Telegram -> Botcash | Botcash -> Telegram |
|------|---------------------|---------------------|
| `full_mirror` | All messages | All posts |
| `selective` | Only `/post` commands | Opted-in posts |
| `read_only` | None | All posts |
| `private` | DMs only | DMs only |

## Testing

```bash
# Run all tests
pytest

# With coverage
pytest --cov=botcash_telegram

# Run specific test file
pytest tests/test_identity.py
```

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    TELEGRAM BRIDGE                        │
│                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │   Handlers  │  │  Identity   │  │  Botcash    │      │
│  │             │──│  Service    │──│  Client     │      │
│  │ /link       │  │             │  │             │      │
│  │ /post       │  │ Link/Unlink │  │ RPC calls   │      │
│  │ /dm         │  │ Verify      │  │ Post/DM     │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
│         │                │                │              │
│         │         ┌──────────────┐       │              │
│         └─────────│  SQLite DB   │───────┘              │
│                   │              │                       │
│                   │ Identities   │                       │
│                   │ Messages     │                       │
│                   │ Rate Limits  │                       │
│                   └──────────────┘                       │
└──────────────────────────────────────────────────────────┘
```

## License

MIT License - See LICENSE file.
