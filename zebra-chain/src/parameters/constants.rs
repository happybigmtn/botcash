//! Definitions of Zebra chain constants, including:
//! - slow start interval,
//! - slow start shift

use crate::block::Height;

/// An initial period from Genesis to this Height where the block subsidy is gradually incremented. [What is slow-start mining][slow-mining]
///
/// [slow-mining]: https://z.cash/support/faq/#what-is-slow-start-mining
pub const SLOW_START_INTERVAL: Height = Height(20_000);

/// `SlowStartShift()` as described in [protocol specification §7.8][7.8]
///
/// [7.8]: https://zips.z.cash/protocol/protocol.pdf#subsidies
///
/// This calculation is exact, because `SLOW_START_INTERVAL` is divisible by 2.
pub const SLOW_START_SHIFT: Height = Height(SLOW_START_INTERVAL.0 / 2);

/// Magic numbers used to identify different Zcash networks.
pub mod magics {
    use crate::parameters::network::magic::Magic;

    /// The production mainnet.
    pub const MAINNET: Magic = Magic([0x24, 0xe9, 0x27, 0x64]);
    /// The testnet.
    pub const TESTNET: Magic = Magic([0xfa, 0x1a, 0xf9, 0xbf]);
    /// The regtest, see <https://github.com/zcash/zcash/blob/master/src/chainparams.cpp#L716-L719>
    pub const REGTEST: Magic = Magic([0xaa, 0xe8, 0x3f, 0x5f]);
    /// Botcash mainnet magic bytes ("BCAS" = 0x42434153).
    pub const BOTCASH: Magic = Magic([0x42, 0x43, 0x41, 0x53]);
}

/// Constants for the Botcash mainnet.
///
/// These mirror the definitions in `librustzcash/components/zcash_protocol/src/constants/botcash.rs`.
pub mod botcash {
    /// The prefix for a Base58Check-encoded Botcash [`PublicKeyHash`].
    /// This produces addresses starting with "B1".
    pub const B58_PUBKEY_ADDRESS_PREFIX: [u8; 2] = [0x05, 0xa2];

    /// The prefix for a Base58Check-encoded Botcash [`ScriptHash`].
    /// This produces addresses starting with "B3".
    pub const B58_SCRIPT_ADDRESS_PREFIX: [u8; 2] = [0x05, 0xa7];

    /// The HRP for a Bech32m-encoded Botcash [ZIP 320] TEX address.
    pub const HRP_TEX_ADDRESS: &str = "btex";

    /// The 2 bytes prefix for Bech32m-encoded transparent TEX addresses.
    /// Derived for the "btex" HRP.
    pub const TEX_ADDRESS_PREFIX: [u8; 2] = [0x1c, 0xc0];
}
