//! Constants for the Botcash main network.

/// The Botcash coin type, as defined by [SLIP 44].
///
/// Uses 347 as a distinctive identifier for Botcash.
///
/// [SLIP 44]: https://github.com/satoshilabs/slips/blob/master/slip-0044.md
pub const COIN_TYPE: u32 = 347;

/// The HRP for a Bech32-encoded Botcash Sapling [`ExtendedSpendingKey`].
///
/// Defined in [ZIP 32].
///
/// [`ExtendedSpendingKey`]: https://docs.rs/sapling-crypto/latest/sapling_crypto/zip32/struct.ExtendedSpendingKey.html
/// [ZIP 32]: https://github.com/zcash/zips/blob/main/zips/zip-0032.rst
pub const HRP_SAPLING_EXTENDED_SPENDING_KEY: &str = "secret-extended-key-botcash";

/// The HRP for a Bech32-encoded Botcash [`ExtendedFullViewingKey`].
///
/// Defined in [ZIP 32].
///
/// [`ExtendedFullViewingKey`]: https://docs.rs/sapling-crypto/latest/sapling_crypto/zip32/struct.ExtendedFullViewingKey.html
/// [ZIP 32]: https://github.com/zcash/zips/blob/main/zips/zip-0032.rst
pub const HRP_SAPLING_EXTENDED_FULL_VIEWING_KEY: &str = "bviews";

/// The HRP for a Bech32-encoded Botcash Sapling [`PaymentAddress`].
///
/// Defined in section 5.6.4 of the [Zcash Protocol Specification].
///
/// [`PaymentAddress`]: https://docs.rs/sapling-crypto/latest/sapling_crypto/struct.PaymentAddress.html
/// [Zcash Protocol Specification]: https://github.com/zcash/zips/blob/main/rendered/protocol/protocol.pdf
pub const HRP_SAPLING_PAYMENT_ADDRESS: &str = "bs";

/// The prefix for a Base58Check-encoded Botcash Sprout address.
/// This produces addresses starting with "bZ".
///
/// Defined in the [Zcash Protocol Specification section 5.6.3][sproutpaymentaddrencoding].
/// Note: Sprout is deprecated but prefix is required for protocol completeness.
///
/// [sproutpaymentaddrencoding]: https://zips.z.cash/protocol/protocol.pdf#sproutpaymentaddrencoding
pub const B58_SPROUT_ADDRESS_PREFIX: [u8; 2] = [0x0d, 0x8f];

/// The prefix for a Base58Check-encoded DER-encoded Botcash [`SecretKey`], as specified via the
/// bitcoin-derived [`EncodeSecret`] format function.
///
/// [`SecretKey`]: https://docs.rs/secp256k1/latest/secp256k1/struct.SecretKey.html
/// [`EncodeSecret`]: https://github.com/zcash/zcash/blob/1f1f7a385adc048154e7f25a3a0de76f3658ca09/src/key_io.cpp#L298
pub const B58_SECRET_KEY_PREFIX: [u8; 1] = [0x80];

/// The prefix for a Base58Check-encoded Botcash [`PublicKeyHash`].
/// This produces addresses starting with "B1".
///
/// [`PublicKeyHash`]: https://docs.rs/zcash_transparent/latest/zcash_transparent/address/enum.TransparentAddress.html
pub const B58_PUBKEY_ADDRESS_PREFIX: [u8; 2] = [0x05, 0xa2];

/// The prefix for a Base58Check-encoded Botcash [`ScriptHash`].
/// This produces addresses starting with "B3".
///
/// [`ScriptHash`]: https://docs.rs/zcash_transparent/latest/zcash_transparent/address/enum.TransparentAddress.html
pub const B58_SCRIPT_ADDRESS_PREFIX: [u8; 2] = [0x05, 0xa7];

/// The HRP for a Bech32m-encoded Botcash [ZIP 320] TEX address.
///
/// [ZIP 320]: https://zips.z.cash/zip-0320
pub const HRP_TEX_ADDRESS: &str = "btex";

/// The HRP for a Bech32m-encoded Botcash Unified Address.
///
/// Defined in [ZIP 316][zip-0316].
///
/// [zip-0316]: https://zips.z.cash/zip-0316
pub const HRP_UNIFIED_ADDRESS: &str = "bu";

/// The HRP for a Bech32m-encoded Botcash Unified FVK.
///
/// Defined in [ZIP 316][zip-0316].
///
/// [zip-0316]: https://zips.z.cash/zip-0316
pub const HRP_UNIFIED_FVK: &str = "buview";

/// The HRP for a Bech32m-encoded Botcash Unified IVK.
///
/// Defined in [ZIP 316][zip-0316].
///
/// [zip-0316]: https://zips.z.cash/zip-0316
pub const HRP_UNIFIED_IVK: &str = "buivk";
