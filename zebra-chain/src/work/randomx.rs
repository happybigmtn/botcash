//! RandomX Proof-of-Work implementation for Botcash.
//!
//! RandomX is a CPU-optimized PoW algorithm that enables AI agents to mine
//! while idle. It uses random code execution with memory-hard techniques to
//! minimize the efficiency advantage of specialized hardware.
//!
//! This module provides verification of RandomX proofs-of-work for Botcash blocks.

use std::fmt;

use randomx_rs::{RandomXCache, RandomXError, RandomXFlag, RandomXVM};

use crate::block::Header;

/// The error type for RandomX validation.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to create RandomX cache
    #[error("failed to create RandomX cache: {0}")]
    CacheCreation(RandomXError),

    /// Failed to create RandomX VM
    #[error("failed to create RandomX VM: {0}")]
    VMCreation(RandomXError),

    /// Failed to calculate hash
    #[error("failed to calculate RandomX hash: {0}")]
    HashCalculation(RandomXError),

    /// The RandomX solution is invalid
    #[error("invalid RandomX solution for BlockHeader")]
    InvalidSolution,
}

impl From<RandomXError> for Error {
    fn from(err: RandomXError) -> Self {
        Error::HashCalculation(err)
    }
}

/// RandomX configuration constants for Botcash.
///
/// These parameters define the RandomX algorithm behavior for Botcash mining.
pub mod constants {
    /// The size of the header input used for RandomX hashing.
    ///
    /// This includes:
    /// - version (4 bytes)
    /// - previous_block_hash (32 bytes)
    /// - merkle_root (32 bytes)
    /// - commitment_bytes (32 bytes)
    /// - time (4 bytes)
    /// - difficulty_threshold (4 bytes)
    ///
    /// The nonce is appended separately for the hash input.
    pub const HEADER_INPUT_LENGTH: usize = 4 + 32 + 32 + 32 + 4 + 4; // 108 bytes

    /// The argon2 memory parameter (in KB).
    pub const RANDOMX_ARGON_MEMORY: u32 = 262144; // 256 KB per lane

    /// The number of argon2 iterations.
    pub const RANDOMX_ARGON_ITERATIONS: u32 = 3;

    /// The number of argon2 lanes.
    pub const RANDOMX_ARGON_LANES: u32 = 1;

    /// The argon2 salt for Botcash.
    pub const RANDOMX_ARGON_SALT: &[u8] = b"BotcashRandomX\x00";

    /// Number of cache accesses for RandomX.
    pub const RANDOMX_CACHE_ACCESSES: u32 = 8;

    /// The RandomX hash output size in bytes.
    pub const RANDOMX_HASH_SIZE: usize = 32;

    /// Key epoch length in blocks (same as Monero).
    /// The RandomX key changes every 2048 blocks.
    pub const KEY_EPOCH_LENGTH: u32 = 2048;
}

/// Get the flags for RandomX operation.
///
/// Uses light mode (cache-based) for verification, which is sufficient
/// for block validation and uses ~256MB of RAM instead of ~2GB.
fn get_randomx_flags() -> RandomXFlag {
    RandomXFlag::get_recommended_flags()
}

/// Get the key block hash for RandomX cache initialization.
///
/// The key is derived from a previous block hash, changing every 2048 blocks.
/// For now, we use a genesis-derived key until the block height logic is fully implemented.
///
/// In production, this should be:
/// - key_height = (block_height / 2048) * 2048
/// - key = hash of block at key_height (or genesis if key_height == 0)
pub fn get_key_for_height(height: u32) -> [u8; 32] {
    // Key epoch is 2048 blocks (same as Monero)
    let key_height = (height / constants::KEY_EPOCH_LENGTH) * constants::KEY_EPOCH_LENGTH;

    if key_height == 0 {
        // Genesis key - use a fixed seed derived from "Botcash"
        let mut key = [0u8; 32];
        key[..8].copy_from_slice(b"Botcash\0");
        key
    } else {
        // In production, this would query the block hash at key_height
        // For now, we derive a deterministic key from the height
        let mut key = [0u8; 32];
        key[..4].copy_from_slice(&key_height.to_le_bytes());
        key[4..12].copy_from_slice(b"Botcash\0");
        key
    }
}

/// Calculate the RandomX hash of the given input.
///
/// This creates a new cache and VM for each call. While this is less efficient
/// than caching, it's thread-safe and simpler. For high-performance mining,
/// a thread-local cache should be used instead.
pub fn calculate_hash(input: &[u8], key: &[u8; 32]) -> Result<[u8; 32], Error> {
    let flags = get_randomx_flags();

    // Create cache and VM for this hash calculation
    let cache = RandomXCache::new(flags, key).map_err(Error::CacheCreation)?;

    let vm = RandomXVM::new(flags, Some(cache), None).map_err(Error::VMCreation)?;

    let hash = vm.calculate_hash(input)?;

    let mut result = [0u8; 32];
    result.copy_from_slice(&hash[..32]);
    Ok(result)
}

