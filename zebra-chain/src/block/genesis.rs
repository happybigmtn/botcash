//! Genesis block definitions for different networks.
//!
//! Genesis blocks are the first block in a blockchain, with no parent block.
//! They are hard-coded into the node software and define the start of the chain.

use std::sync::Arc;

use hex::FromHex;

use crate::{block::Block, serialization::ZcashDeserializeInto};

/// Genesis block for Regtest, copied from zcashd via `getblock 0 0` RPC method
pub fn regtest_genesis_block() -> Arc<Block> {
    let regtest_genesis_block_bytes =
        <Vec<u8>>::from_hex(include_str!("genesis/block-regtest-0-000-000.txt").trim())
            .expect("Block bytes are in valid hex representation");

    regtest_genesis_block_bytes
        .zcash_deserialize_into()
        .map(Arc::new)
        .expect("hard-coded Regtest genesis block data must deserialize successfully")
}

/// Genesis block for Botcash mainnet.
///
/// Contains the genesis message: "Privacy is not secrecy. Agents deserve both."
///
/// Block parameters:
/// - Timestamp: 2026-02-01 00:00:00 UTC (1769904000)
/// - Difficulty: 0x1f07ffff (easiest difficulty)
/// - Block subsidy: 0 BCASH (no premine)
/// - Uses RandomX PoW (verified at consensus layer, not in Solution field)
///
/// The Equihash solution field contains zeros because Botcash uses RandomX PoW,
/// which is verified separately by the consensus layer based on the network type.
pub fn botcash_genesis_block() -> Arc<Block> {
    let botcash_genesis_block_bytes =
        <Vec<u8>>::from_hex(include_str!("genesis/block-botcash-0-000-000.txt").trim())
            .expect("Block bytes are in valid hex representation");

    botcash_genesis_block_bytes
        .zcash_deserialize_into()
        .map(Arc::new)
        .expect("hard-coded Botcash genesis block data must deserialize successfully")
}

/// The hash of the Botcash genesis block.
///
/// This is the SHA256d hash of the block header, displayed in big-endian order.
/// Internal byte order (little-endian) is used in the actual block::Hash.
pub const BOTCASH_GENESIS_HASH: &str =
    "b42125dbe5ba96501aa1336634d2689dcabe0e5cb1c57450bd6cae5328a0b6f3";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Height;

    #[test]
    fn regtest_genesis_block_is_valid() {
        let _init_guard = zebra_test::init();

        let block = regtest_genesis_block();
        assert_eq!(block.coinbase_height(), Some(Height(0)));
        assert_eq!(block.transactions.len(), 1);
    }

    #[test]
    fn botcash_genesis_block_is_valid() {
        let _init_guard = zebra_test::init();

        let block = botcash_genesis_block();
        assert_eq!(block.coinbase_height(), Some(Height(0)));
        assert_eq!(block.transactions.len(), 1);
    }

    #[test]
    fn botcash_genesis_hash_is_correct() {
        let _init_guard = zebra_test::init();

        let block = botcash_genesis_block();
        let hash = block.hash();

        // Verify the hash matches our expected constant
        let expected_hash: crate::block::Hash = BOTCASH_GENESIS_HASH
            .parse()
            .expect("genesis hash should parse");
        assert_eq!(hash, expected_hash);
    }

    #[test]
    fn botcash_genesis_contains_message() {
        let _init_guard = zebra_test::init();

        let block = botcash_genesis_block();
        let coinbase = &block.transactions[0];

        // Get the coinbase input
        let input = coinbase.inputs().first().expect("coinbase has input");

        // Check that the coinbase data contains our message
        if let crate::transparent::Input::Coinbase { data, height, .. } = input {
            assert_eq!(*height, Height(0));
            let data_str = String::from_utf8_lossy(data.as_ref());
            assert!(
                data_str.contains("Privacy is not secrecy"),
                "Genesis block should contain the Botcash genesis message"
            );
        } else {
            panic!("Expected coinbase input");
        }
    }
}
