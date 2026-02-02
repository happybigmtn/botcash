"""Botcash JSON-RPC client for X/Twitter bridge operations."""

import hashlib
import secrets
from dataclasses import dataclass
from typing import Any

import aiohttp
import structlog

logger = structlog.get_logger()


@dataclass
class PostResult:
    """Result of creating a social post."""
    tx_id: str
    success: bool
    error: str | None = None


@dataclass
class Balance:
    """Botcash address balance."""
    address: str
    confirmed: int  # zatoshis
    pending: int    # zatoshis

    @property
    def confirmed_bcash(self) -> float:
        """Get confirmed balance in BCASH."""
        return self.confirmed / 100_000_000

    @property
    def total_bcash(self) -> float:
        """Get total balance (confirmed + pending) in BCASH."""
        return (self.confirmed + self.pending) / 100_000_000


class BotcashRpcError(Exception):
    """Error from Botcash RPC."""

    def __init__(self, code: int, message: str):
        self.code = code
        self.message = message
        super().__init__(f"RPC Error {code}: {message}")


class BotcashClient:
    """Client for interacting with Botcash node via JSON-RPC."""

    def __init__(
        self,
        rpc_url: str,
        rpc_user: str = "",
        rpc_password: str = "",
        bridge_address: str = "",
    ):
        """Initialize Botcash client.

        Args:
            rpc_url: URL of the Botcash JSON-RPC endpoint
            rpc_user: RPC username (if auth enabled)
            rpc_password: RPC password (if auth enabled)
            bridge_address: Botcash address for bridge-sponsored transactions
        """
        self.rpc_url = rpc_url
        self.rpc_user = rpc_user
        self.rpc_password = rpc_password
        self.bridge_address = bridge_address
        self._request_id = 0
        self._session: aiohttp.ClientSession | None = None

    async def _get_session(self) -> aiohttp.ClientSession:
        """Get or create HTTP session."""
        if self._session is None or self._session.closed:
            auth = None
            if self.rpc_user and self.rpc_password:
                auth = aiohttp.BasicAuth(self.rpc_user, self.rpc_password)
            self._session = aiohttp.ClientSession(auth=auth)
        return self._session

    async def close(self) -> None:
        """Close HTTP session."""
        if self._session and not self._session.closed:
            await self._session.close()

    async def _call(self, method: str, params: list[Any] | None = None) -> Any:
        """Make JSON-RPC call to Botcash node.

        Args:
            method: RPC method name
            params: Method parameters

        Returns:
            RPC result

        Raises:
            BotcashRpcError: If RPC returns an error
        """
        self._request_id += 1
        payload = {
            "jsonrpc": "2.0",
            "id": self._request_id,
            "method": method,
            "params": params or [],
        }

        session = await self._get_session()
        async with session.post(
            self.rpc_url,
            json=payload,
            headers={"Content-Type": "application/json"},
        ) as response:
            data = await response.json()

        if "error" in data and data["error"]:
            err = data["error"]
            raise BotcashRpcError(err.get("code", -1), err.get("message", "Unknown error"))

        return data.get("result")

    async def get_blockchain_info(self) -> dict[str, Any]:
        """Get blockchain info to verify connection."""
        return await self._call("getblockchaininfo")

    async def get_balance(self, address: str) -> Balance:
        """Get balance for a Botcash address.

        Args:
            address: Botcash address (bs1..., B1..., etc.)

        Returns:
            Balance information
        """
        # Use z_getbalance for shielded addresses
        if address.startswith("bs") or address.startswith("bu"):
            result = await self._call("z_getbalance", [address])
            return Balance(
                address=address,
                confirmed=int(result * 100_000_000),
                pending=0,
            )
        else:
            # Transparent address
            result = await self._call("getreceivedbyaddress", [address])
            return Balance(
                address=address,
                confirmed=int(result * 100_000_000),
                pending=0,
            )

    async def validate_address(self, address: str) -> bool:
        """Validate a Botcash address.

        Args:
            address: Address to validate

        Returns:
            True if valid Botcash address
        """
        try:
            if address.startswith("bs") or address.startswith("bu"):
                result = await self._call("z_validateaddress", [address])
                return result.get("isvalid", False)
            else:
                result = await self._call("validateaddress", [address])
                return result.get("isvalid", False)
        except BotcashRpcError:
            return False

    def generate_challenge(self) -> str:
        """Generate a random challenge for identity verification.

        Returns:
            32-byte hex-encoded challenge
        """
        return secrets.token_hex(32)

    def compute_challenge_hash(self, challenge: str, twitter_user_id: str) -> str:
        """Compute hash of challenge for verification.

        Args:
            challenge: The challenge string
            twitter_user_id: Twitter user ID

        Returns:
            SHA256 hash of challenge+twitter_user_id
        """
        data = f"{challenge}:{twitter_user_id}".encode()
        return hashlib.sha256(data).hexdigest()

    async def create_post(
        self,
        from_address: str,
        content: str,
        tags: list[str] | None = None,
    ) -> PostResult:
        """Create a social post on Botcash.

        Args:
            from_address: Sender's Botcash address
            content: Post content (max ~450 bytes)
            tags: Optional hashtags

        Returns:
            PostResult with tx_id if successful
        """
        try:
            result = await self._call("z_socialpost", [from_address, content, tags or []])
            return PostResult(tx_id=result["txid"], success=True)
        except BotcashRpcError as e:
            logger.error("Failed to create post", error=str(e))
            return PostResult(tx_id="", success=False, error=str(e))

    async def send_dm(
        self,
        from_address: str,
        to_address: str,
        content: str,
    ) -> PostResult:
        """Send an encrypted DM.

        Args:
            from_address: Sender's Botcash address
            to_address: Recipient's Botcash address
            content: Message content

        Returns:
            PostResult with tx_id if successful
        """
        try:
            result = await self._call("z_socialdm", [from_address, to_address, content])
            return PostResult(tx_id=result["txid"], success=True)
        except BotcashRpcError as e:
            logger.error("Failed to send DM", error=str(e))
            return PostResult(tx_id="", success=False, error=str(e))

    async def create_bridge_link(
        self,
        botcash_address: str,
        platform: str,
        platform_id: str,
        proof: str,
    ) -> PostResult:
        """Create on-chain bridge link transaction.

        Args:
            botcash_address: User's Botcash address
            platform: Platform name ("twitter")
            platform_id: Twitter user ID
            proof: OAuth proof (state hash)

        Returns:
            PostResult with tx_id if successful
        """
        try:
            result = await self._call("z_bridge_link", [
                botcash_address,
                platform,
                platform_id,
                proof,
                "selective",  # default privacy mode
            ])
            return PostResult(tx_id=result["txid"], success=True)
        except BotcashRpcError as e:
            logger.error("Failed to create bridge link", error=str(e))
            return PostResult(tx_id="", success=False, error=str(e))

    async def get_feed(
        self,
        addresses: list[str],
        limit: int = 20,
        offset: int = 0,
    ) -> list[dict[str, Any]]:
        """Get social feed for addresses.

        Args:
            addresses: List of addresses to get feed for
            limit: Max posts to return
            offset: Pagination offset

        Returns:
            List of post dictionaries
        """
        try:
            result = await self._call("z_socialfeed", [addresses, limit, offset])
            return result.get("posts", [])
        except BotcashRpcError as e:
            logger.error("Failed to get feed", error=str(e))
            return []

    async def get_post_by_txid(self, tx_id: str) -> dict[str, Any] | None:
        """Get a post by its transaction ID.

        Args:
            tx_id: Transaction ID

        Returns:
            Post dictionary or None if not found
        """
        try:
            result = await self._call("z_socialpost_get", [tx_id])
            return result
        except BotcashRpcError:
            return None

    async def get_profile(self, address: str) -> dict[str, Any] | None:
        """Get profile for a Botcash address.

        Args:
            address: Botcash address

        Returns:
            Profile dictionary or None if not found
        """
        try:
            result = await self._call("z_socialprofile", [address])
            return result
        except BotcashRpcError:
            return None

    async def get_new_posts_since(
        self,
        address: str,
        since_height: int,
        limit: int = 100,
    ) -> list[dict[str, Any]]:
        """Get new posts from an address since a block height.

        Args:
            address: Botcash address to watch
            since_height: Block height to start from
            limit: Max posts to return

        Returns:
            List of new posts
        """
        try:
            result = await self._call("z_socialfeed_since", [address, since_height, limit])
            return result.get("posts", [])
        except BotcashRpcError as e:
            logger.error("Failed to get new posts", error=str(e))
            return []