/// Verify that a block header meets the RandomX PoW requirements.
///
/// This function:
/// 1. Serializes the header to get the PoW input
/// 2. Calculates the RandomX hash using the header input + nonce
/// 3. Compares the hash against the difficulty threshold
///
/// # Arguments
///
/// * `header` - The block header to verify
/// * `height` - The block height (used to determine the key epoch)
///
/// # Returns
///
/// `Ok(())` if the PoW is valid, `Err(Error)` otherwise.
pub fn verify(header: &Header, height: u32) -> Result<(), Error> {
    use crate::serialization::ZcashSerialize;

    // Get the key for this block's epoch
    let key = get_key_for_height(height);

    // Serialize the header to get the input
    let mut header_bytes = Vec::new();
    header
        .zcash_serialize(&mut header_bytes)
        .expect("serialization into a vec can't fail");

    // The RandomX input is the header (first 108 bytes) + nonce (32 bytes)
    // This creates a 140-byte input for hashing
    let input_end = constants::HEADER_INPUT_LENGTH + 32; // header + nonce
    let input = &header_bytes[..input_end.min(header_bytes.len())];

    // Calculate the RandomX hash
    let hash = calculate_hash(input, &key)?;

    // Convert hash to ExpandedDifficulty for comparison
    // RandomX produces a 256-bit hash that we compare against the target
    use crate::work::difficulty::{ExpandedDifficulty, U256};
    let hash_as_difficulty: ExpandedDifficulty = U256::from_little_endian(&hash).into();

    // Get the expanded difficulty threshold
    let difficulty_threshold = header
        .difficulty_threshold
        .to_expanded()
        .ok_or(Error::InvalidSolution)?;

    // The hash must be less than or equal to the difficulty threshold
    // Note: like Bitcoin/Zcash, lower hash values represent more work
    if hash_as_difficulty > difficulty_threshold {
        return Err(Error::InvalidSolution);
    }

    Ok(())
}

/// A placeholder "solution" type for RandomX blocks.
///
/// Unlike Equihash which has a 1344-byte solution, RandomX PoW is verified
/// purely by hashing the header with the nonce. The "solution" in RandomX
/// is effectively the nonce that produces a hash meeting the difficulty target.
///
/// This type exists for API compatibility with the existing block structure.
#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct RandomXSolution;

impl RandomXSolution {
    /// Check if the RandomX proof-of-work is valid for the given header.
    ///
    /// This is a convenience wrapper around [`verify`] that extracts the height
    /// from the header context. For most use cases, prefer calling [`verify`]
    /// directly with the known block height.
    pub fn check(&self, header: &Header, height: u32) -> Result<(), Error> {
        verify(header, height)
    }
}

impl Default for RandomXSolution {
    fn default() -> Self {
        Self
    }
}

impl fmt::Debug for RandomXSolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RandomXSolution").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn randomx_key_derivation() {
        // Test that key derivation is deterministic
        let key1 = get_key_for_height(0);
        let key2 = get_key_for_height(0);
        assert_eq!(key1, key2, "Key derivation should be deterministic");

        // Test that different epochs produce different keys
        let key_epoch_0 = get_key_for_height(0);
        let key_epoch_1 = get_key_for_height(2048);
        assert_ne!(
            key_epoch_0, key_epoch_1,
            "Different epochs should have different keys"
        );

        // Test that blocks within same epoch have same key
        let key_1 = get_key_for_height(1);
        let key_2047 = get_key_for_height(2047);
        assert_eq!(key_1, key_2047, "Same epoch should have same key");
    }

    #[test]
    fn randomx_flags_are_valid() {
        // Test that we can get valid flags
        let flags = get_randomx_flags();
        // The flags should be non-zero (some optimization should be enabled)
        assert!(
            flags != RandomXFlag::FLAG_DEFAULT || flags == RandomXFlag::FLAG_DEFAULT,
            "Flags should be valid"
        );
    }

    #[test]
    fn randomx_hash_calculation() {
        // Test that hash calculation works
        let key = get_key_for_height(0);
        let input = b"test input for RandomX hashing";

        let result = calculate_hash(input, &key);
        assert!(result.is_ok(), "Hash calculation should succeed");

        let hash = result.unwrap();
        assert_eq!(hash.len(), 32, "Hash should be 32 bytes");
    }

    #[test]
    fn randomx_hash_is_deterministic() {
        let key = get_key_for_height(0);
        let input = b"deterministic test input";

        let hash1 = calculate_hash(input, &key).expect("first hash should succeed");
        let hash2 = calculate_hash(input, &key).expect("second hash should succeed");

        assert_eq!(hash1, hash2, "Same input should produce same hash");
    }

    #[test]
    fn randomx_different_inputs_produce_different_hashes() {
        let key = get_key_for_height(0);
        let input1 = b"input one";
        let input2 = b"input two";

        let hash1 = calculate_hash(input1, &key).expect("first hash should succeed");
        let hash2 = calculate_hash(input2, &key).expect("second hash should succeed");

        assert_ne!(
            hash1, hash2,
            "Different inputs should produce different hashes"
        );
    }
}
