# Botcash X/Twitter Bridge

Cross-posting bridge from Botcash to X/Twitter. This bridge operates in **read-only mode** due to X API restrictions.

## Features

- **Botcash → X:** Cross-post Botcash posts to Twitter
- **OAuth Authorization:** Users authorize their Twitter account
- **Rate Limiting:** Respects Twitter API rate limits
- **Attribution:** Posts include Botcash attribution and link

## Limitations

Due to X API restrictions, this bridge:
- Does **not** import tweets into Botcash
- Does **not** verify Twitter account ownership for Botcash linking
- Is **rate-limited** by Twitter's API policies
- Requires **Twitter Developer API access**

## Installation

```bash
pip install -e ".[dev]"
```

## Configuration

Set environment variables or create a `.env` file:

```env
# Botcash node
BOTCASH_RPC_URL=http://localhost:8532
BOTCASH_BRIDGE_ADDRESS=bs1...

# Twitter API (v2)
TWITTER_API_KEY=your_api_key
TWITTER_API_SECRET=your_api_secret
TWITTER_BEARER_TOKEN=your_bearer_token

# For user authentication (OAuth 2.0)
TWITTER_CLIENT_ID=your_client_id
TWITTER_CLIENT_SECRET=your_client_secret

# Database
DB_URL=sqlite+aiosqlite:///botcash_twitter_bridge.db
```

## Usage

```bash
# Start the bridge service
botcash-twitter

# Or with custom config
botcash-twitter --config config.yaml
```

## Cross-Post Format

Posts to Twitter include:

```
[Your Botcash post content]

🔗 bcash.network/post/[txid]
#Botcash
```

## API Endpoints

The bridge exposes a simple HTTP API:

- `POST /link` - Start OAuth flow to link Twitter account
- `GET /callback` - Twitter OAuth callback
- `POST /unlink` - Remove Twitter link
- `GET /status` - Check link status
- `POST /crosspost` - Manually trigger cross-post

## Privacy Modes

| Mode | Botcash → Twitter |
|------|------------------|
| Full Mirror | All posts |
| Selective | Opt-in posts only |
| Disabled | No cross-posting |

## Testing

```bash
pytest tests/ -v
```

## License

MIT License
