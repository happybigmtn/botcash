//! Zebra supported RPC methods.
//!
//! Based on the [`zcashd` RPC methods](https://zcash.github.io/rpc/)
//! as used by `lightwalletd.`
//!
//! Some parts of the `zcashd` RPC documentation are outdated.
//! So this implementation follows the `zcashd` server and `lightwalletd` client implementations.
//!
//! # Developing this module
//!
//! If RPCs are added or changed, ensure the following:
//!
//! - Request types can be instantiated from dependent crates, and
//!   response types are fully-readable (up to each leaf component), meaning
//!   every field on response types can be read, and any types used in response
//!   types has an appropriate API for either directly accessing their fields, or
//!   has an appropriate API for accessing any relevant data.
//!
//!   This should be achieved, wherever possible, by:
//!   - Using `derive(Getters, new)` to keep new code succinct and consistent.
//!     Ensure that fields on response types that implement `Copy` are tagged
//!     with `#[getter(copy)]` field attributes to avoid unnecessary references.
//!     This should be easily noticeable in the `serialization_tests` test crate, where
//!     any fields implementing `Copy` but not tagged with `#[getter(Copy)]` will
//!     be returned by reference, and will require dereferencing with the dereference
//!     operator, `*`. If a value returned by a getter method requires dereferencing,
//!     the associated field in the response type should likely be tagged with `#[getter(Copy)]`.
//!   - If a field is added, use `#[new(...)]` so that it's not added to the
//!     constructor. If that is unavoidable, then it will require a major
//!     version bump.
//!
//! - A test has been added to the `serialization_tests` test crate to ensure the above.

use std::{
    cmp,
    collections::{HashMap, HashSet},
    fmt,
    ops::RangeInclusive,
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use derive_getters::Getters;
use derive_new::new;
use futures::{future::OptionFuture, stream::FuturesOrdered, StreamExt, TryFutureExt};
use hex::{FromHex, ToHex};
use indexmap::IndexMap;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use jsonrpsee_proc_macros::rpc;
use jsonrpsee_types::{ErrorCode, ErrorObject};
use tokio::{
    sync::{broadcast, watch},
    task::JoinHandle,
};
use tower::{Service, ServiceExt};
use tracing::Instrument;

use zcash_address::{unified::Encoding, TryFromAddress};
use zcash_primitives::consensus::Parameters;

use zebra_chain::{
    amount::{self, Amount, NegativeAllowed, NonNegative},
    block::{self, Block, Commitment, Height, SerializedBlock, TryIntoHeight},
    chain_sync_status::ChainSyncStatus,
    chain_tip::{ChainTip, NetworkChainTipHeightEstimator},
    parameters::{
        subsidy::{
            block_subsidy, funding_stream_values, miner_subsidy, FundingStreamReceiver,
            ParameterSubsidy,
        },
        ConsensusBranchId, Network, NetworkUpgrade, POW_AVERAGING_WINDOW,
    },
    primitives,
    serialization::{ZcashDeserialize, ZcashDeserializeInto, ZcashSerialize},
    subtree::NoteCommitmentSubtreeIndex,
    transaction::{self, SerializedTransaction, Transaction, UnminedTx},
    transparent::{self, Address, OutputIndex},
    value_balance::ValueBalance,
    work::{
        difficulty::{CompactDifficulty, ExpandedDifficulty, ParameterDifficulty, U256},
        equihash::Solution,
    },
};
use zebra_consensus::{funding_stream_address, ParameterCheckpoint, RouterError};
use zebra_network::{address_book_peers::AddressBookPeers, PeerSocketAddr};
use zebra_node_services::mempool;
use zebra_state::{HashOrHeight, OutputLocation, ReadRequest, ReadResponse, TransactionLocation};

use crate::{
    config,
    methods::types::validate_address::validate_address,
    queue::Queue,
    server::{
        self,
        error::{MapError, OkOrError},
    },
};

pub(crate) mod hex_data;
pub(crate) mod trees;
pub(crate) mod types;

use hex_data::HexData;
use trees::{GetSubtreesByIndexResponse, GetTreestateResponse, SubtreeRpcData};
use types::{
    get_block_template::{
        constants::{
            DEFAULT_SOLUTION_RATE_WINDOW_SIZE, MEMPOOL_LONG_POLL_INTERVAL,
            ZCASHD_FUNDING_STREAM_ORDER,
        },
        proposal::proposal_block_from_template,
        BlockTemplateResponse, BlockTemplateTimeSource, GetBlockTemplateHandler,
        GetBlockTemplateParameters, GetBlockTemplateResponse,
    },
    get_blockchain_info::GetBlockchainInfoBalance,
    get_mining_info::GetMiningInfoResponse,
    get_raw_mempool::{self, GetRawMempoolResponse},
    long_poll::LongPollInput,
    peer_info::PeerInfo,
    submit_block::{SubmitBlockErrorResponse, SubmitBlockParameters, SubmitBlockResponse},
    subsidy::GetBlockSubsidyResponse,
    transaction::TransactionObject,
    unified_address::ZListUnifiedReceiversResponse,
    validate_address::ValidateAddressResponse,
    z_validate_address::{ZValidateAddressResponse, ZValidateAddressType},
};

#[cfg(test)]
mod tests;

#[rpc(server)]
/// RPC method signatures.
pub trait Rpc {
    /// Returns software information from the RPC server, as a [`GetInfo`] JSON struct.
    ///
    /// zcashd reference: [`getinfo`](https://zcash.github.io/rpc/getinfo.html)
    /// method: post
    /// tags: control
    ///
    /// # Notes
    ///
    /// [The zcashd reference](https://zcash.github.io/rpc/getinfo.html) might not show some fields
    /// in Zebra's [`GetInfo`]. Zebra uses the field names and formats from the
    /// [zcashd code](https://github.com/zcash/zcash/blob/v4.6.0-1/src/rpc/misc.cpp#L86-L87).
    ///
    /// Some fields from the zcashd reference are missing from Zebra's [`GetInfo`]. It only contains the fields
    /// [required for lightwalletd support.](https://github.com/zcash/lightwalletd/blob/v0.4.9/common/common.go#L91-L95)
    #[method(name = "getinfo")]
    async fn get_info(&self) -> Result<GetInfoResponse>;

    /// Returns blockchain state information, as a [`GetBlockchainInfoResponse`] JSON struct.
    ///
    /// zcashd reference: [`getblockchaininfo`](https://zcash.github.io/rpc/getblockchaininfo.html)
    /// method: post
    /// tags: blockchain
    ///
    /// # Notes
    ///
    /// Some fields from the zcashd reference are missing from Zebra's [`GetBlockchainInfoResponse`]. It only contains the fields
    /// [required for lightwalletd support.](https://github.com/zcash/lightwalletd/blob/v0.4.9/common/common.go#L72-L89)
    #[method(name = "getblockchaininfo")]
    async fn get_blockchain_info(&self) -> Result<GetBlockchainInfoResponse>;

    /// Returns the total balance of a provided `addresses` in an [`AddressBalance`] instance.
    ///
    /// zcashd reference: [`getaddressbalance`](https://zcash.github.io/rpc/getaddressbalance.html)
    /// method: post
    /// tags: address
    ///
    /// # Parameters
    ///
    /// - `address_strings`: (object, example={"addresses": ["tmYXBYJj1K7vhejSec5osXK2QsGa5MTisUQ"]}) A JSON map with a single entry
    ///     - `addresses`: (array of strings) A list of base-58 encoded addresses.
    ///
    /// # Notes
    ///
    /// zcashd also accepts a single string parameter instead of an array of strings, but Zebra
    /// doesn't because lightwalletd always calls this RPC with an array of addresses.
    ///
    /// zcashd also returns the total amount of Zatoshis received by the addresses, but Zebra
    /// doesn't because lightwalletd doesn't use that information.
    ///
    /// The RPC documentation says that the returned object has a string `balance` field, but
    /// zcashd actually [returns an
    /// integer](https://github.com/zcash/lightwalletd/blob/bdaac63f3ee0dbef62bde04f6817a9f90d483b00/common/common.go#L128-L130).
    #[method(name = "getaddressbalance")]
    async fn get_address_balance(
        &self,
        address_strings: GetAddressBalanceRequest,
    ) -> Result<GetAddressBalanceResponse>;

    /// Sends the raw bytes of a signed transaction to the local node's mempool, if the transaction is valid.
    /// Returns the [`SentTransactionHash`] for the transaction, as a JSON string.
    ///
    /// zcashd reference: [`sendrawtransaction`](https://zcash.github.io/rpc/sendrawtransaction.html)
    /// method: post
    /// tags: transaction
    ///
    /// # Parameters
    ///
    /// - `raw_transaction_hex`: (string, required, example="signedhex") The hex-encoded raw transaction bytes.
    /// - `allow_high_fees`: (bool, optional) A legacy parameter accepted by zcashd but ignored by Zebra.
    ///
    /// # Notes
    ///
    /// zcashd accepts an optional `allowhighfees` parameter. Zebra doesn't support this parameter,
    /// because lightwalletd doesn't use it.
    #[method(name = "sendrawtransaction")]
    async fn send_raw_transaction(
        &self,
        raw_transaction_hex: String,
        _allow_high_fees: Option<bool>,
    ) -> Result<SendRawTransactionResponse>;

    /// Returns the requested block by hash or height, as a [`GetBlock`] JSON string.
    /// If the block is not in Zebra's state, returns
    /// [error code `-8`.](https://github.com/zcash/zcash/issues/5758) if a height was
    /// passed or -5 if a hash was passed.
    ///
    /// zcashd reference: [`getblock`](https://zcash.github.io/rpc/getblock.html)
    /// method: post
    /// tags: blockchain
    ///
    /// # Parameters
    ///
    /// - `hash_or_height`: (string, required, example="1") The hash or height for the block to be returned.
    /// - `verbosity`: (number, optional, default=1, example=1) 0 for hex encoded data, 1 for a json object, and 2 for json object with transaction data.
    ///
    /// # Notes
    ///
    /// The `size` field is only returned with verbosity=2.
    ///
    /// The undocumented `chainwork` field is not returned.
    #[method(name = "getblock")]
    async fn get_block(
        &self,
        hash_or_height: String,
        verbosity: Option<u8>,
    ) -> Result<GetBlockResponse>;

    /// Returns the requested block header by hash or height, as a [`GetBlockHeader`] JSON string.
    /// If the block is not in Zebra's state,
    /// returns [error code `-8`.](https://github.com/zcash/zcash/issues/5758)
    /// if a height was passed or -5 if a hash was passed.
    ///
    /// zcashd reference: [`getblockheader`](https://zcash.github.io/rpc/getblockheader.html)
    /// method: post
    /// tags: blockchain
    ///
    /// # Parameters
    ///
    /// - `hash_or_height`: (string, required, example="1") The hash or height for the block to be returned.
    /// - `verbose`: (bool, optional, default=false, example=true) false for hex encoded data, true for a json object
    ///
    /// # Notes
    ///
    /// The undocumented `chainwork` field is not returned.
    #[method(name = "getblockheader")]
    async fn get_block_header(
        &self,
        hash_or_height: String,
        verbose: Option<bool>,
    ) -> Result<GetBlockHeaderResponse>;

    /// Returns the hash of the current best blockchain tip block, as a [`GetBlockHash`] JSON string.
    ///
    /// zcashd reference: [`getbestblockhash`](https://zcash.github.io/rpc/getbestblockhash.html)
    /// method: post
    /// tags: blockchain
    #[method(name = "getbestblockhash")]
    fn get_best_block_hash(&self) -> Result<GetBlockHashResponse>;

    /// Returns the height and hash of the current best blockchain tip block, as a [`GetBlockHeightAndHashResponse`] JSON struct.
    ///
    /// zcashd reference: none
    /// method: post
    /// tags: blockchain
    #[method(name = "getbestblockheightandhash")]
    fn get_best_block_height_and_hash(&self) -> Result<GetBlockHeightAndHashResponse>;

    /// Returns all transaction ids in the memory pool, as a JSON array.
    ///
    /// # Parameters
    ///
    /// - `verbose`: (boolean, optional, default=false) true for a json object, false for array of transaction ids.
    ///
    /// zcashd reference: [`getrawmempool`](https://zcash.github.io/rpc/getrawmempool.html)
    /// method: post
    /// tags: blockchain
    #[method(name = "getrawmempool")]
    async fn get_raw_mempool(&self, verbose: Option<bool>) -> Result<GetRawMempoolResponse>;

    /// Returns information about the given block's Sapling & Orchard tree state.
    ///
    /// zcashd reference: [`z_gettreestate`](https://zcash.github.io/rpc/z_gettreestate.html)
    /// method: post
    /// tags: blockchain
    ///
    /// # Parameters
    ///
    /// - `hash | height`: (string, required, example="00000000febc373a1da2bd9f887b105ad79ddc26ac26c2b28652d64e5207c5b5") The block hash or height.
    ///
    /// # Notes
    ///
    /// The zcashd doc reference above says that the parameter "`height` can be
    /// negative where -1 is the last known valid block". On the other hand,
    /// `lightwalletd` only uses positive heights, so Zebra does not support
    /// negative heights.
    #[method(name = "z_gettreestate")]
    async fn z_get_treestate(&self, hash_or_height: String) -> Result<GetTreestateResponse>;

    /// Returns information about a range of Sapling or Orchard subtrees.
    ///
    /// zcashd reference: [`z_getsubtreesbyindex`](https://zcash.github.io/rpc/z_getsubtreesbyindex.html) - TODO: fix link
    /// method: post
    /// tags: blockchain
    ///
    /// # Parameters
    ///
    /// - `pool`: (string, required) The pool from which subtrees should be returned. Either "sapling" or "orchard".
    /// - `start_index`: (number, required) The index of the first 2^16-leaf subtree to return.
    /// - `limit`: (number, optional) The maximum number of subtree values to return.
    ///
    /// # Notes
    ///
    /// While Zebra is doing its initial subtree index rebuild, subtrees will become available
    /// starting at the chain tip. This RPC will return an empty list if the `start_index` subtree
    /// exists, but has not been rebuilt yet. This matches `zcashd`'s behaviour when subtrees aren't
    /// available yet. (But `zcashd` does its rebuild before syncing any blocks.)
    #[method(name = "z_getsubtreesbyindex")]
    async fn z_get_subtrees_by_index(
        &self,
        pool: String,
        start_index: NoteCommitmentSubtreeIndex,
        limit: Option<NoteCommitmentSubtreeIndex>,
    ) -> Result<GetSubtreesByIndexResponse>;

    /// Returns the raw transaction data, as a [`GetRawTransaction`] JSON string or structure.
    ///
    /// zcashd reference: [`getrawtransaction`](https://zcash.github.io/rpc/getrawtransaction.html)
    /// method: post
    /// tags: transaction
    ///
    /// # Parameters
    ///
    /// - `txid`: (string, required, example="mytxid") The transaction ID of the transaction to be returned.
    /// - `verbose`: (number, optional, default=0, example=1) If 0, return a string of hex-encoded data, otherwise return a JSON object.
    /// - `blockhash` (string, optional) The block in which to look for the transaction
    #[method(name = "getrawtransaction")]
    async fn get_raw_transaction(
        &self,
        txid: String,
        verbose: Option<u8>,
        block_hash: Option<String>,
    ) -> Result<GetRawTransactionResponse>;

    /// Returns the transaction ids made by the provided transparent addresses.
    ///
    /// zcashd reference: [`getaddresstxids`](https://zcash.github.io/rpc/getaddresstxids.html)
    /// method: post
    /// tags: address
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required, example={\"addresses\": [\"tmYXBYJj1K7vhejSec5osXK2QsGa5MTisUQ\"], \"start\": 1000, \"end\": 2000}) A struct with the following named fields:
    ///     - `addresses`: (json array of string, required) The addresses to get transactions from.
    ///     - `start`: (numeric, optional) The lower height to start looking for transactions (inclusive).
    ///     - `end`: (numeric, optional) The top height to stop looking for transactions (inclusive).
    ///
    /// # Notes
    ///
    /// Only the multi-argument format is used by lightwalletd and this is what we currently support:
    /// <https://github.com/zcash/lightwalletd/blob/631bb16404e3d8b045e74a7c5489db626790b2f6/common/common.go#L97-L102>
    #[method(name = "getaddresstxids")]
    async fn get_address_tx_ids(&self, request: GetAddressTxIdsRequest) -> Result<Vec<String>>;

    /// Returns all unspent outputs for a list of addresses.
    ///
    /// zcashd reference: [`getaddressutxos`](https://zcash.github.io/rpc/getaddressutxos.html)
    /// method: post
    /// tags: address
    ///
    /// # Parameters
    ///
    /// - `addresses`: (array, required, example={\"addresses\": [\"tmYXBYJj1K7vhejSec5osXK2QsGa5MTisUQ\"]}) The addresses to get outputs from.
    ///
    /// # Notes
    ///
    /// lightwalletd always uses the multi-address request, without chaininfo:
    /// <https://github.com/zcash/lightwalletd/blob/master/frontend/service.go#L402>
    #[method(name = "getaddressutxos")]
    async fn get_address_utxos(
        &self,
        address_strings: AddressStrings,
    ) -> Result<GetAddressUtxosResponse>;

    /// Stop the running zebrad process.
    ///
    /// # Notes
    ///
    /// - Works for non windows targets only.
    /// - Works only if the network of the running zebrad process is `Regtest`.
    ///
    /// zcashd reference: [`stop`](https://zcash.github.io/rpc/stop.html)
    /// method: post
    /// tags: control
    #[method(name = "stop")]
    fn stop(&self) -> Result<String>;

    /// Returns the height of the most recent block in the best valid block chain (equivalently,
    /// the number of blocks in this chain excluding the genesis block).
    ///
    /// zcashd reference: [`getblockcount`](https://zcash.github.io/rpc/getblockcount.html)
    /// method: post
    /// tags: blockchain
    #[method(name = "getblockcount")]
    fn get_block_count(&self) -> Result<u32>;

    /// Returns the hash of the block of a given height iff the index argument correspond
    /// to a block in the best chain.
    ///
    /// zcashd reference: [`getblockhash`](https://zcash-rpc.github.io/getblockhash.html)
    /// method: post
    /// tags: blockchain
    ///
    /// # Parameters
    ///
    /// - `index`: (numeric, required, example=1) The block index.
    ///
    /// # Notes
    ///
    /// - If `index` is positive then index = block height.
    /// - If `index` is negative then -1 is the last known valid block.
    #[method(name = "getblockhash")]
    async fn get_block_hash(&self, index: i32) -> Result<GetBlockHashResponse>;

    /// Returns a block template for mining new Zcash blocks.
    ///
    /// # Parameters
    ///
    /// - `jsonrequestobject`: (string, optional) A JSON object containing arguments.
    ///
    /// zcashd reference: [`getblocktemplate`](https://zcash-rpc.github.io/getblocktemplate.html)
    /// method: post
    /// tags: mining
    ///
    /// # Notes
    ///
    /// Arguments to this RPC are currently ignored.
    /// Long polling, block proposals, server lists, and work IDs are not supported.
    ///
    /// Miners can make arbitrary changes to blocks, as long as:
    /// - the data sent to `submitblock` is a valid Zcash block, and
    /// - the parent block is a valid block that Zebra already has, or will receive soon.
    ///
    /// Zebra verifies blocks in parallel, and keeps recent chains in parallel,
    /// so moving between chains and forking chains is very cheap.
    #[method(name = "getblocktemplate")]
    async fn get_block_template(
        &self,
        parameters: Option<GetBlockTemplateParameters>,
    ) -> Result<GetBlockTemplateResponse>;

    /// Submits block to the node to be validated and committed.
    /// Returns the [`SubmitBlockResponse`] for the operation, as a JSON string.
    ///
    /// zcashd reference: [`submitblock`](https://zcash.github.io/rpc/submitblock.html)
    /// method: post
    /// tags: mining
    ///
    /// # Parameters
    ///
    /// - `hexdata`: (string, required)
    /// - `jsonparametersobject`: (string, optional) - currently ignored
    ///
    /// # Notes
    ///
    ///  - `jsonparametersobject` holds a single field, workid, that must be included in submissions if provided by the server.
    #[method(name = "submitblock")]
    async fn submit_block(
        &self,
        hex_data: HexData,
        _parameters: Option<SubmitBlockParameters>,
    ) -> Result<SubmitBlockResponse>;

    /// Returns mining-related information.
    ///
    /// zcashd reference: [`getmininginfo`](https://zcash.github.io/rpc/getmininginfo.html)
    /// method: post
    /// tags: mining
    #[method(name = "getmininginfo")]
    async fn get_mining_info(&self) -> Result<GetMiningInfoResponse>;

    /// Returns the estimated network solutions per second based on the last `num_blocks` before
    /// `height`.
    ///
    /// If `num_blocks` is not supplied, uses 120 blocks. If it is 0 or -1, uses the difficulty
    /// averaging window.
    /// If `height` is not supplied or is -1, uses the tip height.
    ///
    /// zcashd reference: [`getnetworksolps`](https://zcash.github.io/rpc/getnetworksolps.html)
    /// method: post
    /// tags: mining
    #[method(name = "getnetworksolps")]
    async fn get_network_sol_ps(&self, num_blocks: Option<i32>, height: Option<i32>)
        -> Result<u64>;

    /// Returns the estimated network solutions per second based on the last `num_blocks` before
    /// `height`.
    ///
    /// This method name is deprecated, use [`getnetworksolps`](Self::get_network_sol_ps) instead.
    /// See that method for details.
    ///
    /// zcashd reference: [`getnetworkhashps`](https://zcash.github.io/rpc/getnetworkhashps.html)
    /// method: post
    /// tags: mining
    #[method(name = "getnetworkhashps")]
    async fn get_network_hash_ps(
        &self,
        num_blocks: Option<i32>,
        height: Option<i32>,
    ) -> Result<u64> {
        self.get_network_sol_ps(num_blocks, height).await
    }

    /// Returns data about each connected network node.
    ///
    /// zcashd reference: [`getpeerinfo`](https://zcash.github.io/rpc/getpeerinfo.html)
    /// method: post
    /// tags: network
    #[method(name = "getpeerinfo")]
    async fn get_peer_info(&self) -> Result<Vec<PeerInfo>>;

    /// Checks if a zcash transparent address of type P2PKH, P2SH or TEX is valid.
    /// Returns information about the given address if valid.
    ///
    /// zcashd reference: [`validateaddress`](https://zcash.github.io/rpc/validateaddress.html)
    /// method: post
    /// tags: util
    ///
    /// # Parameters
    ///
    /// - `address`: (string, required) The zcash address to validate.
    #[method(name = "validateaddress")]
    async fn validate_address(&self, address: String) -> Result<ValidateAddressResponse>;

    /// Checks if a zcash address of type P2PKH, P2SH, TEX, SAPLING or UNIFIED is valid.
    /// Returns information about the given address if valid.
    ///
    /// zcashd reference: [`z_validateaddress`](https://zcash.github.io/rpc/z_validateaddress.html)
    /// method: post
    /// tags: util
    ///
    /// # Parameters
    ///
    /// - `address`: (string, required) The zcash address to validate.
    ///
    /// # Notes
    ///
    /// - No notes
    #[method(name = "z_validateaddress")]
    async fn z_validate_address(&self, address: String) -> Result<ZValidateAddressResponse>;

    /// Returns the block subsidy reward of the block at `height`, taking into account the mining slow start.
    /// Returns an error if `height` is less than the height of the first halving for the current network.
    ///
    /// zcashd reference: [`getblocksubsidy`](https://zcash.github.io/rpc/getblocksubsidy.html)
    /// method: post
    /// tags: mining
    ///
    /// # Parameters
    ///
    /// - `height`: (numeric, optional, example=1) Can be any valid current or future height.
    ///
    /// # Notes
    ///
    /// If `height` is not supplied, uses the tip height.
    #[method(name = "getblocksubsidy")]
    async fn get_block_subsidy(&self, height: Option<u32>) -> Result<GetBlockSubsidyResponse>;

    /// Returns the proof-of-work difficulty as a multiple of the minimum difficulty.
    ///
    /// zcashd reference: [`getdifficulty`](https://zcash.github.io/rpc/getdifficulty.html)
    /// method: post
    /// tags: blockchain
    #[method(name = "getdifficulty")]
    async fn get_difficulty(&self) -> Result<f64>;

    /// Returns the list of individual payment addresses given a unified address.
    ///
    /// zcashd reference: [`z_listunifiedreceivers`](https://zcash.github.io/rpc/z_listunifiedreceivers.html)
    /// method: post
    /// tags: wallet
    ///
    /// # Parameters
    ///
    /// - `address`: (string, required) The zcash unified address to get the list from.
    ///
    /// # Notes
    ///
    /// - No notes
    #[method(name = "z_listunifiedreceivers")]
    async fn z_list_unified_receivers(
        &self,
        address: String,
    ) -> Result<ZListUnifiedReceiversResponse>;

    /// Invalidates a block if it is not yet finalized, removing it from the non-finalized
    /// state if it is present and rejecting it during contextual validation if it is submitted.
    ///
    /// # Parameters
    ///
    /// - `block_hash`: (hex-encoded block hash, required) The block hash to invalidate.
    // TODO: Invalidate block hashes even if they're not present in the non-finalized state (#9553).
    #[method(name = "invalidateblock")]
    async fn invalidate_block(&self, block_hash: block::Hash) -> Result<()>;

    /// Reconsiders a previously invalidated block if it exists in the cache of previously invalidated blocks.
    ///
    /// # Parameters
    ///
    /// - `block_hash`: (hex-encoded block hash, required) The block hash to reconsider.
    #[method(name = "reconsiderblock")]
    async fn reconsider_block(&self, block_hash: block::Hash) -> Result<Vec<block::Hash>>;

    #[method(name = "generate")]
    /// Mine blocks immediately. Returns the block hashes of the generated blocks.
    ///
    /// # Parameters
    ///
    /// - `num_blocks`: (numeric, required, example=1) Number of blocks to be generated.
    ///
    /// # Notes
    ///
    /// Only works if the network of the running zebrad process is `Regtest`.
    ///
    /// zcashd reference: [`generate`](https://zcash.github.io/rpc/generate.html)
    /// method: post
    /// tags: generating
    async fn generate(&self, num_blocks: u32) -> Result<Vec<GetBlockHashResponse>>;

    #[method(name = "addnode")]
    /// Add or remove a node from the address book.
    ///
    /// # Parameters
    ///
    /// - `addr`: (string, required) The address of the node to add or remove.
    /// - `command`: (string, required) The command to execute, either "add", "onetry", or "remove".
    ///
    /// # Notes
    ///
    /// Only the "add" command is currently supported.
    ///
    /// zcashd reference: [`addnode`](https://zcash.github.io/rpc/addnode.html)
    /// method: post
    /// tags: network
    async fn add_node(&self, addr: PeerSocketAddr, command: AddNodeCommand) -> Result<()>;

    // ==================== Botcash Social Protocol RPC Methods ====================

    /// Creates a social post on the Botcash network.
    ///
    /// This method creates a shielded transaction containing a social post message
    /// in the memo field. The post will be visible to anyone scanning the blockchain
    /// with the appropriate viewing keys.
    ///
    /// method: post
    /// tags: social
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The post request containing:
    ///   - `from`: (string) The sender's shielded address
    ///   - `content`: (string) The post content (max 500 bytes)
    ///   - `tags`: (array of strings, optional) Tags for categorization
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires wallet functionality to sign and broadcast the transaction.
    #[method(name = "z_socialpost")]
    async fn z_social_post(
        &self,
        request: types::social::SocialPostRequest,
    ) -> Result<types::social::SocialPostResponse>;

    /// Sends an encrypted direct message to another user.
    ///
    /// Creates a shielded transaction with the DM content encrypted in the memo field.
    /// Only the recipient can decrypt and read the message using their viewing key.
    ///
    /// method: post
    /// tags: social
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The DM request containing:
    ///   - `from`: (string) The sender's shielded address
    ///   - `to`: (string) The recipient's shielded address
    ///   - `content`: (string) The message content (max 500 bytes)
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    #[method(name = "z_socialdm")]
    async fn z_social_dm(
        &self,
        request: types::social::SocialDmRequest,
    ) -> Result<types::social::SocialDmResponse>;

    /// Creates a follow relationship to another user.
    ///
    /// This records the follow action on-chain, allowing indexers to build
    /// social graphs. The follow is recorded in a shielded transaction memo.
    ///
    /// method: post
    /// tags: social
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The follow request containing:
    ///   - `from`: (string) The follower's shielded address
    ///   - `target`: (string) The address to follow
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    #[method(name = "z_socialfollow")]
    async fn z_social_follow(
        &self,
        request: types::social::SocialFollowRequest,
    ) -> Result<types::social::SocialFollowResponse>;

    /// Retrieves social posts visible to the provided viewing keys.
    ///
    /// Scans the blockchain for social messages (posts, comments, DMs, etc.)
    /// that are decryptable with the provided incoming viewing keys (IVKs).
    ///
    /// method: post
    /// tags: social
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The feed request containing:
    ///   - `ivks`: (array of strings) Incoming viewing keys to scan with
    ///   - `limit`: (number, optional, default=50) Maximum posts to return
    ///   - `startHeight`: (number, optional) Block height to start scanning from
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Scanning large ranges may be slow; consider using an indexer service.
    #[method(name = "z_socialfeed")]
    async fn z_social_feed(
        &self,
        request: types::social::SocialFeedRequest,
    ) -> Result<types::social::SocialFeedResponse>;

    // ==================== Botcash Attention Market RPC Methods ====================

    /// Boosts content visibility in the attention market.
    ///
    /// Pays BCASH to boost the visibility of content in the market feeds.
    /// The payment contributes to the sender's credit pool for redistribution.
    ///
    /// method: post
    /// tags: attention
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The boost request containing:
    ///   - `from`: (string) The sender's shielded address
    ///   - `targetTxid`: (string) The transaction ID of content to boost
    ///   - `amount`: (number) Amount in zatoshis to spend on the boost
    ///   - `durationBlocks`: (number, optional, default=1440) Duration in blocks (~1 day)
    ///   - `category`: (number, optional) Category code (0-255)
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires wallet functionality to sign and broadcast the transaction.
    #[method(name = "z_attentionboost")]
    async fn z_attention_boost(
        &self,
        request: types::social::AttentionBoostRequest,
    ) -> Result<types::social::AttentionBoostResponse>;

    /// Tips content using credits instead of BCASH.
    ///
    /// Credits are earned from previous attention market payments and expire after 7 days.
    /// When tipping with credits, the recipient receives real BCASH from the pool.
    ///
    /// method: post
    /// tags: attention
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The tip request containing:
    ///   - `from`: (string) The sender's shielded address
    ///   - `targetTxid`: (string) The transaction ID of content to tip
    ///   - `creditAmount`: (number) Amount of credits to tip (in zatoshis)
    ///   - `message`: (string, optional) Message to include with the tip
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires wallet functionality and sufficient credit balance.
    #[method(name = "z_credittip")]
    async fn z_credit_tip(
        &self,
        request: types::social::CreditTipRequest,
    ) -> Result<types::social::CreditTipResponse>;

    /// Gets the credit balance for an address.
    ///
    /// Returns the available credit balance, credits expiring soon, and individual grants.
    /// Credits are earned from attention market payments and expire after 7 days.
    ///
    /// method: post
    /// tags: attention
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The balance request containing:
    ///   - `address`: (string) The address to check credit balance for
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Typically requires an indexer to track credit balances.
    #[method(name = "z_creditbalance")]
    async fn z_credit_balance(
        &self,
        request: types::social::CreditBalanceRequest,
    ) -> Result<types::social::CreditBalanceResponse>;

    /// Gets the attention market feed.
    ///
    /// Returns content ordered by the specified algorithm (hot, top, new, or boosted).
    /// Hot feed uses time-decayed AU scores; top uses all-time AU; boosted shows only active boosts.
    ///
    /// method: post
    /// tags: attention
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The feed request containing:
    ///   - `feedType`: (string, optional, default="hot") One of: "hot", "top", "new", "boosted"
    ///   - `category`: (number, optional) Filter by category code (0-255)
    ///   - `limit`: (number, optional, default=50) Maximum items to return
    ///   - `offset`: (number, optional, default=0) Pagination offset
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Typically requires an indexer to compute rankings efficiently.
    #[method(name = "z_marketfeed")]
    async fn z_market_feed(
        &self,
        request: types::social::MarketFeedRequest,
    ) -> Result<types::social::MarketFeedResponse>;

    /// Gets statistics for an attention market epoch.
    ///
    /// Returns total payments, participant count, and redistribution info for the epoch.
    /// Each epoch is 1440 blocks (~1 day at 60s block time).
    ///
    /// method: post
    /// tags: attention
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The stats request containing:
    ///   - `epochNumber`: (number, optional) The epoch to query. If not specified, returns current epoch.
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Typically requires an indexer to track epoch statistics.
    #[method(name = "z_epochstats")]
    async fn z_epoch_stats(
        &self,
        request: types::social::EpochStatsRequest,
    ) -> Result<types::social::EpochStatsResponse>;

    // ==================== Botcash Batch Queue RPC Methods ====================

    /// Queues social actions for batching.
    ///
    /// Actions are accumulated locally until sent with z_batchsend or auto-sent
    /// when the queue reaches MAX_BATCH_QUEUE_SIZE (5) if auto_send is enabled.
    /// Batching reduces fees by combining multiple actions into a single transaction.
    ///
    /// method: post
    /// tags: batch
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The queue request containing:
    ///   - `from`: (string) The sender's shielded address
    ///   - `actions`: (array) Actions to queue (post, dm, follow, etc.)
    ///   - `autoSend`: (boolean, optional, default=false) Auto-send when queue is full
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Maximum 5 actions can be batched per transaction.
    #[method(name = "z_batchqueue")]
    async fn z_batch_queue(
        &self,
        request: types::social::BatchQueueRequest,
    ) -> Result<types::social::BatchQueueResponse>;

    /// Sends the current batch queue as a single transaction.
    ///
    /// Creates a batched transaction containing all queued actions for the address.
    /// The queue is cleared after successful submission.
    ///
    /// method: post
    /// tags: batch
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The send request containing:
    ///   - `from`: (string) The sender's address (must match queued actions)
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Returns an error if the queue is empty.
    #[method(name = "z_batchsend")]
    async fn z_batch_send(
        &self,
        request: types::social::BatchSendRequest,
    ) -> Result<types::social::BatchSendResponse>;

    /// Gets the current batch queue status for an address.
    ///
    /// Returns information about queued actions including count, types, and size.
    ///
    /// method: post
    /// tags: batch
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The status request containing:
    ///   - `from`: (string) The address to check queue status for
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    #[method(name = "z_batchstatus")]
    async fn z_batch_status(
        &self,
        request: types::social::BatchStatusRequest,
    ) -> Result<types::social::BatchStatusResponse>;

    /// Clears the batch queue for an address.
    ///
    /// Removes all queued actions without sending them.
    ///
    /// method: post
    /// tags: batch
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The clear request containing:
    ///   - `from`: (string) The address whose queue should be cleared
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    #[method(name = "z_batchclear")]
    async fn z_batch_clear(
        &self,
        request: types::social::BatchClearRequest,
    ) -> Result<types::social::BatchClearResponse>;

    // ==================== Botcash Governance RPC Methods ====================

    /// Creates a new governance proposal.
    ///
    /// Proposals require a minimum deposit (default 10 BCASH) that is returned
    /// if the proposal receives sufficient support (>10% of votes). Proposals
    /// go through three phases: PROPOSAL (7 days for discussion), VOTING (14 days),
    /// and EXECUTION (30-day timelock if passed).
    ///
    /// method: post
    /// tags: governance
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The proposal request containing:
    ///   - `from`: (string) The proposer's unified or shielded address
    ///   - `proposalType`: (string, optional) Type: "parameter", "upgrade", "spending", "other"
    ///   - `title`: (string) Proposal title (max 255 chars)
    ///   - `description`: (string) Full proposal description
    ///   - `parameters`: (array, optional) Parameter changes for "parameter" proposals
    ///   - `deposit`: (u64, optional) Deposit amount in zatoshis (default: 10 BCASH)
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Creates a GovernanceProposal (0xE1) message in the memo field.
    /// See specs/governance.md for the full governance system design.
    #[method(name = "z_governancepropose")]
    async fn z_governance_propose(
        &self,
        request: types::social::GovernanceProposalRequest,
    ) -> Result<types::social::GovernanceProposalResponse>;

    /// Casts a vote on an existing governance proposal.
    ///
    /// Voting power is calculated based on karma (social reputation) and BCASH
    /// balance using the formula: sqrt(karma) + sqrt(bcash_balance). Votes can
    /// be changed until the voting period ends.
    ///
    /// method: post
    /// tags: governance
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The vote request containing:
    ///   - `from`: (string) The voter's unified or shielded address
    ///   - `proposalId`: (string) The proposal ID to vote on (hex-encoded, 32 bytes)
    ///   - `vote`: (string) Vote choice: "yes", "no", or "abstain"
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Creates a GovernanceVote (0xE0) message in the memo field.
    /// Abstain votes count towards quorum but not towards the approval threshold.
    #[method(name = "z_governancevote")]
    async fn z_governance_vote(
        &self,
        request: types::social::GovernanceVoteRequest,
    ) -> Result<types::social::GovernanceVoteResponse>;

    /// Gets the current status of a governance proposal.
    ///
    /// Returns detailed information about a proposal including vote tallies,
    /// quorum progress, approval percentage, and execution status.
    ///
    /// method: post
    /// tags: governance
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The status request containing:
    ///   - `proposalId`: (string) The proposal ID to query (hex-encoded, 32 bytes)
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires an indexer to track proposal and vote transactions.
    /// Status values: "pending", "voting", "passed", "rejected", "executed".
    #[method(name = "z_governancestatus")]
    async fn z_governance_status(
        &self,
        request: types::social::GovernanceProposalStatusRequest,
    ) -> Result<types::social::GovernanceProposalStatusResponse>;

    /// Lists governance proposals with optional filtering.
    ///
    /// Returns a paginated list of proposals matching the specified criteria.
    /// Proposals are ordered by creation height (newest first).
    ///
    /// method: post
    /// tags: governance
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The list request containing:
    ///   - `status`: (string, optional) Filter: "all", "pending", "voting", "passed", "rejected", "executed"
    ///   - `limit`: (u32, optional) Maximum proposals to return (default: 50)
    ///   - `offset`: (u32, optional) Pagination offset (default: 0)
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires an indexer to track proposal transactions.
    #[method(name = "z_governancelist")]
    async fn z_governance_list(
        &self,
        request: types::social::GovernanceListRequest,
    ) -> Result<types::social::GovernanceListResponse>;

    // ==================== Channel RPC Methods (Layer-2 Social Channels) ====================

    /// Opens a new Layer-2 social channel between parties.
    ///
    /// Channels enable high-frequency off-chain messaging (chat, group DM, thread
    /// replies) while maintaining the security of on-chain settlement. The initiator
    /// must provide a deposit that is locked until the channel is settled.
    ///
    /// # method: post
    /// # tags: channel
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The channel open request containing:
    ///   - `from`: (string) The initiator's unified or shielded address
    ///   - `parties`: (array) List of party addresses for the channel
    ///   - `deposit`: (u64) Total deposit in zatoshis
    ///   - `timeoutBlocks`: (u32, optional) Timeout before unilateral settlement (default: 1440)
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `channelId`: (string) Unique channel identifier (32 bytes hex-encoded)
    /// - `txid`: (string) Transaction ID that opened the channel
    /// - `openedAtBlock`: (u32) Block height when the channel was opened
    /// - `timeoutBlock`: (u32) Block height when unilateral settlement becomes available
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires wallet support for transaction creation and signing.
    #[method(name = "z_channelopen")]
    async fn z_channel_open(
        &self,
        request: types::social::ChannelOpenRequest,
    ) -> Result<types::social::ChannelOpenResponse>;

    /// Closes a channel cooperatively with agreement from all parties.
    ///
    /// A cooperative close immediately returns deposits to parties. All parties
    /// must have signed the final state. If any party is unresponsive, use
    /// `z_channelsettle` after the timeout period.
    ///
    /// # method: post
    /// # tags: channel
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The channel close request containing:
    ///   - `from`: (string) The closer's address (must be a channel party)
    ///   - `channelId`: (string) Channel ID to close (32 bytes hex-encoded)
    ///   - `finalSeq`: (u32) Final sequence number of off-chain messages
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) Transaction ID of the close
    /// - `channelId`: (string) The closed channel ID
    /// - `finalSeq`: (u32) The final sequence number
    /// - `cooperative`: (bool) True if all parties agreed
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires wallet support for transaction creation and signing.
    #[method(name = "z_channelclose")]
    async fn z_channel_close(
        &self,
        request: types::social::ChannelCloseRequest,
    ) -> Result<types::social::ChannelCloseResponse>;

    /// Settles a channel with final state and message hash.
    ///
    /// Settlement can be cooperative (immediate) or unilateral (after timeout).
    /// The message hash is a Merkle root of all off-chain messages, enabling
    /// dispute resolution if needed.
    ///
    /// # method: post
    /// # tags: channel
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The settlement request containing:
    ///   - `from`: (string) The settler's address (must be a channel party)
    ///   - `channelId`: (string) Channel ID to settle (32 bytes hex-encoded)
    ///   - `finalSeq`: (u32) Final sequence number
    ///   - `messageHash`: (string) Merkle root of messages (32 bytes hex-encoded)
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) Transaction ID of the settlement
    /// - `channelId`: (string) The settled channel ID
    /// - `finalSeq`: (u32) The final sequence number
    /// - `finalBalances`: (object) Map of address to final balance in zatoshis
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires wallet support for transaction creation and signing.
    #[method(name = "z_channelsettle")]
    async fn z_channel_settle(
        &self,
        request: types::social::ChannelSettleRequest,
    ) -> Result<types::social::ChannelSettleResponse>;

    /// Gets the current status of a channel.
    ///
    /// Returns detailed information about a channel including its state,
    /// parties, deposit amount, and message count.
    ///
    /// # method: post
    /// # tags: channel
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The status request containing:
    ///   - `channelId`: (string) Channel ID to query (32 bytes hex-encoded)
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `channelId`: (string) The channel ID
    /// - `state`: (string) Current state: "open", "closing", "settled", "disputed"
    /// - `parties`: (array) List of party addresses
    /// - `deposit`: (u64) Total deposit in zatoshis
    /// - `currentSeq`: (u32) Current sequence number
    /// - `openedAtBlock`: (u32) Block when opened
    /// - `timeoutBlock`: (u32) Block when unilateral settlement allowed
    /// - `latestMessageHash`: (string, optional) Latest message Merkle root
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires an indexer to track channel state.
    #[method(name = "z_channelstatus")]
    async fn z_channel_status(
        &self,
        request: types::social::ChannelStatusRequest,
    ) -> Result<types::social::ChannelStatusResponse>;

    /// Lists channels for an address.
    ///
    /// Returns a paginated list of channels where the given address is a party.
    /// Can be filtered by channel state.
    ///
    /// # method: post
    /// # tags: channel
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The list request containing:
    ///   - `address`: (string) Address to list channels for
    ///   - `state`: (string, optional) Filter by state: "open", "closing", "settled", "disputed"
    ///   - `limit`: (u32, optional) Maximum channels to return (default: 50)
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `channels`: (array) List of channel summaries
    /// - `totalCount`: (u32) Total matching channels
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires an indexer to track channel state.
    #[method(name = "z_channellist")]
    async fn z_channel_list(
        &self,
        request: types::social::ChannelListRequest,
    ) -> Result<types::social::ChannelListResponse>;

    // ==================== Recovery RPC Methods (Social Recovery) ====================

    /// Configures social recovery for an address.
    ///
    /// Designates trusted guardians who can collectively help recover an account
    /// if the owner loses access to their keys. Uses Shamir's Secret Sharing to
    /// split recovery keys among guardians.
    ///
    /// # method: post
    /// # tags: recovery
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The recovery config request containing:
    ///   - `from`: (string) The owner's unified or shielded address
    ///   - `guardians`: (array) List of guardian addresses (1-15 guardians)
    ///   - `threshold`: (usize) Number of guardians required to recover (M of N)
    ///   - `timelockBlocks`: (u32, optional) Time delay before recovery completes (default: 10080 = ~7 days)
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) Transaction ID of the recovery config
    /// - `recoveryId`: (string) Unique recovery configuration ID
    /// - `guardianCount`: (usize) Number of guardians registered
    /// - `threshold`: (usize) Required approvals for recovery
    /// - `timelockBlocks`: (u32) Time delay in blocks
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Creates a RecoveryConfig (0xF0) message in the memo field.
    /// See specs/recovery.md for the full social recovery design.
    #[method(name = "z_recoveryconfig")]
    async fn z_recovery_config(
        &self,
        request: types::social::RecoveryConfigRequest,
    ) -> Result<types::social::RecoveryConfigResponse>;

    /// Initiates a recovery request for an account.
    ///
    /// Called from a new device when the owner has lost access to their keys.
    /// Starts the recovery process which requires guardian approvals and a
    /// time-locked waiting period.
    ///
    /// # method: post
    /// # tags: recovery
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The recovery request containing:
    ///   - `from`: (string) The new device's address (initiating recovery)
    ///   - `targetAddress`: (string) The address being recovered
    ///   - `newPubkey`: (string) The new public key to rotate to (hex-encoded)
    ///   - `proof`: (string) Signed challenge proving knowledge of identity
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) Transaction ID of the recovery request
    /// - `recoveryId`: (string) The recovery configuration ID
    /// - `requestId`: (string) Unique ID for this recovery request
    /// - `timelockExpiresBlock`: (u32) Block when timelock expires
    /// - `approvalsNeeded`: (usize) Number of guardian approvals needed
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Creates a RecoveryRequest (0xF1) message in the memo field.
    /// The original owner can cancel the request during the timelock period.
    #[method(name = "z_recoveryrequest")]
    async fn z_recovery_request(
        &self,
        request: types::social::RecoveryRequestRequest,
    ) -> Result<types::social::RecoveryRequestResponse>;

    /// Approves a pending recovery request as a guardian.
    ///
    /// Guardians verify the requester's identity out-of-band (video call, secret
    /// question, etc.) before approving. The guardian submits their encrypted
    /// Shamir share to enable key reconstruction.
    ///
    /// # method: post
    /// # tags: recovery
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The approval request containing:
    ///   - `from`: (string) The guardian's address
    ///   - `requestId`: (string) The recovery request ID to approve
    ///   - `encryptedShare`: (string) Guardian's Shamir share encrypted to new pubkey
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) Transaction ID of the approval
    /// - `approvalsCount`: (usize) Total approvals received
    /// - `approvalsNeeded`: (usize) Approvals still needed
    /// - `thresholdMet`: (bool) Whether threshold has been reached
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Creates a RecoveryApprove (0xF2) message in the memo field.
    /// Guardians can revoke their approval within the timelock period.
    #[method(name = "z_recoveryapprove")]
    async fn z_recovery_approve(
        &self,
        request: types::social::RecoveryApproveRequest,
    ) -> Result<types::social::RecoveryApproveResponse>;

    /// Cancels an active recovery request.
    ///
    /// Can only be called by the original owner of the address being recovered.
    /// Used to stop unauthorized recovery attempts. Must be called before the
    /// timelock expires and recovery is executed.
    ///
    /// # method: post
    /// # tags: recovery
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The cancel request containing:
    ///   - `from`: (string) The original owner's address
    ///   - `requestId`: (string) The recovery request ID to cancel
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) Transaction ID of the cancellation
    /// - `cancelled`: (bool) Whether the cancellation was successful
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Creates a RecoveryCancel (0xF3) message in the memo field.
    /// Only the original owner can cancel a recovery request.
    #[method(name = "z_recoverycancel")]
    async fn z_recovery_cancel(
        &self,
        request: types::social::RecoveryCancelRequest,
    ) -> Result<types::social::RecoveryCancelResponse>;

    /// Gets the recovery status for an address.
    ///
    /// Returns information about the recovery configuration and any pending
    /// recovery requests for the specified address.
    ///
    /// # method: post
    /// # tags: recovery
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The status request containing:
    ///   - `address`: (string) The address to check recovery status for
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `hasRecovery`: (bool) Whether recovery is configured
    /// - `guardianCount`: (usize, optional) Number of guardians if configured
    /// - `threshold`: (usize, optional) Required approvals if configured
    /// - `timelockBlocks`: (u32, optional) Timelock duration if configured
    /// - `status`: (string) Current status: "active", "pending", "approved", etc.
    /// - `pendingRequest`: (object, optional) Details of any pending recovery request
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires an indexer to track recovery state.
    #[method(name = "z_recoverystatus")]
    async fn z_recovery_status(
        &self,
        request: types::social::RecoveryStatusRequest,
    ) -> Result<types::social::RecoveryStatusResponse>;

    /// Lists guardians for an address's recovery configuration.
    ///
    /// Returns the list of guardian addresses and their status for a given
    /// recovery configuration.
    ///
    /// # method: post
    /// # tags: recovery
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The list request containing:
    ///   - `address`: (string) The address to list guardians for
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `address`: (string) The address queried
    /// - `guardians`: (array) List of guardian summaries with address and active status
    /// - `threshold`: (usize) Required approvals for recovery
    /// - `timelockBlocks`: (u32) Timelock duration in blocks
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Not available in zcashd.
    /// Requires an indexer to track guardian state.
    #[method(name = "z_recoveryguardians")]
    async fn z_recovery_guardians(
        &self,
        request: types::social::GuardianListRequest,
    ) -> Result<types::social::GuardianListResponse>;

    // ==================== Multi-Sig Identity Methods ====================

    /// Sets up a multi-sig identity for an address.
    ///
    /// Configures an address to require M-of-N signatures for all future
    /// social actions. Suitable for high-value accounts (influencers,
    /// businesses, agents with significant stake).
    ///
    /// # method: post
    /// # tags: multisig
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The setup request containing:
    ///   - `address`: (string) The address to configure as multi-sig
    ///   - `publicKeys`: (array) Compressed public keys (33 bytes hex each, 2-15 keys)
    ///   - `threshold`: (number) Signatures required (1 to key count)
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) The transaction ID of the setup
    /// - `address`: (string) The multi-sig address
    /// - `keyCount`: (number) Number of keys
    /// - `threshold`: (number) Signatures required
    /// - `setupBlock`: (number) Block height when setup was submitted
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires wallet support.
    /// Once set up, all social actions from this address require multi-sig.
    #[method(name = "z_multisigsetup")]
    async fn z_multisig_setup(
        &self,
        request: types::social::MultisigSetupRequest,
    ) -> Result<types::social::MultisigSetupResponse>;

    /// Performs a social action with multi-sig authorization.
    ///
    /// Wraps a social action (post, follow, etc.) with the required
    /// signatures from the multi-sig key holders.
    ///
    /// # method: post
    /// # tags: multisig
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The action request containing:
    ///   - `multisigAddress`: (string) The multi-sig address performing the action
    ///   - `actionType`: (string) The action type ("post", "follow", "dm", etc.)
    ///   - `actionPayload`: (object) The action-specific payload
    ///   - `signatures`: (array) Array of signatures, each with:
    ///     - `keyIndex`: (number) Index of signing key (0-based)
    ///     - `signature`: (string) Schnorr signature (64 bytes hex)
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) The transaction ID of the action
    /// - `multisigAddress`: (string) The address that performed the action
    /// - `actionType`: (string) The action type
    /// - `signatureCount`: (number) Number of signatures used
    /// - `actionBlock`: (number) Block height when submitted
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires wallet support
    /// and coordination between key holders to gather signatures.
    #[method(name = "z_multisigaction")]
    async fn z_multisig_action(
        &self,
        request: types::social::MultisigActionRequest,
    ) -> Result<types::social::MultisigActionResponse>;

    /// Gets the multi-sig status of an address.
    ///
    /// Returns information about whether an address is configured as
    /// a multi-sig identity and its configuration details.
    ///
    /// # method: post
    /// # tags: multisig
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The status request containing:
    ///   - `address`: (string) The address to check
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `address`: (string) The queried address
    /// - `isMultisig`: (bool) Whether configured as multi-sig
    /// - `keyCount`: (number, optional) Number of keys (if multi-sig)
    /// - `threshold`: (number, optional) Required signatures (if multi-sig)
    /// - `setupBlock`: (number, optional) Setup block height (if multi-sig)
    /// - `publicKeys`: (array, optional) The public keys (if multi-sig)
    /// - `status`: (string) Status ("active", "pending", "notmultisig", "revoked")
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires an indexer.
    #[method(name = "z_multisigstatus")]
    async fn z_multisig_status(
        &self,
        request: types::social::MultisigStatusRequest,
    ) -> Result<types::social::MultisigStatusResponse>;

    /// Lists multi-sig identities.
    ///
    /// Returns a paginated list of multi-sig identity addresses
    /// with optional filtering by status.
    ///
    /// # method: post
    /// # tags: multisig
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The list request containing:
    ///   - `status`: (string, optional) Filter by status
    ///   - `limit`: (number, optional) Max results (default 50)
    ///   - `offset`: (number, optional) Pagination offset
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `identities`: (array) List of multi-sig summaries
    /// - `totalCount`: (number) Total matching identities
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires an indexer.
    #[method(name = "z_multisiglist")]
    async fn z_multisig_list(
        &self,
        request: types::social::MultisigListRequest,
    ) -> Result<types::social::MultisigListResponse>;

    // ==================== Bridge Methods ====================

    /// Links an external platform identity to a Botcash address.
    ///
    /// Creates an on-chain identity link between a Botcash address and an
    /// external platform account (Telegram, Discord, Nostr, Mastodon, Twitter).
    /// Requires a signed proof of ownership from both identities.
    ///
    /// # method: post
    /// # tags: bridge
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The link request containing:
    ///   - `from`: (string) The Botcash address to link
    ///   - `platform`: (string) The platform ("telegram", "discord", "nostr", "mastodon", "twitter")
    ///   - `platformId`: (string) The platform-specific user identifier
    ///   - `proof`: (string) Signed challenge proving ownership (hex-encoded)
    ///   - `privacyMode`: (string, optional) Privacy mode ("full", "selective", "readonly", "private")
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) The transaction ID of the link
    /// - `platform`: (string) The linked platform
    /// - `platformId`: (string) The linked platform user ID
    /// - `address`: (string) The linked Botcash address
    /// - `status`: (string) Current link status
    /// - `linkedAtBlock`: (u32) Block height when linked
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension for cross-platform identity bridging.
    /// Requires wallet support to sign and submit the link transaction.
    #[method(name = "z_bridgelink")]
    async fn z_bridge_link(
        &self,
        request: types::social::BridgeLinkRequest,
    ) -> Result<types::social::BridgeLinkResponse>;

    /// Unlinks an external platform identity from a Botcash address.
    ///
    /// Removes an existing identity link. Only the owner of the Botcash
    /// address can unlink.
    ///
    /// # method: post
    /// # tags: bridge
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The unlink request containing:
    ///   - `from`: (string) The Botcash address that owns the link
    ///   - `platform`: (string) The platform to unlink
    ///   - `platformId`: (string) The platform user ID to unlink
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) The transaction ID of the unlink
    /// - `platform`: (string) The unlinked platform
    /// - `platformId`: (string) The unlinked platform user ID
    /// - `success`: (bool) Whether the unlink was successful
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires wallet support.
    #[method(name = "z_bridgeunlink")]
    async fn z_bridge_unlink(
        &self,
        request: types::social::BridgeUnlinkRequest,
    ) -> Result<types::social::BridgeUnlinkResponse>;

    /// Posts content from an external platform to Botcash via a bridge.
    ///
    /// Creates a cross-post that includes attribution to the original
    /// platform and post. Used by bridge services to relay content.
    ///
    /// # method: post
    /// # tags: bridge
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The post request containing:
    ///   - `from`: (string) The Botcash address to post from
    ///   - `platform`: (string) The source platform
    ///   - `originalId`: (string) The original post ID on the source platform
    ///   - `content`: (string) The content to post
    ///   - `inReplyTo`: (string, optional) Transaction ID if this is a reply
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `txid`: (string) The transaction ID of the post
    /// - `platform`: (string) The source platform
    /// - `originalId`: (string) The original post ID
    /// - `postedAtBlock`: (u32) Block height when posted
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension for bridge operators.
    /// The posting address must have an active link to the source platform.
    #[method(name = "z_bridgepost")]
    async fn z_bridge_post(
        &self,
        request: types::social::BridgePostRequest,
    ) -> Result<types::social::BridgePostResponse>;

    /// Queries the bridge link status for a Botcash address.
    ///
    /// Returns information about all platform links for the specified address,
    /// including link status, privacy mode, and activity statistics.
    ///
    /// # method: post
    /// # tags: bridge
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The status request containing:
    ///   - `address`: (string) The Botcash address to query
    ///   - `platform`: (string, optional) Filter by specific platform
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `address`: (string) The queried address
    /// - `links`: (array) List of bridge links with status and statistics
    /// - `activeLinksCount`: (u32) Number of active links
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires indexer support.
    #[method(name = "z_bridgestatus")]
    async fn z_bridge_status(
        &self,
        request: types::social::BridgeStatusRequest,
    ) -> Result<types::social::BridgeStatusResponse>;

    /// Lists all bridge links matching the filter criteria.
    ///
    /// Returns a paginated list of bridge links across all addresses,
    /// useful for bridge operators and indexers.
    ///
    /// # method: post
    /// # tags: bridge
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The list request containing:
    ///   - `platform`: (string, optional) Filter by platform
    ///   - `status`: (string, optional) Filter by link status
    ///   - `limit`: (u32, optional) Maximum results (default 100)
    ///   - `offset`: (u32, optional) Pagination offset
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `links`: (array) List of bridge link summaries
    /// - `totalCount`: (u32) Total matching links
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires indexer support.
    #[method(name = "z_bridgelist")]
    async fn z_bridge_list(
        &self,
        request: types::social::BridgeListRequest,
    ) -> Result<types::social::BridgeListResponse>;

    /// Gets a verification challenge for proving bridge identity ownership.
    ///
    /// Generates a challenge that must be signed on both the Botcash
    /// address and the external platform to prove ownership.
    ///
    /// # method: post
    /// # tags: bridge
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The verify request containing:
    ///   - `address`: (string) The Botcash address requesting verification
    ///   - `platform`: (string) The platform to verify
    ///   - `platformId`: (string) The platform user ID to verify
    ///
    /// # Returns
    ///
    /// An object containing:
    /// - `challenge`: (string) The challenge to sign (hex-encoded)
    /// - `expiresAt`: (i64) Unix timestamp when the challenge expires
    /// - `instructions`: (string) Human-readable signing instructions
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Challenges expire after 10 minutes.
    #[method(name = "z_bridgeverify")]
    async fn z_bridge_verify(
        &self,
        request: types::social::BridgeVerifyRequest,
    ) -> Result<types::social::BridgeVerifyResponse>;

    // ==================== Moderation RPC Methods (Trust & Reports) ====================

    /// Creates or updates a trust relationship with another user.
    ///
    /// Trust relationships form a web of trust for reputation. Users can mark
    /// others as trusted, neutral, or distrusted with an optional reason.
    /// Trust propagates through the social graph with decay.
    ///
    /// # method: post
    /// # tags: moderation
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The trust request containing:
    ///   - `from`: (string) The truster's address
    ///   - `target`: (string) The address to trust/distrust
    ///   - `level`: (string) Trust level: "trusted", "neutral", or "distrust"
    ///   - `reason`: (string, optional) Reason for the trust decision (max 200 chars)
    ///
    /// # Returns
    ///
    /// A `TrustResponse` object containing:
    /// - `txid`: (string) The trust transaction ID
    /// - `target`: (string) The trusted/distrusted address
    /// - `level`: (string) The trust level set
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension for the reputation system.
    /// Requires wallet support to sign and submit the trust transaction.
    #[method(name = "z_trust")]
    async fn z_trust(
        &self,
        request: types::social::TrustRequest,
    ) -> Result<types::social::TrustResponse>;

    /// Queries trust relationships for an address.
    ///
    /// Returns incoming and/or outgoing trust relationships, along with
    /// computed trust scores based on the web of trust.
    ///
    /// # method: post
    /// # tags: moderation
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The query request containing:
    ///   - `address`: (string) The address to query trust for
    ///   - `includeIncoming`: (bool, optional) Include trusts received (default: true)
    ///   - `includeOutgoing`: (bool, optional) Include trusts given (default: true)
    ///   - `limit`: (u32, optional) Maximum results (default: 100, max: 1000)
    ///
    /// # Returns
    ///
    /// A `TrustQueryResponse` object containing:
    /// - `address`: (string) The queried address
    /// - `trustScore`: (i64) Computed trust score
    /// - `trustedByCount`: (u32) Number of addresses that trust this address
    /// - `distrustedByCount`: (u32) Number of addresses that distrust this address
    /// - `trusts`: (array) List of TrustSummary objects
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires indexer support.
    #[method(name = "z_trustquery")]
    async fn z_trust_query(
        &self,
        request: types::social::TrustQueryRequest,
    ) -> Result<types::social::TrustQueryResponse>;

    /// Submits a stake-weighted report against content.
    ///
    /// Reports require staking BCASH as collateral. Valid reports return stake
    /// plus reward; false reports forfeit stake. This prevents report spam
    /// while incentivizing honest reporting.
    ///
    /// # method: post
    /// # tags: moderation
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The report request containing:
    ///   - `from`: (string) The reporter's address
    ///   - `targetTxid`: (string) Transaction ID of content to report
    ///   - `category`: (string) Report category: "spam", "scam", "harassment", "illegal", "other"
    ///   - `stake`: (u64) BCASH stake in zatoshi (minimum 1,000,000 = 0.01 BCASH)
    ///   - `evidence`: (string, optional) Evidence text (max 300 chars)
    ///
    /// # Returns
    ///
    /// A `ReportResponse` object containing:
    /// - `txid`: (string) The report transaction ID
    /// - `targetTxid`: (string) The reported content's transaction ID
    /// - `category`: (string) The report category
    /// - `stake`: (u64) The staked amount
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension for content moderation.
    /// Requires wallet support to sign and submit the report transaction.
    #[method(name = "z_report")]
    async fn z_report(
        &self,
        request: types::social::ReportRequest,
    ) -> Result<types::social::ReportResponse>;

    /// Gets the status of a submitted report.
    ///
    /// Returns the current status of a report including whether it has been
    /// validated, rejected, or is still pending review.
    ///
    /// # method: post
    /// # tags: moderation
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The status request containing:
    ///   - `reportTxid`: (string) Transaction ID of the report
    ///
    /// # Returns
    ///
    /// A `ReportStatusResponse` object containing:
    /// - `reportTxid`: (string) The report transaction ID
    /// - `targetTxid`: (string) The reported content's transaction ID
    /// - `category`: (string) The report category
    /// - `stake`: (u64) The staked amount
    /// - `status`: (string) Current status: "pending", "validated", "rejected", "expired"
    /// - `blockHeight`: (u32) Block height when report was submitted
    /// - `resolution`: (string, optional) Resolution details if decided
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires indexer support.
    #[method(name = "z_reportstatus")]
    async fn z_report_status(
        &self,
        request: types::social::ReportStatusRequest,
    ) -> Result<types::social::ReportStatusResponse>;

    /// Lists reports matching filter criteria.
    ///
    /// Returns a paginated list of reports, optionally filtered by target,
    /// reporter, category, or status.
    ///
    /// # method: post
    /// # tags: moderation
    ///
    /// # Parameters
    ///
    /// - `request`: (object, required) The list request containing:
    ///   - `targetTxid`: (string, optional) Filter by reported content
    ///   - `reporterAddress`: (string, optional) Filter by reporter
    ///   - `category`: (string, optional) Filter by category
    ///   - `status`: (string, optional) Filter by status
    ///   - `limit`: (u32, optional) Maximum results (default: 50, max: 1000)
    ///
    /// # Returns
    ///
    /// A `ReportListResponse` object containing:
    /// - `reports`: (array) List of ReportSummary objects
    /// - `totalCount`: (u32) Total matching reports (may exceed limit)
    ///
    /// # Notes
    ///
    /// This is a Botcash-specific extension. Requires indexer support.
    #[method(name = "z_reportlist")]
    async fn z_report_list(
        &self,
        request: types::social::ReportListRequest,
    ) -> Result<types::social::ReportListResponse>;
}

/// RPC method implementations.
#[derive(Clone)]
pub struct RpcImpl<Mempool, State, ReadState, Tip, AddressBook, BlockVerifierRouter, SyncStatus>
where
    Mempool: Service<
            mempool::Request,
            Response = mempool::Response,
            Error = zebra_node_services::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    Mempool::Future: Send,
    State: Service<
            zebra_state::Request,
            Response = zebra_state::Response,
            Error = zebra_state::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    State::Future: Send,
    ReadState: Service<
            zebra_state::ReadRequest,
            Response = zebra_state::ReadResponse,
            Error = zebra_state::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    ReadState::Future: Send,
    Tip: ChainTip + Clone + Send + Sync + 'static,
    AddressBook: AddressBookPeers + Clone + Send + Sync + 'static,
    BlockVerifierRouter: Service<zebra_consensus::Request, Response = block::Hash, Error = zebra_consensus::BoxError>
        + Clone
        + Send
        + Sync
        + 'static,
    <BlockVerifierRouter as Service<zebra_consensus::Request>>::Future: Send,
    SyncStatus: ChainSyncStatus + Clone + Send + Sync + 'static,
{
    // Configuration
    //
    /// Zebra's application version, with build metadata.
    build_version: String,

    /// Zebra's RPC user agent.
    user_agent: String,

    /// The configured network for this RPC service.
    network: Network,

    /// Test-only option that makes Zebra say it is at the chain tip,
    /// no matter what the estimated height or local clock is.
    debug_force_finished_sync: bool,

    // Services
    //
    /// A handle to the mempool service.
    mempool: Mempool,

    /// A handle to the state service.
    state: State,

    /// A handle to the state service.
    read_state: ReadState,

    /// Allows efficient access to the best tip of the blockchain.
    latest_chain_tip: Tip,

    // Tasks
    //
    /// A sender component of a channel used to send transactions to the mempool queue.
    queue_sender: broadcast::Sender<UnminedTx>,

    /// Peer address book.
    address_book: AddressBook,

    /// The last warning or error event logged by the server.
    last_warn_error_log_rx: LoggedLastEvent,

    /// Handler for the `getblocktemplate` RPC.
    gbt: GetBlockTemplateHandler<BlockVerifierRouter, SyncStatus>,
}

/// A type alias for the last event logged by the server.
pub type LoggedLastEvent = watch::Receiver<Option<(String, tracing::Level, chrono::DateTime<Utc>)>>;

impl<Mempool, State, ReadState, Tip, AddressBook, BlockVerifierRouter, SyncStatus> fmt::Debug
    for RpcImpl<Mempool, State, ReadState, Tip, AddressBook, BlockVerifierRouter, SyncStatus>
where
    Mempool: Service<
            mempool::Request,
            Response = mempool::Response,
            Error = zebra_node_services::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    Mempool::Future: Send,
    State: Service<
            zebra_state::Request,
            Response = zebra_state::Response,
            Error = zebra_state::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    State::Future: Send,
    ReadState: Service<
            zebra_state::ReadRequest,
            Response = zebra_state::ReadResponse,
            Error = zebra_state::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    ReadState::Future: Send,
    Tip: ChainTip + Clone + Send + Sync + 'static,
    AddressBook: AddressBookPeers + Clone + Send + Sync + 'static,
    BlockVerifierRouter: Service<zebra_consensus::Request, Response = block::Hash, Error = zebra_consensus::BoxError>
        + Clone
        + Send
        + Sync
        + 'static,
    <BlockVerifierRouter as Service<zebra_consensus::Request>>::Future: Send,
    SyncStatus: ChainSyncStatus + Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Skip fields without Debug impls, and skip channels
        f.debug_struct("RpcImpl")
            .field("build_version", &self.build_version)
            .field("user_agent", &self.user_agent)
            .field("network", &self.network)
            .field("debug_force_finished_sync", &self.debug_force_finished_sync)
            .field("getblocktemplate", &self.gbt)
            .finish()
    }
}

impl<Mempool, State, ReadState, Tip, AddressBook, BlockVerifierRouter, SyncStatus>
    RpcImpl<Mempool, State, ReadState, Tip, AddressBook, BlockVerifierRouter, SyncStatus>
where
    Mempool: Service<
            mempool::Request,
            Response = mempool::Response,
            Error = zebra_node_services::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    Mempool::Future: Send,
    State: Service<
            zebra_state::Request,
            Response = zebra_state::Response,
            Error = zebra_state::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    State::Future: Send,
    ReadState: Service<
            zebra_state::ReadRequest,
            Response = zebra_state::ReadResponse,
            Error = zebra_state::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    ReadState::Future: Send,
    Tip: ChainTip + Clone + Send + Sync + 'static,
    AddressBook: AddressBookPeers + Clone + Send + Sync + 'static,
    BlockVerifierRouter: Service<zebra_consensus::Request, Response = block::Hash, Error = zebra_consensus::BoxError>
        + Clone
        + Send
        + Sync
        + 'static,
    <BlockVerifierRouter as Service<zebra_consensus::Request>>::Future: Send,
    SyncStatus: ChainSyncStatus + Clone + Send + Sync + 'static,
{
    /// Create a new instance of the RPC handler.
    //
    // TODO:
    // - put some of the configs or services in their own struct?
    #[allow(clippy::too_many_arguments)]
    pub fn new<VersionString, UserAgentString>(
        network: Network,
        mining_config: config::mining::Config,
        debug_force_finished_sync: bool,
        build_version: VersionString,
        user_agent: UserAgentString,
        mempool: Mempool,
        state: State,
        read_state: ReadState,
        block_verifier_router: BlockVerifierRouter,
        sync_status: SyncStatus,
        latest_chain_tip: Tip,
        address_book: AddressBook,
        last_warn_error_log_rx: LoggedLastEvent,
        mined_block_sender: Option<watch::Sender<(block::Hash, block::Height)>>,
    ) -> (Self, JoinHandle<()>)
    where
        VersionString: ToString + Clone + Send + 'static,
        UserAgentString: ToString + Clone + Send + 'static,
    {
        let (runner, queue_sender) = Queue::start();

        let mut build_version = build_version.to_string();
        let user_agent = user_agent.to_string();

        // Match zcashd's version format, if the version string has anything in it
        if !build_version.is_empty() && !build_version.starts_with('v') {
            build_version.insert(0, 'v');
        }

        let gbt = GetBlockTemplateHandler::new(
            &network,
            mining_config.clone(),
            block_verifier_router,
            sync_status,
            mined_block_sender,
        );

        let rpc_impl = RpcImpl {
            build_version,
            user_agent,
            network: network.clone(),
            debug_force_finished_sync,
            mempool: mempool.clone(),
            state: state.clone(),
            read_state: read_state.clone(),
            latest_chain_tip: latest_chain_tip.clone(),
            queue_sender,
            address_book,
            last_warn_error_log_rx,
            gbt,
        };

        // run the process queue
        let rpc_tx_queue_task_handle = tokio::spawn(
            runner
                .run(mempool, read_state, latest_chain_tip, network)
                .in_current_span(),
        );

        (rpc_impl, rpc_tx_queue_task_handle)
    }

    /// Returns a reference to the configured network.
    pub fn network(&self) -> &Network {
        &self.network
    }
}

#[async_trait]
impl<Mempool, State, ReadState, Tip, AddressBook, BlockVerifierRouter, SyncStatus> RpcServer
    for RpcImpl<Mempool, State, ReadState, Tip, AddressBook, BlockVerifierRouter, SyncStatus>
where
    Mempool: Service<
            mempool::Request,
            Response = mempool::Response,
            Error = zebra_node_services::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    Mempool::Future: Send,
    State: Service<
            zebra_state::Request,
            Response = zebra_state::Response,
            Error = zebra_state::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    State::Future: Send,
    ReadState: Service<
            zebra_state::ReadRequest,
            Response = zebra_state::ReadResponse,
            Error = zebra_state::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    ReadState::Future: Send,
    Tip: ChainTip + Clone + Send + Sync + 'static,
    AddressBook: AddressBookPeers + Clone + Send + Sync + 'static,
    BlockVerifierRouter: Service<zebra_consensus::Request, Response = block::Hash, Error = zebra_consensus::BoxError>
        + Clone
        + Send
        + Sync
        + 'static,
    <BlockVerifierRouter as Service<zebra_consensus::Request>>::Future: Send,
    SyncStatus: ChainSyncStatus + Clone + Send + Sync + 'static,
{
    async fn get_info(&self) -> Result<GetInfoResponse> {
        let version = GetInfoResponse::version_from_string(&self.build_version)
            .expect("invalid version string");

        let connections = self.address_book.recently_live_peers(Utc::now()).len();

        let last_error_recorded = self.last_warn_error_log_rx.borrow().clone();
        let (last_error_log, _level, last_error_log_time) = last_error_recorded.unwrap_or((
            GetInfoResponse::default().errors,
            tracing::Level::INFO,
            Utc::now(),
        ));

        let tip_height = self
            .latest_chain_tip
            .best_tip_height()
            .unwrap_or(Height::MIN);
        let testnet = self.network.is_a_test_network();

        // This field is behind the `ENABLE_WALLET` feature flag in zcashd:
        // https://github.com/zcash/zcash/blob/v6.1.0/src/rpc/misc.cpp#L113
        // However it is not documented as optional:
        // https://github.com/zcash/zcash/blob/v6.1.0/src/rpc/misc.cpp#L70
        // For compatibility, we keep the field in the response, but always return 0.
        let pay_tx_fee = 0.0;

        let relay_fee = zebra_chain::transaction::zip317::MIN_MEMPOOL_TX_FEE_RATE as f64
            / (zebra_chain::amount::COIN as f64);
        let difficulty = chain_tip_difficulty(self.network.clone(), self.read_state.clone(), true)
            .await
            .expect("should always be Ok when `should_use_default` is true");

        let response = GetInfoResponse {
            version,
            build: self.build_version.clone(),
            subversion: self.user_agent.clone(),
            protocol_version: zebra_network::constants::CURRENT_NETWORK_PROTOCOL_VERSION.0,
            blocks: tip_height.0,
            connections,
            proxy: None,
            difficulty,
            testnet,
            pay_tx_fee,
            relay_fee,
            errors: last_error_log,
            errors_timestamp: last_error_log_time.to_string(),
        };

        Ok(response)
    }

    #[allow(clippy::unwrap_in_result)]
    async fn get_blockchain_info(&self) -> Result<GetBlockchainInfoResponse> {
        let debug_force_finished_sync = self.debug_force_finished_sync;
        let network = &self.network;

        let (usage_info_rsp, tip_pool_values_rsp, chain_tip_difficulty) = {
            use zebra_state::ReadRequest::*;
            let state_call = |request| self.read_state.clone().oneshot(request);
            tokio::join!(
                state_call(UsageInfo),
                state_call(TipPoolValues),
                chain_tip_difficulty(network.clone(), self.read_state.clone(), true)
            )
        };

        let (size_on_disk, (tip_height, tip_hash), value_balance, difficulty) = {
            use zebra_state::ReadResponse::*;

            let UsageInfo(size_on_disk) = usage_info_rsp.map_misc_error()? else {
                unreachable!("unmatched response to a TipPoolValues request")
            };

            let (tip, value_balance) = match tip_pool_values_rsp {
                Ok(TipPoolValues {
                    tip_height,
                    tip_hash,
                    value_balance,
                }) => ((tip_height, tip_hash), value_balance),
                Ok(_) => unreachable!("unmatched response to a TipPoolValues request"),
                Err(_) => ((Height::MIN, network.genesis_hash()), Default::default()),
            };

            let difficulty = chain_tip_difficulty
                .expect("should always be Ok when `should_use_default` is true");

            (size_on_disk, tip, value_balance, difficulty)
        };

        let now = Utc::now();
        let (estimated_height, verification_progress) = self
            .latest_chain_tip
            .best_tip_height_and_block_time()
            .map(|(tip_height, tip_block_time)| {
                let height =
                    NetworkChainTipHeightEstimator::new(tip_block_time, tip_height, network)
                        .estimate_height_at(now);

                // If we're testing the mempool, force the estimated height to be the actual tip height, otherwise,
                // check if the estimated height is below Zebra's latest tip height, or if the latest tip's block time is
                // later than the current time on the local clock.
                let height =
                    if tip_block_time > now || height < tip_height || debug_force_finished_sync {
                        tip_height
                    } else {
                        height
                    };

                (height, f64::from(tip_height.0) / f64::from(height.0))
            })
            // TODO: Add a `genesis_block_time()` method on `Network` to use here.
            .unwrap_or((Height::MIN, 0.0));

        // `upgrades` object
        //
        // Get the network upgrades in height order, like `zcashd`.
        let mut upgrades = IndexMap::new();
        for (activation_height, network_upgrade) in network.full_activation_list() {
            // Zebra defines network upgrades based on incompatible consensus rule changes,
            // but zcashd defines them based on ZIPs.
            //
            // All the network upgrades with a consensus branch ID are the same in Zebra and zcashd.
            if let Some(branch_id) = network_upgrade.branch_id() {
                // zcashd's RPC seems to ignore Disabled network upgrades, so Zebra does too.
                let status = if tip_height >= activation_height {
                    NetworkUpgradeStatus::Active
                } else {
                    NetworkUpgradeStatus::Pending
                };

                let upgrade = NetworkUpgradeInfo {
                    name: network_upgrade,
                    activation_height,
                    status,
                };
                upgrades.insert(ConsensusBranchIdHex(branch_id), upgrade);
            }
        }

        // `consensus` object
        let next_block_height =
            (tip_height + 1).expect("valid chain tips are a lot less than Height::MAX");
        let consensus = TipConsensusBranch {
            chain_tip: ConsensusBranchIdHex(
                NetworkUpgrade::current(network, tip_height)
                    .branch_id()
                    .unwrap_or(ConsensusBranchId::RPC_MISSING_ID),
            ),
            next_block: ConsensusBranchIdHex(
                NetworkUpgrade::current(network, next_block_height)
                    .branch_id()
                    .unwrap_or(ConsensusBranchId::RPC_MISSING_ID),
            ),
        };

        let response = GetBlockchainInfoResponse {
            chain: network.bip70_network_name(),
            blocks: tip_height,
            best_block_hash: tip_hash,
            estimated_height,
            chain_supply: GetBlockchainInfoBalance::chain_supply(value_balance),
            value_pools: GetBlockchainInfoBalance::value_pools(value_balance, None),
            upgrades,
            consensus,
            headers: tip_height,
            difficulty,
            verification_progress,
            // TODO: store work in the finalized state for each height (#7109)
            chain_work: 0,
            pruned: false,
            size_on_disk,
            // TODO: Investigate whether this needs to be implemented (it's sprout-only in zcashd)
            commitments: 0,
        };

        Ok(response)
    }

    async fn get_address_balance(
        &self,
        address_strings: GetAddressBalanceRequest,
    ) -> Result<GetAddressBalanceResponse> {
        let valid_addresses = address_strings.valid_addresses()?;

        let request = zebra_state::ReadRequest::AddressBalance(valid_addresses);
        let response = self
            .read_state
            .clone()
            .oneshot(request)
            .await
            .map_misc_error()?;

        match response {
            zebra_state::ReadResponse::AddressBalance { balance, received } => {
                Ok(GetAddressBalanceResponse {
                    balance: u64::from(balance),
                    received,
                })
            }
            _ => unreachable!("Unexpected response from state service: {response:?}"),
        }
    }

    // TODO: use HexData or GetRawTransaction::Bytes to handle the transaction data argument
    async fn send_raw_transaction(
        &self,
        raw_transaction_hex: String,
        _allow_high_fees: Option<bool>,
    ) -> Result<SendRawTransactionResponse> {
        let mempool = self.mempool.clone();
        let queue_sender = self.queue_sender.clone();

        // Reference for the legacy error code:
        // <https://github.com/zcash/zcash/blob/99ad6fdc3a549ab510422820eea5e5ce9f60a5fd/src/rpc/rawtransaction.cpp#L1259-L1260>
        let raw_transaction_bytes = Vec::from_hex(raw_transaction_hex)
            .map_error(server::error::LegacyCode::Deserialization)?;
        let raw_transaction = Transaction::zcash_deserialize(&*raw_transaction_bytes)
            .map_error(server::error::LegacyCode::Deserialization)?;

        let transaction_hash = raw_transaction.hash();

        // send transaction to the rpc queue, ignore any error.
        let unmined_transaction = UnminedTx::from(raw_transaction.clone());
        let _ = queue_sender.send(unmined_transaction);

        let transaction_parameter = mempool::Gossip::Tx(raw_transaction.into());
        let request = mempool::Request::Queue(vec![transaction_parameter]);

        let response = mempool.oneshot(request).await.map_misc_error()?;

        let mut queue_results = match response {
            mempool::Response::Queued(results) => results,
            _ => unreachable!("incorrect response variant from mempool service"),
        };

        assert_eq!(
            queue_results.len(),
            1,
            "mempool service returned more results than expected"
        );

        let queue_result = queue_results
            .pop()
            .expect("there should be exactly one item in Vec")
            .inspect_err(|err| tracing::debug!("sent transaction to mempool: {:?}", &err))
            .map_misc_error()?
            .await
            .map_misc_error()?;

        tracing::debug!("sent transaction to mempool: {:?}", &queue_result);

        queue_result
            .map(|_| SendRawTransactionResponse(transaction_hash))
            // Reference for the legacy error code:
            // <https://github.com/zcash/zcash/blob/99ad6fdc3a549ab510422820eea5e5ce9f60a5fd/src/rpc/rawtransaction.cpp#L1290-L1301>
            // Note that this error code might not exactly match the one returned by zcashd
            // since zcashd's error code selection logic is more granular. We'd need to
            // propagate the error coming from the verifier to be able to return more specific
            // error codes.
            .map_error(server::error::LegacyCode::Verify)
    }

    // # Performance
    //
    // `lightwalletd` calls this RPC with verosity 1 for its initial sync of 2 million blocks, the
    // performance of this RPC with verbosity 1 significantly affects `lightwalletd`s sync time.
    //
    // TODO:
    // - use `height_from_signed_int()` to handle negative heights
    //   (this might be better in the state request, because it needs the state height)
    async fn get_block(
        &self,
        hash_or_height: String,
        verbosity: Option<u8>,
    ) -> Result<GetBlockResponse> {
        let verbosity = verbosity.unwrap_or(1);
        let network = self.network.clone();
        let original_hash_or_height = hash_or_height.clone();

        // If verbosity requires a call to `get_block_header`, resolve it here
        let get_block_header_future = if matches!(verbosity, 1 | 2) {
            Some(self.get_block_header(original_hash_or_height.clone(), Some(true)))
        } else {
            None
        };

        let hash_or_height =
            HashOrHeight::new(&hash_or_height, self.latest_chain_tip.best_tip_height())
                // Reference for the legacy error code:
                // <https://github.com/zcash/zcash/blob/99ad6fdc3a549ab510422820eea5e5ce9f60a5fd/src/rpc/blockchain.cpp#L629>
                .map_error(server::error::LegacyCode::InvalidParameter)?;

        if verbosity == 0 {
            let request = zebra_state::ReadRequest::Block(hash_or_height);
            let response = self
                .read_state
                .clone()
                .oneshot(request)
                .await
                .map_misc_error()?;

            match response {
                zebra_state::ReadResponse::Block(Some(block)) => {
                    Ok(GetBlockResponse::Raw(block.into()))
                }
                zebra_state::ReadResponse::Block(None) => {
                    Err("Block not found").map_error(server::error::LegacyCode::InvalidParameter)
                }
                _ => unreachable!("unmatched response to a block request"),
            }
        } else if let Some(get_block_header_future) = get_block_header_future {
            let get_block_header_result: Result<GetBlockHeaderResponse> =
                get_block_header_future.await;

            let GetBlockHeaderResponse::Object(block_header) = get_block_header_result? else {
                panic!("must return Object")
            };

            let BlockHeaderObject {
                hash,
                confirmations,
                height,
                version,
                merkle_root,
                block_commitments,
                final_sapling_root,
                sapling_tree_size,
                time,
                nonce,
                solution,
                bits,
                difficulty,
                previous_block_hash,
                next_block_hash,
            } = *block_header;

            let transactions_request = match verbosity {
                1 => zebra_state::ReadRequest::TransactionIdsForBlock(hash_or_height),
                2 => zebra_state::ReadRequest::BlockAndSize(hash_or_height),
                _other => panic!("get_block_header_fut should be none"),
            };

            // # Concurrency
            //
            // We look up by block hash so the hash, transaction IDs, and confirmations
            // are consistent.
            let hash_or_height = hash.into();
            let requests = vec![
                // Get transaction IDs from the transaction index by block hash
                //
                // # Concurrency
                //
                // A block's transaction IDs are never modified, so all possible responses are
                // valid. Clients that query block heights must be able to handle chain forks,
                // including getting transaction IDs from any chain fork.
                transactions_request,
                // Orchard trees
                zebra_state::ReadRequest::OrchardTree(hash_or_height),
                // Block info
                zebra_state::ReadRequest::BlockInfo(previous_block_hash.into()),
                zebra_state::ReadRequest::BlockInfo(hash_or_height),
            ];

            let mut futs = FuturesOrdered::new();

            for request in requests {
                futs.push_back(self.read_state.clone().oneshot(request));
            }

            let tx_ids_response = futs.next().await.expect("`futs` should not be empty");
            let (tx, size): (Vec<_>, Option<usize>) = match tx_ids_response.map_misc_error()? {
                zebra_state::ReadResponse::TransactionIdsForBlock(tx_ids) => (
                    tx_ids
                        .ok_or_misc_error("block not found")?
                        .iter()
                        .map(|tx_id| GetBlockTransaction::Hash(*tx_id))
                        .collect(),
                    None,
                ),
                zebra_state::ReadResponse::BlockAndSize(block_and_size) => {
                    let (block, size) = block_and_size.ok_or_misc_error("Block not found")?;
                    let block_time = block.header.time;
                    let transactions =
                        block
                            .transactions
                            .iter()
                            .map(|tx| {
                                GetBlockTransaction::Object(Box::new(
                                    TransactionObject::from_transaction(
                                        tx.clone(),
                                        Some(height),
                                        Some(confirmations.try_into().expect(
                                            "should be less than max block height, i32::MAX",
                                        )),
                                        &network,
                                        Some(block_time),
                                        Some(hash),
                                        Some(true),
                                        tx.hash(),
                                    ),
                                ))
                            })
                            .collect();
                    (transactions, Some(size))
                }
                _ => unreachable!("unmatched response to a transaction_ids_for_block request"),
            };

            let orchard_tree_response = futs.next().await.expect("`futs` should not be empty");
            let zebra_state::ReadResponse::OrchardTree(orchard_tree) =
                orchard_tree_response.map_misc_error()?
            else {
                unreachable!("unmatched response to a OrchardTree request");
            };

            let nu5_activation = NetworkUpgrade::Nu5.activation_height(&network);

            // This could be `None` if there's a chain reorg between state queries.
            let orchard_tree = orchard_tree.ok_or_misc_error("missing Orchard tree")?;

            let final_orchard_root = match nu5_activation {
                Some(activation_height) if height >= activation_height => {
                    Some(orchard_tree.root().into())
                }
                _other => None,
            };

            let sapling = SaplingTrees {
                size: sapling_tree_size,
            };

            let orchard_tree_size = orchard_tree.count();
            let orchard = OrchardTrees {
                size: orchard_tree_size,
            };

            let trees = GetBlockTrees { sapling, orchard };

            let block_info_response = futs.next().await.expect("`futs` should not be empty");
            let zebra_state::ReadResponse::BlockInfo(prev_block_info) =
                block_info_response.map_misc_error()?
            else {
                unreachable!("unmatched response to a BlockInfo request");
            };
            let block_info_response = futs.next().await.expect("`futs` should not be empty");
            let zebra_state::ReadResponse::BlockInfo(block_info) =
                block_info_response.map_misc_error()?
            else {
                unreachable!("unmatched response to a BlockInfo request");
            };

            let delta = block_info.as_ref().and_then(|d| {
                let value_pools = d.value_pools().constrain::<NegativeAllowed>().ok()?;
                let prev_value_pools = prev_block_info
                    .map(|d| d.value_pools().constrain::<NegativeAllowed>())
                    .unwrap_or(Ok(ValueBalance::<NegativeAllowed>::zero()))
                    .ok()?;
                (value_pools - prev_value_pools).ok()
            });
            let size = size.or(block_info.as_ref().map(|d| d.size() as usize));

            Ok(GetBlockResponse::Object(Box::new(BlockObject {
                hash,
                confirmations,
                height: Some(height),
                version: Some(version),
                merkle_root: Some(merkle_root),
                time: Some(time),
                nonce: Some(nonce),
                solution: Some(solution),
                bits: Some(bits),
                difficulty: Some(difficulty),
                tx,
                trees,
                chain_supply: block_info
                    .as_ref()
                    .map(|d| GetBlockchainInfoBalance::chain_supply(*d.value_pools())),
                value_pools: block_info
                    .map(|d| GetBlockchainInfoBalance::value_pools(*d.value_pools(), delta)),
                size: size.map(|size| size as i64),
                block_commitments: Some(block_commitments),
                final_sapling_root: Some(final_sapling_root),
                final_orchard_root,
                previous_block_hash: Some(previous_block_hash),
                next_block_hash,
            })))
        } else {
            Err("invalid verbosity value").map_error(server::error::LegacyCode::InvalidParameter)
        }
    }

    async fn get_block_header(
        &self,
        hash_or_height: String,
        verbose: Option<bool>,
    ) -> Result<GetBlockHeaderResponse> {
        let verbose = verbose.unwrap_or(true);
        let network = self.network.clone();

        let hash_or_height =
            HashOrHeight::new(&hash_or_height, self.latest_chain_tip.best_tip_height())
                // Reference for the legacy error code:
                // <https://github.com/zcash/zcash/blob/99ad6fdc3a549ab510422820eea5e5ce9f60a5fd/src/rpc/blockchain.cpp#L629>
                .map_error(server::error::LegacyCode::InvalidParameter)?;
        let zebra_state::ReadResponse::BlockHeader {
            header,
            hash,
            height,
            next_block_hash,
        } = self
            .read_state
            .clone()
            .oneshot(zebra_state::ReadRequest::BlockHeader(hash_or_height))
            .await
            .map_err(|_| "block height not in best chain")
            .map_error(
                // ## Compatibility with `zcashd`.
                //
                // Since this function is reused by getblock(), we return the errors
                // expected by it (they differ whether a hash or a height was passed).
                if hash_or_height.hash().is_some() {
                    server::error::LegacyCode::InvalidAddressOrKey
                } else {
                    server::error::LegacyCode::InvalidParameter
                },
            )?
        else {
            panic!("unexpected response to BlockHeader request")
        };

        let response = if !verbose {
            GetBlockHeaderResponse::Raw(HexData(header.zcash_serialize_to_vec().map_misc_error()?))
        } else {
            let zebra_state::ReadResponse::SaplingTree(sapling_tree) = self
                .read_state
                .clone()
                .oneshot(zebra_state::ReadRequest::SaplingTree(hash_or_height))
                .await
                .map_misc_error()?
            else {
                panic!("unexpected response to SaplingTree request")
            };

            // This could be `None` if there's a chain reorg between state queries.
            let sapling_tree = sapling_tree.ok_or_misc_error("missing Sapling tree")?;

            let zebra_state::ReadResponse::Depth(depth) = self
                .read_state
                .clone()
                .oneshot(zebra_state::ReadRequest::Depth(hash))
                .await
                .map_misc_error()?
            else {
                panic!("unexpected response to SaplingTree request")
            };

            // From <https://zcash.github.io/rpc/getblock.html>
            // TODO: Deduplicate const definition, consider refactoring this to avoid duplicate logic
            const NOT_IN_BEST_CHAIN_CONFIRMATIONS: i64 = -1;

            // Confirmations are one more than the depth.
            // Depth is limited by height, so it will never overflow an i64.
            let confirmations = depth
                .map(|depth| i64::from(depth) + 1)
                .unwrap_or(NOT_IN_BEST_CHAIN_CONFIRMATIONS);

            let mut nonce = *header.nonce;
            nonce.reverse();

            let sapling_activation = NetworkUpgrade::Sapling.activation_height(&network);
            let sapling_tree_size = sapling_tree.count();
            let final_sapling_root: [u8; 32] =
                if sapling_activation.is_some() && height >= sapling_activation.unwrap() {
                    let mut root: [u8; 32] = sapling_tree.root().into();
                    root.reverse();
                    root
                } else {
                    [0; 32]
                };

            let difficulty = header.difficulty_threshold.relative_to_network(&network);

            let block_commitments = match header.commitment(&network, height).expect(
                "Unexpected failure while parsing the blockcommitments field in get_block_header",
            ) {
                Commitment::PreSaplingReserved(bytes) => bytes,
                Commitment::FinalSaplingRoot(_) => final_sapling_root,
                Commitment::ChainHistoryActivationReserved => [0; 32],
                Commitment::ChainHistoryRoot(root) => root.bytes_in_display_order(),
                Commitment::ChainHistoryBlockTxAuthCommitment(hash) => {
                    hash.bytes_in_display_order()
                }
            };

            let block_header = BlockHeaderObject {
                hash,
                confirmations,
                height,
                version: header.version,
                merkle_root: header.merkle_root,
                block_commitments,
                final_sapling_root,
                sapling_tree_size,
                time: header.time.timestamp(),
                nonce,
                solution: header.solution,
                bits: header.difficulty_threshold,
                difficulty,
                previous_block_hash: header.previous_block_hash,
                next_block_hash,
            };

            GetBlockHeaderResponse::Object(Box::new(block_header))
        };

        Ok(response)
    }

    fn get_best_block_hash(&self) -> Result<GetBlockHashResponse> {
        self.latest_chain_tip
            .best_tip_hash()
            .map(GetBlockHashResponse)
            .ok_or_misc_error("No blocks in state")
    }

    fn get_best_block_height_and_hash(&self) -> Result<GetBlockHeightAndHashResponse> {
        self.latest_chain_tip
            .best_tip_height_and_hash()
            .map(|(height, hash)| GetBlockHeightAndHashResponse { height, hash })
            .ok_or_misc_error("No blocks in state")
    }

    async fn get_raw_mempool(&self, verbose: Option<bool>) -> Result<GetRawMempoolResponse> {
        #[allow(unused)]
        let verbose = verbose.unwrap_or(false);

        use zebra_chain::block::MAX_BLOCK_BYTES;

        let mut mempool = self.mempool.clone();

        let request = if verbose {
            mempool::Request::FullTransactions
        } else {
            mempool::Request::TransactionIds
        };

        // `zcashd` doesn't check if it is synced to the tip here, so we don't either.
        let response = mempool
            .ready()
            .and_then(|service| service.call(request))
            .await
            .map_misc_error()?;

        match response {
            mempool::Response::FullTransactions {
                mut transactions,
                transaction_dependencies,
                last_seen_tip_hash: _,
            } => {
                if verbose {
                    let map = transactions
                        .iter()
                        .map(|unmined_tx| {
                            (
                                unmined_tx.transaction.id.mined_id().encode_hex(),
                                get_raw_mempool::MempoolObject::from_verified_unmined_tx(
                                    unmined_tx,
                                    &transactions,
                                    &transaction_dependencies,
                                ),
                            )
                        })
                        .collect::<HashMap<_, _>>();
                    Ok(GetRawMempoolResponse::Verbose(map))
                } else {
                    // Sort transactions in descending order by fee/size, using
                    // hash in serialized byte order as a tie-breaker. Note that
                    // this is only done in not verbose because in verbose mode
                    // a dictionary is returned, where order does not matter.
                    transactions.sort_by_cached_key(|tx| {
                        // zcashd uses modified fee here but Zebra doesn't currently
                        // support prioritizing transactions
                        cmp::Reverse((
                            i64::from(tx.miner_fee) as u128 * MAX_BLOCK_BYTES as u128
                                / tx.transaction.size as u128,
                            // transaction hashes are compared in their serialized byte-order.
                            tx.transaction.id.mined_id(),
                        ))
                    });
                    let tx_ids: Vec<String> = transactions
                        .iter()
                        .map(|unmined_tx| unmined_tx.transaction.id.mined_id().encode_hex())
                        .collect();

                    Ok(GetRawMempoolResponse::TxIds(tx_ids))
                }
            }

            mempool::Response::TransactionIds(unmined_transaction_ids) => {
                let mut tx_ids: Vec<String> = unmined_transaction_ids
                    .iter()
                    .map(|id| id.mined_id().encode_hex())
                    .collect();

                // Sort returned transaction IDs in numeric/string order.
                tx_ids.sort();

                Ok(GetRawMempoolResponse::TxIds(tx_ids))
            }

            _ => unreachable!("unmatched response to a transactionids request"),
        }
    }

    async fn get_raw_transaction(
        &self,
        txid: String,
        verbose: Option<u8>,
        block_hash: Option<String>,
    ) -> Result<GetRawTransactionResponse> {
        let mut mempool = self.mempool.clone();
        let verbose = verbose.unwrap_or(0) != 0;

        // Reference for the legacy error code:
        // <https://github.com/zcash/zcash/blob/99ad6fdc3a549ab510422820eea5e5ce9f60a5fd/src/rpc/rawtransaction.cpp#L544>
        let txid = transaction::Hash::from_hex(txid)
            .map_error(server::error::LegacyCode::InvalidAddressOrKey)?;

        // Check the mempool first.
        if block_hash.is_none() {
            match mempool
                .ready()
                .and_then(|service| {
                    service.call(mempool::Request::TransactionsByMinedId([txid].into()))
                })
                .await
                .map_misc_error()?
            {
                mempool::Response::Transactions(txns) => {
                    if let Some(tx) = txns.first() {
                        return Ok(if verbose {
                            GetRawTransactionResponse::Object(Box::new(
                                TransactionObject::from_transaction(
                                    tx.transaction.clone(),
                                    None,
                                    None,
                                    &self.network,
                                    None,
                                    None,
                                    Some(false),
                                    txid,
                                ),
                            ))
                        } else {
                            let hex = tx.transaction.clone().into();
                            GetRawTransactionResponse::Raw(hex)
                        });
                    }
                }

                _ => unreachable!("unmatched response to a `TransactionsByMinedId` request"),
            };
        }

        // TODO: this should work for blocks in side chains
        let txid = if let Some(block_hash) = block_hash {
            let block_hash = block::Hash::from_hex(block_hash)
                .map_error(server::error::LegacyCode::InvalidAddressOrKey)?;
            match self
                .read_state
                .clone()
                .oneshot(zebra_state::ReadRequest::TransactionIdsForBlock(
                    block_hash.into(),
                ))
                .await
                .map_misc_error()?
            {
                zebra_state::ReadResponse::TransactionIdsForBlock(tx_ids) => *tx_ids
                    .ok_or_error(
                        server::error::LegacyCode::InvalidAddressOrKey,
                        "block not found",
                    )?
                    .iter()
                    .find(|id| **id == txid)
                    .ok_or_error(
                        server::error::LegacyCode::InvalidAddressOrKey,
                        "txid not found",
                    )?,
                _ => unreachable!("unmatched response to a `TransactionsByMinedId` request"),
            }
        } else {
            txid
        };

        // If the tx wasn't in the mempool, check the state.
        match self
            .read_state
            .clone()
            .oneshot(zebra_state::ReadRequest::Transaction(txid))
            .await
            .map_misc_error()?
        {
            zebra_state::ReadResponse::Transaction(Some(tx)) => Ok(if verbose {
                let block_hash = match self
                    .read_state
                    .clone()
                    .oneshot(zebra_state::ReadRequest::BestChainBlockHash(tx.height))
                    .await
                    .map_misc_error()?
                {
                    zebra_state::ReadResponse::BlockHash(block_hash) => block_hash,
                    _ => unreachable!("unmatched response to a `TransactionsByMinedId` request"),
                };

                GetRawTransactionResponse::Object(Box::new(TransactionObject::from_transaction(
                    tx.tx.clone(),
                    Some(tx.height),
                    Some(tx.confirmations),
                    &self.network,
                    // TODO: Performance gain:
                    // https://github.com/ZcashFoundation/zebra/pull/9458#discussion_r2059352752
                    Some(tx.block_time),
                    block_hash,
                    Some(true),
                    txid,
                )))
            } else {
                let hex = tx.tx.into();
                GetRawTransactionResponse::Raw(hex)
            }),

            zebra_state::ReadResponse::Transaction(None) => {
                Err("No such mempool or main chain transaction")
                    .map_error(server::error::LegacyCode::InvalidAddressOrKey)
            }

            _ => unreachable!("unmatched response to a `Transaction` read request"),
        }
    }

    // TODO:
    // - use `height_from_signed_int()` to handle negative heights
    //   (this might be better in the state request, because it needs the state height)
    async fn z_get_treestate(&self, hash_or_height: String) -> Result<GetTreestateResponse> {
        let mut read_state = self.read_state.clone();
        let network = self.network.clone();

        let hash_or_height =
            HashOrHeight::new(&hash_or_height, self.latest_chain_tip.best_tip_height())
                // Reference for the legacy error code:
                // <https://github.com/zcash/zcash/blob/99ad6fdc3a549ab510422820eea5e5ce9f60a5fd/src/rpc/blockchain.cpp#L629>
                .map_error(server::error::LegacyCode::InvalidParameter)?;

        // Fetch the block referenced by [`hash_or_height`] from the state.
        //
        // # Concurrency
        //
        // For consistency, this lookup must be performed first, then all the other lookups must
        // be based on the hash.
        //
        // TODO: If this RPC is called a lot, just get the block header, rather than the whole block.
        let block = match read_state
            .ready()
            .and_then(|service| service.call(zebra_state::ReadRequest::Block(hash_or_height)))
            .await
            .map_misc_error()?
        {
            zebra_state::ReadResponse::Block(Some(block)) => block,
            zebra_state::ReadResponse::Block(None) => {
                // Reference for the legacy error code:
                // <https://github.com/zcash/zcash/blob/99ad6fdc3a549ab510422820eea5e5ce9f60a5fd/src/rpc/blockchain.cpp#L629>
                return Err("the requested block is not in the main chain")
                    .map_error(server::error::LegacyCode::InvalidParameter);
            }
            _ => unreachable!("unmatched response to a block request"),
        };

        let hash = hash_or_height
            .hash_or_else(|_| Some(block.hash()))
            .expect("block hash");

        let height = hash_or_height
            .height_or_else(|_| block.coinbase_height())
            .expect("verified blocks have a coinbase height");

        let time = u32::try_from(block.header.time.timestamp())
            .expect("Timestamps of valid blocks always fit into u32.");

        let sapling_nu = zcash_primitives::consensus::NetworkUpgrade::Sapling;
        let sapling = if network.is_nu_active(sapling_nu, height.into()) {
            match read_state
                .ready()
                .and_then(|service| {
                    service.call(zebra_state::ReadRequest::SaplingTree(hash.into()))
                })
                .await
                .map_misc_error()?
            {
                zebra_state::ReadResponse::SaplingTree(tree) => tree.map(|t| t.to_rpc_bytes()),
                _ => unreachable!("unmatched response to a Sapling tree request"),
            }
        } else {
            None
        };

        let orchard_nu = zcash_primitives::consensus::NetworkUpgrade::Nu5;
        let orchard = if network.is_nu_active(orchard_nu, height.into()) {
            match read_state
                .ready()
                .and_then(|service| {
                    service.call(zebra_state::ReadRequest::OrchardTree(hash.into()))
                })
                .await
                .map_misc_error()?
            {
                zebra_state::ReadResponse::OrchardTree(tree) => tree.map(|t| t.to_rpc_bytes()),
                _ => unreachable!("unmatched response to an Orchard tree request"),
            }
        } else {
            None
        };

        Ok(GetTreestateResponse::from_parts(
            hash, height, time, sapling, orchard,
        ))
    }

    async fn z_get_subtrees_by_index(
        &self,
        pool: String,
        start_index: NoteCommitmentSubtreeIndex,
        limit: Option<NoteCommitmentSubtreeIndex>,
    ) -> Result<GetSubtreesByIndexResponse> {
        let mut read_state = self.read_state.clone();

        const POOL_LIST: &[&str] = &["sapling", "orchard"];

        if pool == "sapling" {
            let request = zebra_state::ReadRequest::SaplingSubtrees { start_index, limit };
            let response = read_state
                .ready()
                .and_then(|service| service.call(request))
                .await
                .map_misc_error()?;

            let subtrees = match response {
                zebra_state::ReadResponse::SaplingSubtrees(subtrees) => subtrees,
                _ => unreachable!("unmatched response to a subtrees request"),
            };

            let subtrees = subtrees
                .values()
                .map(|subtree| SubtreeRpcData {
                    root: subtree.root.encode_hex(),
                    end_height: subtree.end_height,
                })
                .collect();

            Ok(GetSubtreesByIndexResponse {
                pool,
                start_index,
                subtrees,
            })
        } else if pool == "orchard" {
            let request = zebra_state::ReadRequest::OrchardSubtrees { start_index, limit };
            let response = read_state
                .ready()
                .and_then(|service| service.call(request))
                .await
                .map_misc_error()?;

            let subtrees = match response {
                zebra_state::ReadResponse::OrchardSubtrees(subtrees) => subtrees,
                _ => unreachable!("unmatched response to a subtrees request"),
            };

            let subtrees = subtrees
                .values()
                .map(|subtree| SubtreeRpcData {
                    root: subtree.root.encode_hex(),
                    end_height: subtree.end_height,
                })
                .collect();

            Ok(GetSubtreesByIndexResponse {
                pool,
                start_index,
                subtrees,
            })
        } else {
            Err(ErrorObject::owned(
                server::error::LegacyCode::Misc.into(),
                format!("invalid pool name, must be one of: {POOL_LIST:?}").as_str(),
                None::<()>,
            ))
        }
    }

    async fn get_address_tx_ids(&self, request: GetAddressTxIdsRequest) -> Result<Vec<String>> {
        let mut read_state = self.read_state.clone();
        let latest_chain_tip = self.latest_chain_tip.clone();

        let height_range = build_height_range(
            request.start,
            request.end,
            best_chain_tip_height(&latest_chain_tip)?,
        )?;

        let valid_addresses = AddressStrings {
            addresses: request.addresses,
        }
        .valid_addresses()?;

        let request = zebra_state::ReadRequest::TransactionIdsByAddresses {
            addresses: valid_addresses,
            height_range,
        };
        let response = read_state
            .ready()
            .and_then(|service| service.call(request))
            .await
            .map_misc_error()?;

        let hashes = match response {
            zebra_state::ReadResponse::AddressesTransactionIds(hashes) => {
                let mut last_tx_location = TransactionLocation::from_usize(Height(0), 0);

                hashes
                    .iter()
                    .map(|(tx_loc, tx_id)| {
                        // Check that the returned transactions are in chain order.
                        assert!(
                            *tx_loc > last_tx_location,
                            "Transactions were not in chain order:\n\
                                 {tx_loc:?} {tx_id:?} was after:\n\
                                 {last_tx_location:?}",
                        );

                        last_tx_location = *tx_loc;

                        tx_id.to_string()
                    })
                    .collect()
            }
            _ => unreachable!("unmatched response to a TransactionsByAddresses request"),
        };

        Ok(hashes)
    }

    async fn get_address_utxos(
        &self,
        address_strings: AddressStrings,
    ) -> Result<GetAddressUtxosResponse> {
        let mut read_state = self.read_state.clone();
        let mut response_utxos = vec![];

        let valid_addresses = address_strings.valid_addresses()?;

        // get utxos data for addresses
        let request = zebra_state::ReadRequest::UtxosByAddresses(valid_addresses);
        let response = read_state
            .ready()
            .and_then(|service| service.call(request))
            .await
            .map_misc_error()?;
        let utxos = match response {
            zebra_state::ReadResponse::AddressUtxos(utxos) => utxos,
            _ => unreachable!("unmatched response to a UtxosByAddresses request"),
        };

        let mut last_output_location = OutputLocation::from_usize(Height(0), 0, 0);

        for utxo_data in utxos.utxos() {
            let address = utxo_data.0;
            let txid = *utxo_data.1;
            let height = utxo_data.2.height();
            let output_index = utxo_data.2.output_index();
            let script = utxo_data.3.lock_script.clone();
            let satoshis = u64::from(utxo_data.3.value);

            let output_location = *utxo_data.2;
            // Check that the returned UTXOs are in chain order.
            assert!(
                output_location > last_output_location,
                "UTXOs were not in chain order:\n\
                     {output_location:?} {address:?} {txid:?} was after:\n\
                     {last_output_location:?}",
            );

            let entry = Utxo {
                address,
                txid,
                output_index,
                script,
                satoshis,
                height,
            };
            response_utxos.push(entry);

            last_output_location = output_location;
        }

        Ok(response_utxos)
    }

    fn stop(&self) -> Result<String> {
        #[cfg(not(target_os = "windows"))]
        if self.network.is_regtest() {
            match nix::sys::signal::raise(nix::sys::signal::SIGINT) {
                Ok(_) => Ok("Zebra server stopping".to_string()),
                Err(error) => Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    format!("Failed to shut down: {error}").as_str(),
                    None::<()>,
                )),
            }
        } else {
            Err(ErrorObject::borrowed(
                ErrorCode::MethodNotFound.code(),
                "stop is only available on regtest networks",
                None,
            ))
        }
        #[cfg(target_os = "windows")]
        Err(ErrorObject::borrowed(
            ErrorCode::MethodNotFound.code(),
            "stop is not available in windows targets",
            None,
        ))
    }

    fn get_block_count(&self) -> Result<u32> {
        best_chain_tip_height(&self.latest_chain_tip).map(|height| height.0)
    }

    async fn get_block_hash(&self, index: i32) -> Result<GetBlockHashResponse> {
        let mut read_state = self.read_state.clone();
        let latest_chain_tip = self.latest_chain_tip.clone();

        // TODO: look up this height as part of the state request?
        let tip_height = best_chain_tip_height(&latest_chain_tip)?;

        let height = height_from_signed_int(index, tip_height)?;

        let request = zebra_state::ReadRequest::BestChainBlockHash(height);
        let response = read_state
            .ready()
            .and_then(|service| service.call(request))
            .await
            .map_error(server::error::LegacyCode::default())?;

        match response {
            zebra_state::ReadResponse::BlockHash(Some(hash)) => Ok(GetBlockHashResponse(hash)),
            zebra_state::ReadResponse::BlockHash(None) => Err(ErrorObject::borrowed(
                server::error::LegacyCode::InvalidParameter.into(),
                "Block not found",
                None,
            )),
            _ => unreachable!("unmatched response to a block request"),
        }
    }

    async fn get_block_template(
        &self,
        parameters: Option<GetBlockTemplateParameters>,
    ) -> Result<GetBlockTemplateResponse> {
        use types::get_block_template::{
            check_parameters, check_synced_to_tip, fetch_mempool_transactions,
            fetch_state_tip_and_local_time, validate_block_proposal,
            zip317::select_mempool_transactions,
        };

        // Clone Configs
        let network = self.network.clone();
        let extra_coinbase_data = self.gbt.extra_coinbase_data();

        // Clone Services
        let mempool = self.mempool.clone();
        let mut latest_chain_tip = self.latest_chain_tip.clone();
        let sync_status = self.gbt.sync_status();
        let read_state = self.read_state.clone();

        if let Some(HexData(block_proposal_bytes)) = parameters
            .as_ref()
            .and_then(GetBlockTemplateParameters::block_proposal_data)
        {
            return validate_block_proposal(
                self.gbt.block_verifier_router(),
                block_proposal_bytes,
                network,
                latest_chain_tip,
                sync_status,
            )
            .await;
        }

        // To implement long polling correctly, we split this RPC into multiple phases.
        check_parameters(&parameters)?;

        let client_long_poll_id = parameters.as_ref().and_then(|params| params.long_poll_id);

        let miner_address = self
            .gbt
            .miner_address()
            .ok_or_misc_error("miner_address not configured")?;

        // - Checks and fetches that can change during long polling
        //
        // Set up the loop.
        let mut max_time_reached = false;

        // The loop returns the server long poll ID,
        // which should be different to the client long poll ID.
        let (
            server_long_poll_id,
            chain_tip_and_local_time,
            mempool_txs,
            mempool_tx_deps,
            submit_old,
        ) = loop {
            // Check if we are synced to the tip.
            // The result of this check can change during long polling.
            //
            // Optional TODO:
            // - add `async changed()` method to ChainSyncStatus (like `ChainTip`)
            check_synced_to_tip(&network, latest_chain_tip.clone(), sync_status.clone())?;
            // TODO: return an error if we have no peers, like `zcashd` does,
            //       and add a developer config that mines regardless of how many peers we have.
            // https://github.com/zcash/zcash/blob/6fdd9f1b81d3b228326c9826fa10696fc516444b/src/miner.cpp#L865-L880

            // We're just about to fetch state data, then maybe wait for any changes.
            // Mark all the changes before the fetch as seen.
            // Changes are also ignored in any clones made after the mark.
            latest_chain_tip.mark_best_tip_seen();

            // Fetch the state data and local time for the block template:
            // - if the tip block hash changes, we must return from long polling,
            // - if the local clock changes on testnet, we might return from long polling
            //
            // We always return after 90 minutes on mainnet, even if we have the same response,
            // because the max time has been reached.
            let chain_tip_and_local_time @ zebra_state::GetBlockTemplateChainInfo {
                tip_hash,
                tip_height,
                max_time,
                cur_time,
                ..
            } = fetch_state_tip_and_local_time(read_state.clone()).await?;

            // Fetch the mempool data for the block template:
            // - if the mempool transactions change, we might return from long polling.
            //
            // If the chain fork has just changed, miners want to get the new block as fast
            // as possible, rather than wait for transactions to re-verify. This increases
            // miner profits (and any delays can cause chain forks). So we don't wait between
            // the chain tip changing and getting mempool transactions.
            //
            // Optional TODO:
            // - add a `MempoolChange` type with an `async changed()` method (like `ChainTip`)
            let Some((mempool_txs, mempool_tx_deps)) =
                fetch_mempool_transactions(mempool.clone(), tip_hash)
                    .await?
                    // If the mempool and state responses are out of sync:
                    // - if we are not long polling, omit mempool transactions from the template,
                    // - if we are long polling, continue to the next iteration of the loop to make fresh state and mempool requests.
                    .or_else(|| client_long_poll_id.is_none().then(Default::default))
            else {
                continue;
            };

            // - Long poll ID calculation
            let server_long_poll_id = LongPollInput::new(
                tip_height,
                tip_hash,
                max_time,
                mempool_txs.iter().map(|tx| tx.transaction.id),
            )
            .generate_id();

            // The loop finishes if:
            // - the client didn't pass a long poll ID,
            // - the server long poll ID is different to the client long poll ID, or
            // - the previous loop iteration waited until the max time.
            if Some(&server_long_poll_id) != client_long_poll_id.as_ref() || max_time_reached {
                let mut submit_old = client_long_poll_id
                    .as_ref()
                    .map(|old_long_poll_id| server_long_poll_id.submit_old(old_long_poll_id));

                // On testnet, the max time changes the block difficulty, so old shares are
                // invalid. On mainnet, this means there has been 90 minutes without a new
                // block or mempool transaction, which is very unlikely. So the miner should
                // probably reset anyway.
                if max_time_reached {
                    submit_old = Some(false);
                }

                break (
                    server_long_poll_id,
                    chain_tip_and_local_time,
                    mempool_txs,
                    mempool_tx_deps,
                    submit_old,
                );
            }

            // - Polling wait conditions
            //
            // TODO: when we're happy with this code, split it into a function.
            //
            // Periodically check the mempool for changes.
            //
            // Optional TODO:
            // Remove this polling wait if we switch to using futures to detect sync status
            // and mempool changes.
            let wait_for_mempool_request =
                tokio::time::sleep(Duration::from_secs(MEMPOOL_LONG_POLL_INTERVAL));

            // Return immediately if the chain tip has changed.
            // The clone preserves the seen status of the chain tip.
            let mut wait_for_best_tip_change = latest_chain_tip.clone();
            let wait_for_best_tip_change = wait_for_best_tip_change.best_tip_changed();

            // Wait for the maximum block time to elapse. This can change the block header
            // on testnet. (On mainnet it can happen due to a network disconnection, or a
            // rapid drop in hash rate.)
            //
            // This duration might be slightly lower than the actual maximum,
            // if cur_time was clamped to min_time. In that case the wait is very long,
            // and it's ok to return early.
            //
            // It can also be zero if cur_time was clamped to max_time. In that case,
            // we want to wait for another change, and ignore this timeout. So we use an
            // `OptionFuture::None`.
            let duration_until_max_time = max_time.saturating_duration_since(cur_time);
            let wait_for_max_time: OptionFuture<_> = if duration_until_max_time.seconds() > 0 {
                Some(tokio::time::sleep(duration_until_max_time.to_std()))
            } else {
                None
            }
            .into();

            // Optional TODO:
            // `zcashd` generates the next coinbase transaction while waiting for changes.
            // When Zebra supports shielded coinbase, we might want to do this in parallel.
            // But the coinbase value depends on the selected transactions, so this needs
            // further analysis to check if it actually saves us any time.

            tokio::select! {
                // Poll the futures in the listed order, for efficiency.
                // We put the most frequent conditions first.
                biased;

                // This timer elapses every few seconds
                _elapsed = wait_for_mempool_request => {
                    tracing::debug!(
                        ?max_time,
                        ?cur_time,
                        ?server_long_poll_id,
                        ?client_long_poll_id,
                        MEMPOOL_LONG_POLL_INTERVAL,
                        "checking for a new mempool change after waiting a few seconds"
                    );
                }

                // The state changes after around a target block interval (75s)
                tip_changed_result = wait_for_best_tip_change => {
                    match tip_changed_result {
                        Ok(()) => {
                            // Spurious updates shouldn't happen in the state, because the
                            // difficulty and hash ordering is a stable total order. But
                            // since they could cause a busy-loop, guard against them here.
                            latest_chain_tip.mark_best_tip_seen();

                            let new_tip_hash = latest_chain_tip.best_tip_hash();
                            if new_tip_hash == Some(tip_hash) {
                                tracing::debug!(
                                    ?max_time,
                                    ?cur_time,
                                    ?server_long_poll_id,
                                    ?client_long_poll_id,
                                    ?tip_hash,
                                    ?tip_height,
                                    "ignoring spurious state change notification"
                                );

                                // Wait for the mempool interval, then check for any changes.
                                tokio::time::sleep(Duration::from_secs(
                                    MEMPOOL_LONG_POLL_INTERVAL,
                                )).await;

                                continue;
                            }

                            tracing::debug!(
                                ?max_time,
                                ?cur_time,
                                ?server_long_poll_id,
                                ?client_long_poll_id,
                                "returning from long poll because state has changed"
                            );
                        }

                        Err(recv_error) => {
                            // This log is rare and helps with debugging, so it's ok to be info.
                            tracing::info!(
                                ?recv_error,
                                ?max_time,
                                ?cur_time,
                                ?server_long_poll_id,
                                ?client_long_poll_id,
                                "returning from long poll due to a state error.\
                                Is Zebra shutting down?"
                            );

                            return Err(recv_error).map_error(server::error::LegacyCode::default());
                        }
                    }
                }

                // The max time does not elapse during normal operation on mainnet,
                // and it rarely elapses on testnet.
                Some(_elapsed) = wait_for_max_time => {
                    // This log is very rare so it's ok to be info.
                    tracing::info!(
                        ?max_time,
                        ?cur_time,
                        ?server_long_poll_id,
                        ?client_long_poll_id,
                        "returning from long poll because max time was reached"
                    );

                    max_time_reached = true;
                }
            }
        };

        // - Processing fetched data to create a transaction template
        //
        // Apart from random weighted transaction selection,
        // the template only depends on the previously fetched data.
        // This processing never fails.

        // Calculate the next block height.
        let next_block_height =
            (chain_tip_and_local_time.tip_height + 1).expect("tip is far below Height::MAX");

        tracing::debug!(
            mempool_tx_hashes = ?mempool_txs
                .iter()
                .map(|tx| tx.transaction.id.mined_id())
                .collect::<Vec<_>>(),
            "selecting transactions for the template from the mempool"
        );

        // Randomly select some mempool transactions.
        let mempool_txs = select_mempool_transactions(
            &network,
            next_block_height,
            &miner_address,
            mempool_txs,
            mempool_tx_deps,
            extra_coinbase_data.clone(),
        );

        tracing::debug!(
            selected_mempool_tx_hashes = ?mempool_txs
                .iter()
                .map(|#[cfg(not(test))] tx, #[cfg(test)] (_, tx)| tx.transaction.id.mined_id())
                .collect::<Vec<_>>(),
            "selected transactions for the template from the mempool"
        );

        // - After this point, the template only depends on the previously fetched data.

        let response = BlockTemplateResponse::new_internal(
            &network,
            &miner_address,
            &chain_tip_and_local_time,
            server_long_poll_id,
            mempool_txs,
            submit_old,
            extra_coinbase_data,
        );

        Ok(response.into())
    }

    async fn submit_block(
        &self,
        HexData(block_bytes): HexData,
        _parameters: Option<SubmitBlockParameters>,
    ) -> Result<SubmitBlockResponse> {
        let mut block_verifier_router = self.gbt.block_verifier_router();

        let block: Block = match block_bytes.zcash_deserialize_into() {
            Ok(block_bytes) => block_bytes,
            Err(error) => {
                tracing::info!(
                    ?error,
                    "submit block failed: block bytes could not be deserialized into a structurally valid block"
                );

                return Ok(SubmitBlockErrorResponse::Rejected.into());
            }
        };

        let height = block
            .coinbase_height()
            .ok_or_error(0, "coinbase height not found")?;
        let block_hash = block.hash();

        let block_verifier_router_response = block_verifier_router
            .ready()
            .await
            .map_err(|error| ErrorObject::owned(0, error.to_string(), None::<()>))?
            .call(zebra_consensus::Request::Commit(Arc::new(block)))
            .await;

        let chain_error = match block_verifier_router_response {
            // Currently, this match arm returns `null` (Accepted) for blocks committed
            // to any chain, but Accepted is only for blocks in the best chain.
            //
            // TODO (#5487):
            // - Inconclusive: check if the block is on a side-chain
            // The difference is important to miners, because they want to mine on the best chain.
            Ok(hash) => {
                tracing::info!(?hash, ?height, "submit block accepted");

                self.gbt
                    .advertise_mined_block(hash, height)
                    .map_error_with_prefix(0, "failed to send mined block")?;

                return Ok(SubmitBlockResponse::Accepted);
            }

            // Turns BoxError into Result<VerifyChainError, BoxError>,
            // by downcasting from Any to VerifyChainError.
            Err(box_error) => {
                let error = box_error
                    .downcast::<RouterError>()
                    .map(|boxed_chain_error| *boxed_chain_error);

                tracing::info!(
                    ?error,
                    ?block_hash,
                    ?height,
                    "submit block failed verification"
                );

                error
            }
        };

        let response = match chain_error {
            Ok(source) if source.is_duplicate_request() => SubmitBlockErrorResponse::Duplicate,

            // Currently, these match arms return Reject for the older duplicate in a queue,
            // but queued duplicates should be DuplicateInconclusive.
            //
            // Optional TODO (#5487):
            // - DuplicateInconclusive: turn these non-finalized state duplicate block errors
            //   into BlockError enum variants, and handle them as DuplicateInconclusive:
            //   - "block already sent to be committed to the state"
            //   - "replaced by newer request"
            // - keep the older request in the queue,
            //   and return a duplicate error for the newer request immediately.
            //   This improves the speed of the RPC response.
            //
            // Checking the download queues and BlockVerifierRouter buffer for duplicates
            // might require architectural changes to Zebra, so we should only do it
            // if mining pools really need it.
            Ok(_verify_chain_error) => SubmitBlockErrorResponse::Rejected,

            // This match arm is currently unreachable, but if future changes add extra error types,
            // we want to turn them into `Rejected`.
            Err(_unknown_error_type) => SubmitBlockErrorResponse::Rejected,
        };

        Ok(response.into())
    }

    async fn get_mining_info(&self) -> Result<GetMiningInfoResponse> {
        let network = self.network.clone();
        let mut read_state = self.read_state.clone();

        let chain_tip = self.latest_chain_tip.clone();
        let tip_height = chain_tip.best_tip_height().unwrap_or(Height(0)).0;

        let mut current_block_tx = None;
        if tip_height > 0 {
            let mined_tx_ids = chain_tip.best_tip_mined_transaction_ids();
            current_block_tx =
                (!mined_tx_ids.is_empty()).then(|| mined_tx_ids.len().saturating_sub(1));
        }

        let solution_rate_fut = self.get_network_sol_ps(None, None);
        // Get the current block size.
        let mut current_block_size = None;
        if tip_height > 0 {
            let request = zebra_state::ReadRequest::TipBlockSize;
            let response: zebra_state::ReadResponse = read_state
                .ready()
                .and_then(|service| service.call(request))
                .await
                .map_error(server::error::LegacyCode::default())?;
            current_block_size = match response {
                zebra_state::ReadResponse::TipBlockSize(Some(block_size)) => Some(block_size),
                _ => None,
            };
        }

        Ok(GetMiningInfoResponse::new_internal(
            tip_height,
            current_block_size,
            current_block_tx,
            network,
            solution_rate_fut.await?,
        ))
    }

    async fn get_network_sol_ps(
        &self,
        num_blocks: Option<i32>,
        height: Option<i32>,
    ) -> Result<u64> {
        // Default number of blocks is 120 if not supplied.
        let mut num_blocks = num_blocks.unwrap_or(DEFAULT_SOLUTION_RATE_WINDOW_SIZE);
        // But if it is 0 or negative, it uses the proof of work averaging window.
        if num_blocks < 1 {
            num_blocks = i32::try_from(POW_AVERAGING_WINDOW).expect("fits in i32");
        }
        let num_blocks =
            usize::try_from(num_blocks).expect("just checked for negatives, i32 fits in usize");

        // Default height is the tip height if not supplied. Negative values also mean the tip
        // height. Since negative values aren't valid heights, we can just use the conversion.
        let height = height.and_then(|height| height.try_into_height().ok());

        let mut read_state = self.read_state.clone();

        let request = ReadRequest::SolutionRate { num_blocks, height };

        let response = read_state
            .ready()
            .and_then(|service| service.call(request))
            .await
            .map_err(|error| ErrorObject::owned(0, error.to_string(), None::<()>))?;

        let solution_rate = match response {
            // zcashd returns a 0 rate when the calculation is invalid
            ReadResponse::SolutionRate(solution_rate) => solution_rate.unwrap_or(0),

            _ => unreachable!("unmatched response to a solution rate request"),
        };

        Ok(solution_rate
            .try_into()
            .expect("per-second solution rate always fits in u64"))
    }

    async fn get_peer_info(&self) -> Result<Vec<PeerInfo>> {
        let address_book = self.address_book.clone();
        Ok(address_book
            .recently_live_peers(chrono::Utc::now())
            .into_iter()
            .map(PeerInfo::from)
            .collect())
    }

    async fn validate_address(&self, raw_address: String) -> Result<ValidateAddressResponse> {
        let network = self.network.clone();

        validate_address(network, raw_address)
    }

    async fn z_validate_address(&self, raw_address: String) -> Result<ZValidateAddressResponse> {
        let network = self.network.clone();

        let Ok(address) = raw_address.parse::<zcash_address::ZcashAddress>() else {
            return Ok(ZValidateAddressResponse::invalid());
        };

        let address = match address.convert::<primitives::Address>() {
            Ok(address) => address,
            Err(err) => {
                tracing::debug!(?err, "conversion error");
                return Ok(ZValidateAddressResponse::invalid());
            }
        };

        if address.network() == network.kind() {
            Ok(ZValidateAddressResponse {
                is_valid: true,
                address: Some(raw_address),
                address_type: Some(ZValidateAddressType::from(&address)),
                is_mine: Some(false),
            })
        } else {
            tracing::info!(
                ?network,
                address_network = ?address.network(),
                "invalid address network in z_validateaddress RPC: address is for {:?} but Zebra is on {:?}",
                address.network(),
                network
            );

            Ok(ZValidateAddressResponse::invalid())
        }
    }

    async fn get_block_subsidy(&self, height: Option<u32>) -> Result<GetBlockSubsidyResponse> {
        let latest_chain_tip = self.latest_chain_tip.clone();
        let network = self.network.clone();

        let height = if let Some(height) = height {
            Height(height)
        } else {
            best_chain_tip_height(&latest_chain_tip)?
        };

        if height < network.height_for_first_halving() {
            return Err(ErrorObject::borrowed(
                0,
                "Zebra does not support founders' reward subsidies, \
                        use a block height that is after the first halving",
                None,
            ));
        }

        // Always zero for post-halving blocks
        let founders = Amount::zero();

        let total_block_subsidy =
            block_subsidy(height, &network).map_error(server::error::LegacyCode::default())?;
        let miner_subsidy = miner_subsidy(height, &network, total_block_subsidy)
            .map_error(server::error::LegacyCode::default())?;

        let (lockbox_streams, mut funding_streams): (Vec<_>, Vec<_>) =
            funding_stream_values(height, &network, total_block_subsidy)
                .map_error(server::error::LegacyCode::default())?
                .into_iter()
                // Separate the funding streams into deferred and non-deferred streams
                .partition(|(receiver, _)| matches!(receiver, FundingStreamReceiver::Deferred));

        let is_nu6 = NetworkUpgrade::current(&network, height) == NetworkUpgrade::Nu6;

        let [lockbox_total, funding_streams_total]: [std::result::Result<
            Amount<NonNegative>,
            amount::Error,
        >; 2] = [&lockbox_streams, &funding_streams]
            .map(|streams| streams.iter().map(|&(_, amount)| amount).sum());

        // Use the same funding stream order as zcashd
        funding_streams.sort_by_key(|(receiver, _funding_stream)| {
            ZCASHD_FUNDING_STREAM_ORDER
                .iter()
                .position(|zcashd_receiver| zcashd_receiver == receiver)
        });

        // Format the funding streams and lockbox streams
        let [funding_streams, lockbox_streams]: [Vec<_>; 2] = [funding_streams, lockbox_streams]
            .map(|streams| {
                streams
                    .into_iter()
                    .map(|(receiver, value)| {
                        let address = funding_stream_address(height, &network, receiver);
                        types::subsidy::FundingStream::new_internal(
                            is_nu6, receiver, value, address,
                        )
                    })
                    .collect()
            });

        Ok(GetBlockSubsidyResponse {
            miner: miner_subsidy.into(),
            founders: founders.into(),
            funding_streams,
            lockbox_streams,
            funding_streams_total: funding_streams_total
                .map_error(server::error::LegacyCode::default())?
                .into(),
            lockbox_total: lockbox_total
                .map_error(server::error::LegacyCode::default())?
                .into(),
            total_block_subsidy: total_block_subsidy.into(),
        })
    }

    async fn get_difficulty(&self) -> Result<f64> {
        chain_tip_difficulty(self.network.clone(), self.read_state.clone(), false).await
    }

    async fn z_list_unified_receivers(
        &self,
        address: String,
    ) -> Result<ZListUnifiedReceiversResponse> {
        use zcash_address::unified::Container;

        let (network, unified_address): (
            zcash_protocol::consensus::NetworkType,
            zcash_address::unified::Address,
        ) = zcash_address::unified::Encoding::decode(address.clone().as_str())
            .map_err(|error| ErrorObject::owned(0, error.to_string(), None::<()>))?;

        let mut p2pkh = None;
        let mut p2sh = None;
        let mut orchard = None;
        let mut sapling = None;

        for item in unified_address.items() {
            match item {
                zcash_address::unified::Receiver::Orchard(_data) => {
                    let addr = zcash_address::unified::Address::try_from_items(vec![item])
                        .expect("using data already decoded as valid");
                    orchard = Some(addr.encode(&network));
                }
                zcash_address::unified::Receiver::Sapling(data) => {
                    let addr = zebra_chain::primitives::Address::try_from_sapling(network, data)
                        .expect("using data already decoded as valid");
                    sapling = Some(addr.payment_address().unwrap_or_default());
                }
                zcash_address::unified::Receiver::P2pkh(data) => {
                    let addr =
                        zebra_chain::primitives::Address::try_from_transparent_p2pkh(network, data)
                            .expect("using data already decoded as valid");
                    p2pkh = Some(addr.payment_address().unwrap_or_default());
                }
                zcash_address::unified::Receiver::P2sh(data) => {
                    let addr =
                        zebra_chain::primitives::Address::try_from_transparent_p2sh(network, data)
                            .expect("using data already decoded as valid");
                    p2sh = Some(addr.payment_address().unwrap_or_default());
                }
                _ => (),
            }
        }

        Ok(ZListUnifiedReceiversResponse::new(
            orchard, sapling, p2pkh, p2sh,
        ))
    }

    async fn invalidate_block(&self, block_hash: block::Hash) -> Result<()> {
        self.state
            .clone()
            .oneshot(zebra_state::Request::InvalidateBlock(block_hash))
            .await
            .map(|rsp| assert_eq!(rsp, zebra_state::Response::Invalidated(block_hash)))
            .map_misc_error()
    }

    async fn reconsider_block(&self, block_hash: block::Hash) -> Result<Vec<block::Hash>> {
        self.state
            .clone()
            .oneshot(zebra_state::Request::ReconsiderBlock(block_hash))
            .await
            .map(|rsp| match rsp {
                zebra_state::Response::Reconsidered(block_hashes) => block_hashes,
                _ => unreachable!("unmatched response to a reconsider block request"),
            })
            .map_misc_error()
    }

    async fn generate(&self, num_blocks: u32) -> Result<Vec<Hash>> {
        let rpc = self.clone();
        let network = self.network.clone();

        if !network.disable_pow() {
            return Err(ErrorObject::borrowed(
                0,
                "generate is only supported on networks where PoW is disabled",
                None,
            ));
        }

        let mut block_hashes = Vec::new();
        for _ in 0..num_blocks {
            let block_template = rpc
                .get_block_template(None)
                .await
                .map_error(server::error::LegacyCode::default())?;

            let GetBlockTemplateResponse::TemplateMode(block_template) = block_template else {
                return Err(ErrorObject::borrowed(
                    0,
                    "error generating block template",
                    None,
                ));
            };

            let proposal_block = proposal_block_from_template(
                &block_template,
                BlockTemplateTimeSource::CurTime,
                &network,
            )
            .map_error(server::error::LegacyCode::default())?;

            let hex_proposal_block = HexData(
                proposal_block
                    .zcash_serialize_to_vec()
                    .map_error(server::error::LegacyCode::default())?,
            );

            rpc.submit_block(hex_proposal_block, None)
                .await
                .map_error(server::error::LegacyCode::default())?;

            block_hashes.push(GetBlockHashResponse(proposal_block.hash()));
        }

        Ok(block_hashes)
    }

    async fn add_node(
        &self,
        addr: zebra_network::PeerSocketAddr,
        command: AddNodeCommand,
    ) -> Result<()> {
        if self.network.is_regtest() {
            match command {
                AddNodeCommand::Add => {
                    tracing::info!(?addr, "adding peer address to the address book");
                    if self.address_book.clone().add_peer(addr) {
                        Ok(())
                    } else {
                        return Err(ErrorObject::owned(
                            ErrorCode::InvalidParams.code(),
                            format!("peer address was already present in the address book: {addr}"),
                            None::<()>,
                        ));
                    }
                }
            }
        } else {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "addnode command is only supported on regtest",
                None::<()>,
            ));
        }
    }

    // ==================== Botcash Social Protocol RPC Implementations ====================

    async fn z_social_post(
        &self,
        request: types::social::SocialPostRequest,
    ) -> Result<types::social::SocialPostResponse> {
        // Validate the request
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        if request.content.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "content is required",
                None::<()>,
            ));
        }

        // Check content length (max 500 bytes to leave room for header in 512-byte memo)
        if request.content.len() > 500 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!("content too long: {} bytes, max 500", request.content.len()),
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet functionality to:
        // 1. Look up the spending key for the 'from' address
        // 2. Create a shielded transaction with the social message in the memo
        // 3. Sign and broadcast the transaction
        //
        // For now, return an error indicating wallet support is not yet available.
        // This validates the RPC interface is correctly defined.
        Err(ErrorObject::owned(
            ErrorCode::MethodNotFound.code(),
            "z_socialpost requires wallet support which is not yet implemented in Zebra",
            None::<()>,
        ))
    }

    async fn z_social_dm(
        &self,
        request: types::social::SocialDmRequest,
    ) -> Result<types::social::SocialDmResponse> {
        // Validate the request
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        if request.to.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "to address is required",
                None::<()>,
            ));
        }

        if request.content.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "content is required",
                None::<()>,
            ));
        }

        // Check content length
        if request.content.len() > 500 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!("content too long: {} bytes, max 500", request.content.len()),
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet functionality
        Err(ErrorObject::owned(
            ErrorCode::MethodNotFound.code(),
            "z_socialdm requires wallet support which is not yet implemented in Zebra",
            None::<()>,
        ))
    }

    async fn z_social_follow(
        &self,
        request: types::social::SocialFollowRequest,
    ) -> Result<types::social::SocialFollowResponse> {
        // Validate the request
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        if request.target.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "target address is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet functionality
        Err(ErrorObject::owned(
            ErrorCode::MethodNotFound.code(),
            "z_socialfollow requires wallet support which is not yet implemented in Zebra",
            None::<()>,
        ))
    }

    async fn z_social_feed(
        &self,
        request: types::social::SocialFeedRequest,
    ) -> Result<types::social::SocialFeedResponse> {
        // Validate the request
        if request.ivks.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "at least one incoming viewing key (ivk) is required",
                None::<()>,
            ));
        }

        // Validate limit
        if request.limit == 0 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "limit must be greater than 0",
                None::<()>,
            ));
        }

        if request.limit > 1000 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "limit must not exceed 1000",
                None::<()>,
            ));
        }

        // Get the current chain tip height for the scanned range
        let tip_height = self
            .latest_chain_tip
            .best_tip_height()
            .unwrap_or(Height::MIN);

        let start_height = request.start_height.unwrap_or(0);
        let end_height = tip_height.0;

        // Note: Full implementation requires:
        // 1. Parsing the IVKs
        // 2. Scanning the blockchain for shielded outputs decryptable by those keys
        // 3. Parsing the memo fields for social messages
        // 4. Returning the decoded posts
        //
        // This is computationally expensive and typically done by an indexer service.
        // For now, return an empty feed with the scanned range information.
        Ok(types::social::SocialFeedResponse::new(
            vec![],
            0,
            types::social::ScannedRange::new(start_height, end_height),
        ))
    }

    // ==================== Botcash Attention Market RPC Implementations ====================

    async fn z_attention_boost(
        &self,
        request: types::social::AttentionBoostRequest,
    ) -> Result<types::social::AttentionBoostResponse> {
        // Validate the request
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        if request.target_txid.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "targetTxid is required",
                None::<()>,
            ));
        }

        if request.amount == 0 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "amount must be greater than 0",
                None::<()>,
            ));
        }

        // Minimum boost amount is 0.001 BCASH = 100,000 zatoshis
        const MIN_BOOST_AMOUNT: u64 = 100_000;
        if request.amount < MIN_BOOST_AMOUNT {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "amount must be at least {} zatoshis (0.001 BCASH)",
                    MIN_BOOST_AMOUNT
                ),
                None::<()>,
            ));
        }

        if request.duration_blocks == 0 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "durationBlocks must be greater than 0",
                None::<()>,
            ));
        }

        // Maximum duration is 30 days (~43,200 blocks at 60s)
        const MAX_DURATION_BLOCKS: u32 = 43_200;
        if request.duration_blocks > MAX_DURATION_BLOCKS {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "durationBlocks must not exceed {} (~30 days)",
                    MAX_DURATION_BLOCKS
                ),
                None::<()>,
            ));
        }

        // Category code must be valid (0-255)
        if let Some(category) = request.category {
            if category > 6 {
                // Categories 0-6 are defined, 7-255 reserved
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    "category must be 0-6 (7-255 are reserved)",
                    None::<()>,
                ));
            }
        }

        // Note: Full implementation requires wallet functionality
        Err(ErrorObject::owned(
            ErrorCode::MethodNotFound.code(),
            "z_attentionboost requires wallet support which is not yet implemented in Zebra",
            None::<()>,
        ))
    }

    async fn z_credit_tip(
        &self,
        request: types::social::CreditTipRequest,
    ) -> Result<types::social::CreditTipResponse> {
        // Validate the request
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        if request.target_txid.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "targetTxid is required",
                None::<()>,
            ));
        }

        if request.credit_amount == 0 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "creditAmount must be greater than 0",
                None::<()>,
            ));
        }

        // Validate message length if provided
        if let Some(ref message) = request.message {
            const MAX_TIP_MESSAGE_BYTES: usize = 456; // Max message bytes in CREDIT_TIP memo
            if message.len() > MAX_TIP_MESSAGE_BYTES {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!("message must not exceed {} bytes", MAX_TIP_MESSAGE_BYTES),
                    None::<()>,
                ));
            }
        }

        // Note: Full implementation requires wallet functionality and indexer support
        Err(ErrorObject::owned(
            ErrorCode::MethodNotFound.code(),
            "z_credittip requires wallet and indexer support which is not yet implemented in Zebra",
            None::<()>,
        ))
    }

    async fn z_credit_balance(
        &self,
        request: types::social::CreditBalanceRequest,
    ) -> Result<types::social::CreditBalanceResponse> {
        // Validate the request
        if request.address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "address is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to track credit balances.
        // Credits are computed from attention market payments and their expirations.
        //
        // For now, return a response indicating no credits (requires indexer).
        Ok(types::social::CreditBalanceResponse::new(0, 0, vec![]))
    }

    async fn z_market_feed(
        &self,
        request: types::social::MarketFeedRequest,
    ) -> Result<types::social::MarketFeedResponse> {
        // Validate feed type
        let valid_feed_types = ["hot", "top", "new", "boosted"];
        if !valid_feed_types.contains(&request.feed_type.as_str()) {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!("feedType must be one of: {}", valid_feed_types.join(", ")),
                None::<()>,
            ));
        }

        // Validate category if provided
        if let Some(category) = request.category {
            if category > 6 {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    "category must be 0-6 (7-255 are reserved)",
                    None::<()>,
                ));
            }
        }

        // Validate limit
        if request.limit == 0 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "limit must be greater than 0",
                None::<()>,
            ));
        }

        const MAX_LIMIT: u32 = 1000;
        if request.limit > MAX_LIMIT {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!("limit must not exceed {}", MAX_LIMIT),
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to track:
        // - Attention market content
        // - AU scores (paid + tips)
        // - Time-decay calculations
        // - Boost status
        //
        // For now, return an empty feed indicating indexer is required.
        Ok(types::social::MarketFeedResponse::new(
            vec![],
            0,
            request.feed_type,
        ))
    }

    async fn z_epoch_stats(
        &self,
        request: types::social::EpochStatsRequest,
    ) -> Result<types::social::EpochStatsResponse> {
        // Get current tip height to calculate current epoch
        let tip_height = self
            .latest_chain_tip
            .best_tip_height()
            .unwrap_or(Height::MIN);

        const EPOCH_LENGTH_BLOCKS: u32 = 1440; // ~1 day at 60s blocks
        let current_epoch = tip_height.0 / EPOCH_LENGTH_BLOCKS;

        let epoch_number = request.epoch_number.unwrap_or(current_epoch);

        // Validate epoch number
        if epoch_number > current_epoch {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "epochNumber {} is in the future (current epoch: {})",
                    epoch_number, current_epoch
                ),
                None::<()>,
            ));
        }

        // Calculate epoch block range
        let start_block = epoch_number * EPOCH_LENGTH_BLOCKS;
        let end_block = start_block + EPOCH_LENGTH_BLOCKS - 1;
        let is_complete = epoch_number < current_epoch;

        // Note: Full implementation requires an indexer to track:
        // - Total BCASH paid per epoch
        // - Number of unique payers
        // - Credits distributed
        //
        // For now, return epoch metadata with zero stats (requires indexer).
        Ok(types::social::EpochStatsResponse::new(
            epoch_number,
            start_block,
            end_block,
            0, // total_paid (requires indexer)
            0, // participants (requires indexer)
            0, // distributed (requires indexer)
            is_complete,
        ))
    }

    // ==================== Batch Queue RPC Method Implementations ====================

    async fn z_batch_queue(
        &self,
        request: types::social::BatchQueueRequest,
    ) -> Result<types::social::BatchQueueResponse> {
        // Validate the from address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate actions
        if request.actions.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "at least one action is required",
                None::<()>,
            ));
        }

        if request.actions.len() > types::social::MAX_BATCH_QUEUE_SIZE {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "too many actions: {} (max: {})",
                    request.actions.len(),
                    types::social::MAX_BATCH_QUEUE_SIZE
                ),
                None::<()>,
            ));
        }

        // Collect action types for response
        let action_types: Vec<String> = request
            .actions
            .iter()
            .map(|a| a.action_type().to_string())
            .collect();

        let queued_count = request.actions.len();

        // Note: Full implementation requires wallet support to:
        // - Maintain a queue of pending actions per address
        // - Convert BatchAction to SocialMessage
        // - Encode as BatchMessage when sending
        //
        // For now, return a stub response indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_batchqueue requires wallet support which is not yet implemented in Zebra. \
                Would queue {} actions: {:?}",
                queued_count, action_types
            ),
            None::<()>,
        ))
    }

    async fn z_batch_send(
        &self,
        request: types::social::BatchSendRequest,
    ) -> Result<types::social::BatchSendResponse> {
        // Validate the from address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet support to:
        // - Retrieve the pending action queue for this address
        // - Convert actions to SocialMessage instances
        // - Create a BatchMessage
        // - Sign and broadcast the transaction
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "z_batchsend requires wallet support which is not yet implemented in Zebra",
            None::<()>,
        ))
    }

    async fn z_batch_status(
        &self,
        request: types::social::BatchStatusRequest,
    ) -> Result<types::social::BatchStatusResponse> {
        // Validate the from address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet support to:
        // - Maintain a queue of pending actions per address
        // - Track estimated encoded size
        //
        // For now, return an empty queue status (no wallet = no queue).
        Ok(types::social::BatchStatusResponse::new(
            0, // queue_size
            types::social::MAX_BATCH_QUEUE_SIZE,
            vec![], // action_types
            0,      // estimated_size
        ))
    }

    async fn z_batch_clear(
        &self,
        request: types::social::BatchClearRequest,
    ) -> Result<types::social::BatchClearResponse> {
        // Validate the from address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet support to:
        // - Clear the pending action queue for this address
        //
        // For now, return success with 0 cleared (no wallet = no queue).
        Ok(types::social::BatchClearResponse::new(
            0,    // cleared
            true, // success
        ))
    }

    // ==================== Governance RPC Method Implementations ====================

    async fn z_governance_propose(
        &self,
        request: types::social::GovernanceProposalRequest,
    ) -> Result<types::social::GovernanceProposalResponse> {
        // Validate the proposer address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate title length (max 255 chars as per spec)
        const MAX_TITLE_LENGTH: usize = 255;
        if request.title.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "title is required",
                None::<()>,
            ));
        }
        if request.title.len() > MAX_TITLE_LENGTH {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "title exceeds maximum length of {} characters (got {})",
                    MAX_TITLE_LENGTH,
                    request.title.len()
                ),
                None::<()>,
            ));
        }

        // Validate description is not empty
        if request.description.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "description is required",
                None::<()>,
            ));
        }

        // Validate description length (must fit in memo field, ~500 bytes after encoding)
        const MAX_DESCRIPTION_LENGTH: usize = 65535; // u16 max for length field
        if request.description.len() > MAX_DESCRIPTION_LENGTH {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "description exceeds maximum length of {} characters",
                    MAX_DESCRIPTION_LENGTH
                ),
                None::<()>,
            ));
        }

        // Validate deposit amount (minimum 10 BCASH = 1,000,000,000 zatoshis)
        const MIN_PROPOSAL_DEPOSIT: u64 = 1_000_000_000; // 10 BCASH
        if request.deposit < MIN_PROPOSAL_DEPOSIT {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "deposit must be at least {} zatoshis (10 BCASH), got {}",
                    MIN_PROPOSAL_DEPOSIT, request.deposit
                ),
                None::<()>,
            ));
        }

        // Validate parameter changes for Parameter proposals
        if request.proposal_type == types::social::GovernanceProposalType::Parameter
            && request.parameters.is_empty()
        {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "parameter proposals require at least one parameter change",
                None::<()>,
            ));
        }

        // Validate each parameter change
        for param in &request.parameters {
            if param.param.is_empty() {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    "parameter name cannot be empty",
                    None::<()>,
                ));
            }
            if param.value.is_empty() {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!("parameter '{}' value cannot be empty", param.param),
                    None::<()>,
                ));
            }
        }

        // Note: Full implementation requires wallet support to:
        // - Create a GovernanceProposal (0xE1) memo message
        // - Send the deposit to a governance escrow address
        // - Sign and broadcast the transaction
        // - Calculate voting timeline based on current block height
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_governancepropose requires wallet support which is not yet implemented in Zebra. \
                Would create {:?} proposal '{}' with {} zatoshi deposit",
                request.proposal_type, request.title, request.deposit
            ),
            None::<()>,
        ))
    }

    async fn z_governance_vote(
        &self,
        request: types::social::GovernanceVoteRequest,
    ) -> Result<types::social::GovernanceVoteResponse> {
        // Validate the voter address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate proposal ID format (should be 64 hex chars = 32 bytes)
        if request.proposal_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "proposalId is required",
                None::<()>,
            ));
        }

        const PROPOSAL_ID_HEX_LENGTH: usize = 64; // 32 bytes as hex
        if request.proposal_id.len() != PROPOSAL_ID_HEX_LENGTH {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "proposalId must be {} hex characters (32 bytes), got {}",
                    PROPOSAL_ID_HEX_LENGTH,
                    request.proposal_id.len()
                ),
                None::<()>,
            ));
        }

        // Validate proposal ID is valid hex
        if !request
            .proposal_id
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "proposalId must contain only hexadecimal characters",
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet support to:
        // - Verify the proposal exists and is in voting phase
        // - Calculate voting power: sqrt(karma) + sqrt(bcash_balance)
        // - Create a GovernanceVote (0xE0) memo message
        // - Sign and broadcast the transaction
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_governancevote requires wallet support which is not yet implemented in Zebra. \
                Would cast {:?} vote on proposal {}",
                request.vote, request.proposal_id
            ),
            None::<()>,
        ))
    }

    async fn z_governance_status(
        &self,
        request: types::social::GovernanceProposalStatusRequest,
    ) -> Result<types::social::GovernanceProposalStatusResponse> {
        // Validate proposal ID format (should be 64 hex chars = 32 bytes)
        if request.proposal_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "proposalId is required",
                None::<()>,
            ));
        }

        const PROPOSAL_ID_HEX_LENGTH: usize = 64; // 32 bytes as hex
        if request.proposal_id.len() != PROPOSAL_ID_HEX_LENGTH {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "proposalId must be {} hex characters (32 bytes), got {}",
                    PROPOSAL_ID_HEX_LENGTH,
                    request.proposal_id.len()
                ),
                None::<()>,
            ));
        }

        // Validate proposal ID is valid hex
        if !request
            .proposal_id
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "proposalId must contain only hexadecimal characters",
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Look up the proposal by ID
        // - Aggregate all votes for the proposal
        // - Calculate quorum and approval percentages
        // - Determine current status based on voting period and thresholds
        //
        // For now, return an error indicating indexer support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_governancestatus requires indexer support which is not yet implemented. \
                Would query status for proposal {}",
                request.proposal_id
            ),
            None::<()>,
        ))
    }

    async fn z_governance_list(
        &self,
        request: types::social::GovernanceListRequest,
    ) -> Result<types::social::GovernanceListResponse> {
        // Validate status filter
        const VALID_STATUSES: [&str; 6] =
            ["all", "pending", "voting", "passed", "rejected", "executed"];
        if !VALID_STATUSES.contains(&request.status.as_str()) {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "invalid status filter '{}'. Valid values: {:?}",
                    request.status, VALID_STATUSES
                ),
                None::<()>,
            ));
        }

        // Validate limit (reasonable bounds)
        const MAX_LIST_LIMIT: u32 = 1000;
        if request.limit > MAX_LIST_LIMIT {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "limit exceeds maximum of {} (got {})",
                    MAX_LIST_LIMIT, request.limit
                ),
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Query all proposals from the chain
        // - Filter by status
        // - Apply pagination (offset/limit)
        // - Return proposal summaries with vote tallies
        //
        // For now, return an empty list (no indexer = no proposals visible).
        Ok(types::social::GovernanceListResponse::new(
            vec![], // proposals
            0,      // total_count
        ))
    }

    // ==================== Channel RPC Method Implementations ====================

    async fn z_channel_open(
        &self,
        request: types::social::ChannelOpenRequest,
    ) -> Result<types::social::ChannelOpenResponse> {
        // Validate the initiator address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate parties list
        if request.parties.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "at least one party is required",
                None::<()>,
            ));
        }

        if request.parties.len() > types::social::MAX_CHANNEL_PARTIES {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "too many parties: {} (maximum is {})",
                    request.parties.len(),
                    types::social::MAX_CHANNEL_PARTIES
                ),
                None::<()>,
            ));
        }

        // Check for empty party addresses
        for (i, party) in request.parties.iter().enumerate() {
            if party.is_empty() {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!("party {} address is empty", i),
                    None::<()>,
                ));
            }
        }

        // Validate deposit amount
        if request.deposit < types::social::MIN_CHANNEL_DEPOSIT {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "deposit {} is below minimum {} zatoshis",
                    request.deposit,
                    types::social::MIN_CHANNEL_DEPOSIT
                ),
                None::<()>,
            ));
        }

        // Validate timeout
        if request.timeout_blocks == 0 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "timeoutBlocks must be greater than 0",
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet support to:
        // - Create a ChannelOpen (0xC0) memo message
        // - Lock the deposit in a channel-specific address
        // - Sign and broadcast the transaction
        // - Generate a unique channel ID
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_channelopen requires wallet support which is not yet implemented in Zebra. \
                Would open channel with {} parties and {} zatoshi deposit",
                request.parties.len(),
                request.deposit
            ),
            None::<()>,
        ))
    }

    async fn z_channel_close(
        &self,
        request: types::social::ChannelCloseRequest,
    ) -> Result<types::social::ChannelCloseResponse> {
        // Validate the closer address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate channel ID (should be 64 hex chars = 32 bytes)
        if request.channel_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "channelId is required",
                None::<()>,
            ));
        }

        if request.channel_id.len() != 64 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "channelId must be 64 hex characters (32 bytes), got {}",
                    request.channel_id.len()
                ),
                None::<()>,
            ));
        }

        if !request
            .channel_id
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "channelId must contain only hexadecimal characters",
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet support to:
        // - Create a ChannelClose (0xC1) memo message
        // - Verify the from address is a party to the channel
        // - Get signatures from all parties for cooperative close
        // - Sign and broadcast the transaction
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_channelclose requires wallet support which is not yet implemented in Zebra. \
                Would close channel {} at sequence {}",
                request.channel_id, request.final_seq
            ),
            None::<()>,
        ))
    }

    async fn z_channel_settle(
        &self,
        request: types::social::ChannelSettleRequest,
    ) -> Result<types::social::ChannelSettleResponse> {
        // Validate the settler address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate channel ID (should be 64 hex chars = 32 bytes)
        if request.channel_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "channelId is required",
                None::<()>,
            ));
        }

        if request.channel_id.len() != 64 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "channelId must be 64 hex characters (32 bytes), got {}",
                    request.channel_id.len()
                ),
                None::<()>,
            ));
        }

        if !request
            .channel_id
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "channelId must contain only hexadecimal characters",
                None::<()>,
            ));
        }

        // Validate message hash (should be 64 hex chars = 32 bytes)
        if request.message_hash.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "messageHash is required",
                None::<()>,
            ));
        }

        if request.message_hash.len() != 64 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "messageHash must be 64 hex characters (32 bytes), got {}",
                    request.message_hash.len()
                ),
                None::<()>,
            ));
        }

        if !request
            .message_hash
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "messageHash must contain only hexadecimal characters",
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet support to:
        // - Create a ChannelSettle (0xC2) memo message
        // - Verify the from address is a party to the channel
        // - Verify the timeout has passed (for unilateral settlement)
        // - Calculate final balance distribution
        // - Sign and broadcast the transaction
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_channelsettle requires wallet support which is not yet implemented in Zebra. \
                Would settle channel {} at sequence {} with message hash {}",
                request.channel_id, request.final_seq, request.message_hash
            ),
            None::<()>,
        ))
    }

    async fn z_channel_status(
        &self,
        request: types::social::ChannelStatusRequest,
    ) -> Result<types::social::ChannelStatusResponse> {
        // Validate channel ID (should be 64 hex chars = 32 bytes)
        if request.channel_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "channelId is required",
                None::<()>,
            ));
        }

        if request.channel_id.len() != 64 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "channelId must be 64 hex characters (32 bytes), got {}",
                    request.channel_id.len()
                ),
                None::<()>,
            ));
        }

        if !request
            .channel_id
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "channelId must contain only hexadecimal characters",
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Look up the channel by ID
        // - Get the current state and parties
        // - Track off-chain message count
        // - Return channel status
        //
        // For now, return an error indicating indexer support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_channelstatus requires indexer support which is not yet implemented. \
                Would query status for channel {}",
                request.channel_id
            ),
            None::<()>,
        ))
    }

    async fn z_channel_list(
        &self,
        request: types::social::ChannelListRequest,
    ) -> Result<types::social::ChannelListResponse> {
        // Validate address
        if request.address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "address is required",
                None::<()>,
            ));
        }

        // Validate limit (reasonable bounds)
        const MAX_LIST_LIMIT: u32 = 1000;
        if request.limit > MAX_LIST_LIMIT {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "limit exceeds maximum of {} (got {})",
                    MAX_LIST_LIMIT, request.limit
                ),
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Query all channels for the address
        // - Filter by state if specified
        // - Apply pagination (limit)
        // - Return channel summaries
        //
        // For now, return an empty list (no indexer = no channels visible).
        Ok(types::social::ChannelListResponse::new(
            vec![], // channels
            0,      // total_count
        ))
    }

    // ==================== Recovery RPC Method Implementations ====================

    async fn z_recovery_config(
        &self,
        request: types::social::RecoveryConfigRequest,
    ) -> Result<types::social::RecoveryConfigResponse> {
        // Validate from address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate guardian count
        if request.guardians.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "at least one guardian is required",
                None::<()>,
            ));
        }

        if request.guardians.len() > types::social::MAX_RECOVERY_GUARDIANS {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "too many guardians (max: {}, got: {})",
                    types::social::MAX_RECOVERY_GUARDIANS,
                    request.guardians.len()
                ),
                None::<()>,
            ));
        }

        // Validate threshold
        if request.threshold == 0 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "threshold must be at least 1",
                None::<()>,
            ));
        }

        if usize::from(request.threshold) > request.guardians.len() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "threshold ({}) cannot exceed number of guardians ({})",
                    request.threshold,
                    request.guardians.len()
                ),
                None::<()>,
            ));
        }

        // Validate timelock is within reasonable bounds
        const MIN_TIMELOCK_BLOCKS: u32 = 1440; // ~1 day minimum
        const MAX_TIMELOCK_BLOCKS: u32 = 100800; // ~70 days maximum
        if request.timelock_blocks < MIN_TIMELOCK_BLOCKS {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "timelock too short (min: {} blocks, got: {})",
                    MIN_TIMELOCK_BLOCKS, request.timelock_blocks
                ),
                None::<()>,
            ));
        }
        if request.timelock_blocks > MAX_TIMELOCK_BLOCKS {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "timelock too long (max: {} blocks, got: {})",
                    MAX_TIMELOCK_BLOCKS, request.timelock_blocks
                ),
                None::<()>,
            ));
        }

        // Check for duplicate guardians
        let mut seen = std::collections::HashSet::new();
        for guardian in &request.guardians {
            if !seen.insert(guardian.clone()) {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!("duplicate guardian address: {}", guardian),
                    None::<()>,
                ));
            }
        }

        // Validate guardian addresses are not the same as owner
        for guardian in &request.guardians {
            if guardian == &request.from {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    "owner cannot be their own guardian",
                    None::<()>,
                ));
            }
        }

        // Note: Full implementation requires wallet support to:
        // - Generate Shamir shares for each guardian
        // - Create and sign the RecoveryConfig (0xF0) transaction
        // - Broadcast the transaction
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "z_recoveryconfig requires wallet support which is not yet implemented in Zebra. \
             The recovery configuration validation passed - transaction creation pending.",
            None::<()>,
        ))
    }

    async fn z_recovery_request(
        &self,
        request: types::social::RecoveryRequestRequest,
    ) -> Result<types::social::RecoveryRequestResponse> {
        // Validate from address (new device)
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate target address
        if request.target_address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "target address is required",
                None::<()>,
            ));
        }

        // Validate new pubkey format (should be 33 bytes hex = 66 chars)
        if request.new_pubkey.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "new pubkey is required",
                None::<()>,
            ));
        }

        if request.new_pubkey.len() != 66 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "new pubkey must be 33 bytes hex-encoded (66 chars), got {} chars",
                    request.new_pubkey.len()
                ),
                None::<()>,
            ));
        }

        // Validate hex format
        if !request.new_pubkey.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "new pubkey must be hex-encoded",
                None::<()>,
            ));
        }

        // Validate proof is provided
        if request.proof.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "proof is required",
                None::<()>,
            ));
        }

        // Validate from != target (can't recover own address from same address)
        if request.from == request.target_address {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address must be different from target address",
                None::<()>,
            ));
        }

        // Note: Full implementation requires:
        // - Wallet support for transaction creation
        // - Indexer lookup to verify target has recovery configured
        // - Verification that no pending request already exists
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "z_recoveryrequest requires wallet support which is not yet implemented in Zebra. \
             The recovery request validation passed - transaction creation pending.",
            None::<()>,
        ))
    }

    async fn z_recovery_approve(
        &self,
        request: types::social::RecoveryApproveRequest,
    ) -> Result<types::social::RecoveryApproveResponse> {
        // Validate guardian address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address (guardian) is required",
                None::<()>,
            ));
        }

        // Validate request ID
        if request.request_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "request ID is required",
                None::<()>,
            ));
        }

        // Validate encrypted share
        if request.encrypted_share.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "encrypted share is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires:
        // - Wallet support for transaction creation
        // - Indexer lookup to verify guardian is authorized
        // - Verification that request exists and is pending
        // - Verification that guardian hasn't already approved
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "z_recoveryapprove requires wallet support which is not yet implemented in Zebra. \
             The recovery approval validation passed - transaction creation pending.",
            None::<()>,
        ))
    }

    async fn z_recovery_cancel(
        &self,
        request: types::social::RecoveryCancelRequest,
    ) -> Result<types::social::RecoveryCancelResponse> {
        // Validate owner address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address (owner) is required",
                None::<()>,
            ));
        }

        // Validate request ID
        if request.request_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "request ID is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires:
        // - Wallet support for transaction creation
        // - Indexer lookup to verify from is the original owner
        // - Verification that request exists and is still pending/approved
        // - Verification that timelock hasn't expired
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "z_recoverycancel requires wallet support which is not yet implemented in Zebra. \
             The recovery cancel validation passed - transaction creation pending.",
            None::<()>,
        ))
    }

    async fn z_recovery_status(
        &self,
        request: types::social::RecoveryStatusRequest,
    ) -> Result<types::social::RecoveryStatusResponse> {
        // Validate address
        if request.address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "address is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Look up recovery configuration for the address
        // - Check for any pending recovery requests
        // - Return current status and guardian info
        //
        // For now, return a response indicating no recovery configured.
        Ok(types::social::RecoveryStatusResponse::new(
            request.address,                       // address
            false,                                 // has_recovery
            None,                                  // recovery_id
            None,                                  // guardian_count
            None,                                  // threshold
            None,                                  // timelock_blocks
            types::social::RecoveryStatus::Active, // status (Active = no pending recovery)
            None,                                  // pending_request
        ))
    }

    async fn z_recovery_guardians(
        &self,
        request: types::social::GuardianListRequest,
    ) -> Result<types::social::GuardianListResponse> {
        // Validate address
        if request.address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "address is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Look up recovery configuration for the address
        // - Return guardian addresses and their status
        //
        // For now, return an empty guardian list.
        Ok(types::social::GuardianListResponse::new(
            request.address,    // address
            vec![],             // guardians (empty - no recovery configured)
            0,                  // threshold
            0,                  // timelock_blocks
        ))
    }

    // ==================== Multi-Sig RPC Method Implementations ====================

    async fn z_multisig_setup(
        &self,
        request: types::social::MultisigSetupRequest,
    ) -> Result<types::social::MultisigSetupResponse> {
        // Validate address
        if request.address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "address is required",
                None::<()>,
            ));
        }

        // Validate multi-sig parameters
        if let Err(e) = request.validate() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                e,
                None::<()>,
            ));
        }

        // Check for duplicate keys
        let mut seen_keys = std::collections::HashSet::new();
        for key in &request.public_keys {
            if !seen_keys.insert(key.to_lowercase()) {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    "duplicate public keys are not allowed",
                    None::<()>,
                ));
            }
        }

        // Note: Full implementation requires wallet support to:
        // 1. Create the multi-sig setup transaction (0xF5 message)
        // 2. Sign and broadcast the transaction
        // 3. Return the transaction details
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "multi-sig setup requires wallet support (not yet implemented)",
            Some(serde_json::json!({
                "address": request.address,
                "keyCount": request.public_keys.len(),
                "threshold": request.threshold,
            })),
        ))
    }

    async fn z_multisig_action(
        &self,
        request: types::social::MultisigActionRequest,
    ) -> Result<types::social::MultisigActionResponse> {
        // Validate multi-sig address
        if request.multisig_address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "multisigAddress is required",
                None::<()>,
            ));
        }

        // Validate action type
        let valid_actions = ["post", "comment", "follow", "unfollow", "dm", "tip", "upvote"];
        if !valid_actions.contains(&request.action_type.to_lowercase().as_str()) {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!("invalid action type: {}. Valid types: {:?}", request.action_type, valid_actions),
                None::<()>,
            ));
        }

        // Validate signatures
        if request.signatures.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "at least one signature is required",
                None::<()>,
            ));
        }

        for (i, sig) in request.signatures.iter().enumerate() {
            if let Err(e) = sig.validate() {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!("invalid signature at index {}: {}", i, e),
                    None::<()>,
                ));
            }
        }

        // Check for duplicate key indices in signatures
        let mut seen_indices = std::collections::HashSet::new();
        for sig in &request.signatures {
            if !seen_indices.insert(sig.key_index) {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!("duplicate signature from key index {}", sig.key_index),
                    None::<()>,
                ));
            }
        }

        // Note: Full implementation requires:
        // 1. Look up the multi-sig configuration for the address (indexer)
        // 2. Verify we have enough signatures (>= threshold)
        // 3. Verify each signature is valid for the action payload
        // 4. Create and broadcast the multi-sig action transaction (0xF6)
        //
        // For now, return an error indicating this requires indexer + wallet support.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            "multi-sig action requires indexer and wallet support (not yet implemented)",
            Some(serde_json::json!({
                "multisigAddress": request.multisig_address,
                "actionType": request.action_type,
                "signatureCount": request.signatures.len(),
            })),
        ))
    }

    async fn z_multisig_status(
        &self,
        request: types::social::MultisigStatusRequest,
    ) -> Result<types::social::MultisigStatusResponse> {
        // Validate address
        if request.address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "address is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Look up whether this address has a multi-sig configuration
        // - Return the configuration details if found
        //
        // For now, return "not multi-sig" status (no indexer = no multi-sig visible).
        Ok(types::social::MultisigStatusResponse::new(
            request.address,
            false,                                      // is_multisig
            None,                                       // key_count
            None,                                       // threshold
            None,                                       // setup_block
            None,                                       // public_keys
            types::social::MultisigStatus::NotMultisig, // status
        ))
    }

    async fn z_multisig_list(
        &self,
        request: types::social::MultisigListRequest,
    ) -> Result<types::social::MultisigListResponse> {
        // Validate limit
        if request.limit > 1000 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "limit cannot exceed 1000",
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Scan for all multi-sig setup transactions
        // - Filter by status if specified
        // - Return paginated results
        //
        // For now, return an empty list (no indexer = no multi-sig visible).
        Ok(types::social::MultisigListResponse::new(
            vec![],  // identities (empty)
            0,       // total_count
        ))
    }

    // ==================== Bridge RPC Method Implementations ====================

    async fn z_bridge_link(
        &self,
        request: types::social::BridgeLinkRequest,
    ) -> Result<types::social::BridgeLinkResponse> {
        // Validate from address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate platform_id length
        if request.platform_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "platformId is required",
                None::<()>,
            ));
        }

        if request.platform_id.len() > types::social::MAX_PLATFORM_ID_LENGTH {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "platformId exceeds maximum length of {} characters (got {})",
                    types::social::MAX_PLATFORM_ID_LENGTH,
                    request.platform_id.len()
                ),
                None::<()>,
            ));
        }

        // Validate proof is provided
        if request.proof.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "proof is required (signed challenge)",
                None::<()>,
            ));
        }

        // Validate proof is hex-encoded
        if !request.proof.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "proof must be hex-encoded",
                None::<()>,
            ));
        }

        // Validate proof length (should be at least 64 chars for a 32-byte signature)
        if request.proof.len() < 64 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "proof too short (minimum 64 hex chars for signature, got {})",
                    request.proof.len()
                ),
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet support to:
        // - Verify the proof (signed challenge) against the platform's verification mechanism
        // - Create a BridgeLink (0xB0) memo message
        // - Sign and broadcast the transaction
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_bridgelink requires wallet support which is not yet implemented in Zebra. \
                Would link {} identity {} to address {} with {:?} privacy mode",
                request.platform, request.platform_id, request.from, request.privacy_mode
            ),
            None::<()>,
        ))
    }

    async fn z_bridge_unlink(
        &self,
        request: types::social::BridgeUnlinkRequest,
    ) -> Result<types::social::BridgeUnlinkResponse> {
        // Validate from address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate platform_id
        if request.platform_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "platformId is required",
                None::<()>,
            ));
        }

        if request.platform_id.len() > types::social::MAX_PLATFORM_ID_LENGTH {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "platformId exceeds maximum length of {} characters (got {})",
                    types::social::MAX_PLATFORM_ID_LENGTH,
                    request.platform_id.len()
                ),
                None::<()>,
            ));
        }

        // Note: Full implementation requires wallet support to:
        // - Verify the from address owns the bridge link
        // - Create a BridgeUnlink (0xB1) memo message
        // - Sign and broadcast the transaction
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_bridgeunlink requires wallet support which is not yet implemented in Zebra. \
                Would unlink {} identity {} from address {}",
                request.platform, request.platform_id, request.from
            ),
            None::<()>,
        ))
    }

    async fn z_bridge_post(
        &self,
        request: types::social::BridgePostRequest,
    ) -> Result<types::social::BridgePostResponse> {
        // Validate from address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate original_id
        if request.original_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "originalId is required",
                None::<()>,
            ));
        }

        // Validate content
        if request.content.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "content is required",
                None::<()>,
            ));
        }

        // Validate content length (memo max is 512 bytes, need room for headers)
        const MAX_BRIDGE_CONTENT_SIZE: usize = 450;
        if request.content.len() > MAX_BRIDGE_CONTENT_SIZE {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "content exceeds maximum length of {} bytes (got {})",
                    MAX_BRIDGE_CONTENT_SIZE,
                    request.content.len()
                ),
                None::<()>,
            ));
        }

        // Validate in_reply_to if provided (should be 64 hex chars = 32 bytes txid)
        if let Some(ref reply_to) = request.in_reply_to {
            if reply_to.len() != 64 {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!(
                        "inReplyTo must be 64 hex characters (32 bytes txid), got {}",
                        reply_to.len()
                    ),
                    None::<()>,
                ));
            }
            if !reply_to.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    "inReplyTo must contain only hexadecimal characters",
                    None::<()>,
                ));
            }
        }

        // Note: Full implementation requires wallet support to:
        // - Verify the from address has an active bridge link for this platform
        // - Create a BridgePost (0xB2) memo message
        // - Sign and broadcast the transaction
        //
        // For now, return an error indicating wallet support is needed.
        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_bridgepost requires wallet support which is not yet implemented in Zebra. \
                Would post {} content from {} (original ID: {})",
                request.platform, request.from, request.original_id
            ),
            None::<()>,
        ))
    }

    async fn z_bridge_status(
        &self,
        request: types::social::BridgeStatusRequest,
    ) -> Result<types::social::BridgeStatusResponse> {
        // Validate address
        if request.address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "address is required",
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Look up all bridge links for the address
        // - Filter by platform if specified
        // - Return link details and activity stats
        //
        // For now, return an empty response (no indexer = no bridge links visible).
        Ok(types::social::BridgeStatusResponse::new(
            request.address, // address
            vec![],          // links (empty - no indexer)
            0,               // active_links_count
        ))
    }

    async fn z_bridge_list(
        &self,
        request: types::social::BridgeListRequest,
    ) -> Result<types::social::BridgeListResponse> {
        // Validate limit (reasonable bounds)
        const MAX_LIST_LIMIT: u32 = 1000;
        if request.limit > MAX_LIST_LIMIT {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "limit exceeds maximum of {} (got {})",
                    MAX_LIST_LIMIT, request.limit
                ),
                None::<()>,
            ));
        }

        // Note: Full implementation requires an indexer to:
        // - Query all bridge links
        // - Filter by platform and status if specified
        // - Apply pagination (limit/offset)
        // - Return link summaries
        //
        // For now, return an empty list (no indexer = no bridge links visible).
        Ok(types::social::BridgeListResponse::new(
            vec![], // links
            0,      // total_count
        ))
    }

    async fn z_bridge_verify(
        &self,
        request: types::social::BridgeVerifyRequest,
    ) -> Result<types::social::BridgeVerifyResponse> {
        // Validate address
        if request.address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "address is required",
                None::<()>,
            ));
        }

        // Validate platform_id
        if request.platform_id.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "platformId is required",
                None::<()>,
            ));
        }

        if request.platform_id.len() > types::social::MAX_PLATFORM_ID_LENGTH {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "platformId exceeds maximum length of {} characters (got {})",
                    types::social::MAX_PLATFORM_ID_LENGTH,
                    request.platform_id.len()
                ),
                None::<()>,
            ));
        }

        // Generate a challenge for verification
        // In production, this would be stored server-side with expiration
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(request.address.as_bytes());
        hasher.update(request.platform.to_string().as_bytes());
        hasher.update(request.platform_id.as_bytes());
        // Add timestamp for uniqueness (truncated to 10-minute windows for determinism in tests)
        let timestamp = chrono::Utc::now().timestamp() / 600 * 600;
        hasher.update(&timestamp.to_le_bytes());
        let challenge = hasher.finalize();

        // Calculate expiration time
        let expires_at = chrono::Utc::now().timestamp() + types::social::BRIDGE_CHALLENGE_EXPIRY_SECS;

        // Generate platform-specific instructions
        let instructions = match request.platform {
            types::social::BridgePlatform::Telegram => {
                "Send the challenge hash to the Botcash bridge bot using /verify command"
            }
            types::social::BridgePlatform::Discord => {
                "Post the challenge hash in your server's #botcash-verify channel"
            }
            types::social::BridgePlatform::Nostr => {
                "Sign the challenge with your Nostr private key and submit the signature"
            }
            types::social::BridgePlatform::Mastodon => {
                "Post the challenge hash as a public toot with #BotcashVerify hashtag"
            }
            types::social::BridgePlatform::Twitter => {
                "Tweet the challenge hash with #BotcashVerify (read-only bridge)"
            }
        };

        Ok(types::social::BridgeVerifyResponse::new(
            hex::encode(challenge), // challenge
            expires_at,             // expires_at
            instructions.to_string(), // instructions
        ))
    }

    // ==================== Moderation RPC Method Implementations ====================

    async fn z_trust(
        &self,
        request: types::social::TrustRequest,
    ) -> Result<types::social::TrustResponse> {
        // Validate the truster address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate the target address
        if request.target.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "target address is required",
                None::<()>,
            ));
        }

        // Cannot trust yourself
        if request.from == request.target {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "cannot trust yourself",
                None::<()>,
            ));
        }

        // Validate reason length if provided
        if let Some(ref reason) = request.reason {
            if reason.len() > types::social::MAX_TRUST_REASON_LENGTH {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!(
                        "reason exceeds maximum length of {} characters (got {})",
                        types::social::MAX_TRUST_REASON_LENGTH,
                        reason.len()
                    ),
                    None::<()>,
                ));
            }
        }

        // In production, this would:
        // 1. Build a Trust memo with the target, level, and reason
        // 2. Create a transaction with the memo attached
        // 3. Sign and submit via wallet
        // 4. Return the transaction ID

        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_trust requires wallet support which is not yet implemented in Zebra. \
                Would record {} trust from {} to {}{}",
                request.level,
                request.from,
                request.target,
                request.reason.as_ref().map(|r| format!(" (reason: {})", r)).unwrap_or_default()
            ),
            None::<()>,
        ))
    }

    async fn z_trust_query(
        &self,
        request: types::social::TrustQueryRequest,
    ) -> Result<types::social::TrustQueryResponse> {
        // Validate the address
        if request.address.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "address is required",
                None::<()>,
            ));
        }

        // Validate limit
        if request.limit > types::social::MAX_TRUST_LIMIT {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "limit exceeds maximum of {} (got {})",
                    types::social::MAX_TRUST_LIMIT,
                    request.limit
                ),
                None::<()>,
            ));
        }

        // In production, this would query the indexer for:
        // 1. All trust relationships involving this address
        // 2. Computed trust score based on web of trust algorithm
        // 3. Counts of trusted-by and distrusted-by relationships

        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_trustquery requires indexer support which is not yet implemented. \
                Would query trust for address {} (incoming: {}, outgoing: {}, limit: {})",
                request.address, request.include_incoming, request.include_outgoing, request.limit
            ),
            None::<()>,
        ))
    }

    async fn z_report(
        &self,
        request: types::social::ReportRequest,
    ) -> Result<types::social::ReportResponse> {
        // Validate the reporter address
        if request.from.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "from address is required",
                None::<()>,
            ));
        }

        // Validate the target txid
        if request.target_txid.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "targetTxid is required",
                None::<()>,
            ));
        }

        // Validate target txid format (should be 64 hex chars = 32 bytes)
        if request.target_txid.len() != 64 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "targetTxid must be 64 hex characters (got {})",
                    request.target_txid.len()
                ),
                None::<()>,
            ));
        }

        // Validate hex format
        if hex::decode(&request.target_txid).is_err() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "targetTxid must be valid hex",
                None::<()>,
            ));
        }

        // Validate minimum stake
        if request.stake < types::social::MIN_REPORT_STAKE {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "stake must be at least {} zatoshi (0.01 BCASH), got {}",
                    types::social::MIN_REPORT_STAKE,
                    request.stake
                ),
                None::<()>,
            ));
        }

        // Validate evidence length if provided
        if let Some(ref evidence) = request.evidence {
            if evidence.len() > types::social::MAX_REPORT_EVIDENCE_LENGTH {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!(
                        "evidence exceeds maximum length of {} characters (got {})",
                        types::social::MAX_REPORT_EVIDENCE_LENGTH,
                        evidence.len()
                    ),
                    None::<()>,
                ));
            }
        }

        // In production, this would:
        // 1. Build a Report memo with target, category, stake, and evidence
        // 2. Create a transaction with the memo attached and stake locked
        // 3. Sign and submit via wallet
        // 4. Return the transaction ID

        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_report requires wallet support which is not yet implemented in Zebra. \
                Would submit {} report against tx {} with {} zatoshi stake{}",
                request.category,
                request.target_txid,
                request.stake,
                request.evidence.as_ref().map(|e| format!(" (evidence: {})", e)).unwrap_or_default()
            ),
            None::<()>,
        ))
    }

    async fn z_report_status(
        &self,
        request: types::social::ReportStatusRequest,
    ) -> Result<types::social::ReportStatusResponse> {
        // Validate the report txid
        if request.report_txid.is_empty() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "reportTxid is required",
                None::<()>,
            ));
        }

        // Validate txid format (should be 64 hex chars = 32 bytes)
        if request.report_txid.len() != 64 {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "reportTxid must be 64 hex characters (got {})",
                    request.report_txid.len()
                ),
                None::<()>,
            ));
        }

        // Validate hex format
        if hex::decode(&request.report_txid).is_err() {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                "reportTxid must be valid hex",
                None::<()>,
            ));
        }

        // In production, this would query the indexer for:
        // 1. The report transaction details
        // 2. Current status (pending, validated, rejected, expired)
        // 3. Resolution details if decided

        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_reportstatus requires indexer support which is not yet implemented. \
                Would query status for report {}",
                request.report_txid
            ),
            None::<()>,
        ))
    }

    async fn z_report_list(
        &self,
        request: types::social::ReportListRequest,
    ) -> Result<types::social::ReportListResponse> {
        // Validate limit
        if request.limit > types::social::MAX_REPORT_LIMIT {
            return Err(ErrorObject::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "limit exceeds maximum of {} (got {})",
                    types::social::MAX_REPORT_LIMIT,
                    request.limit
                ),
                None::<()>,
            ));
        }

        // Validate target txid format if provided
        if let Some(ref txid) = request.target_txid {
            if txid.len() != 64 {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    format!(
                        "targetTxid must be 64 hex characters (got {})",
                        txid.len()
                    ),
                    None::<()>,
                ));
            }
            if hex::decode(txid).is_err() {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    "targetTxid must be valid hex",
                    None::<()>,
                ));
            }
        }

        // Validate reporter address if provided
        if let Some(ref addr) = request.reporter_address {
            if addr.is_empty() {
                return Err(ErrorObject::owned(
                    ErrorCode::InvalidParams.code(),
                    "reporterAddress cannot be empty if provided",
                    None::<()>,
                ));
            }
        }

        // In production, this would query the indexer for:
        // 1. Reports matching the filter criteria
        // 2. Total count for pagination
        // 3. Ordered by block height descending

        Err(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!(
                "z_reportlist requires indexer support which is not yet implemented. \
                Would list reports with filters: target={:?}, reporter={:?}, category={:?}, status={:?}, limit={}",
                request.target_txid, request.reporter_address, request.category, request.status, request.limit
            ),
            None::<()>,
        ))
    }
}

// TODO: Move the code below to separate modules.

/// Returns the best chain tip height of `latest_chain_tip`,
/// or an RPC error if there are no blocks in the state.
pub fn best_chain_tip_height<Tip>(latest_chain_tip: &Tip) -> Result<Height>
where
    Tip: ChainTip + Clone + Send + Sync + 'static,
{
    latest_chain_tip
        .best_tip_height()
        .ok_or_misc_error("No blocks in state")
}

/// Response to a `getinfo` RPC request.
///
/// See the notes for the [`Rpc::get_info` method].
#[allow(clippy::too_many_arguments)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Getters, new)]
pub struct GetInfoResponse {
    /// The node version
    #[getter(rename = "raw_version")]
    version: u64,

    /// The node version build number
    build: String,

    /// The server sub-version identifier, used as the network protocol user-agent
    subversion: String,

    /// The protocol version
    #[serde(rename = "protocolversion")]
    protocol_version: u32,

    /// The current number of blocks processed in the server
    blocks: u32,

    /// The total (inbound and outbound) number of connections the node has
    connections: usize,

    /// The proxy (if any) used by the server. Currently always `None` in Zebra.
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<String>,

    /// The current network difficulty
    difficulty: f64,

    /// True if the server is running in testnet mode, false otherwise
    testnet: bool,

    /// The minimum transaction fee in ZEC/kB
    #[serde(rename = "paytxfee")]
    pay_tx_fee: f64,

    /// The minimum relay fee for non-free transactions in ZEC/kB
    #[serde(rename = "relayfee")]
    relay_fee: f64,

    /// The last error or warning message, or "no errors" if there are no errors
    errors: String,

    /// The time of the last error or warning message, or "no errors timestamp" if there are no errors
    #[serde(rename = "errorstimestamp")]
    errors_timestamp: String,
}

#[deprecated(note = "Use `GetInfoResponse` instead")]
pub use self::GetInfoResponse as GetInfo;

impl Default for GetInfoResponse {
    fn default() -> Self {
        GetInfoResponse {
            version: 0,
            build: "some build version".to_string(),
            subversion: "some subversion".to_string(),
            protocol_version: 0,
            blocks: 0,
            connections: 0,
            proxy: None,
            difficulty: 0.0,
            testnet: false,
            pay_tx_fee: 0.0,
            relay_fee: 0.0,
            errors: "no errors".to_string(),
            errors_timestamp: "no errors timestamp".to_string(),
        }
    }
}

impl GetInfoResponse {
    /// Constructs [`GetInfo`] from its constituent parts.
    #[allow(clippy::too_many_arguments)]
    #[deprecated(note = "Use `GetInfoResponse::new` instead")]
    pub fn from_parts(
        version: u64,
        build: String,
        subversion: String,
        protocol_version: u32,
        blocks: u32,
        connections: usize,
        proxy: Option<String>,
        difficulty: f64,
        testnet: bool,
        pay_tx_fee: f64,
        relay_fee: f64,
        errors: String,
        errors_timestamp: String,
    ) -> Self {
        Self {
            version,
            build,
            subversion,
            protocol_version,
            blocks,
            connections,
            proxy,
            difficulty,
            testnet,
            pay_tx_fee,
            relay_fee,
            errors,
            errors_timestamp,
        }
    }

    /// Returns the contents of ['GetInfo'].
    pub fn into_parts(
        self,
    ) -> (
        u64,
        String,
        String,
        u32,
        u32,
        usize,
        Option<String>,
        f64,
        bool,
        f64,
        f64,
        String,
        String,
    ) {
        (
            self.version,
            self.build,
            self.subversion,
            self.protocol_version,
            self.blocks,
            self.connections,
            self.proxy,
            self.difficulty,
            self.testnet,
            self.pay_tx_fee,
            self.relay_fee,
            self.errors,
            self.errors_timestamp,
        )
    }

    /// Create the node version number.
    fn version_from_string(build_string: &str) -> Option<u64> {
        let semver_version = semver::Version::parse(build_string.strip_prefix('v')?).ok()?;
        let build_number = semver_version
            .build
            .as_str()
            .split('.')
            .next()
            .and_then(|num_str| num_str.parse::<u64>().ok())
            .unwrap_or_default();

        // https://github.com/zcash/zcash/blob/v6.1.0/src/clientversion.h#L55-L59
        let version_number = 1_000_000 * semver_version.major
            + 10_000 * semver_version.minor
            + 100 * semver_version.patch
            + build_number;

        Some(version_number)
    }
}

/// Type alias for the array of `GetBlockchainInfoBalance` objects
pub type BlockchainValuePoolBalances = [GetBlockchainInfoBalance; 5];

/// Response to a `getblockchaininfo` RPC request.
///
/// See the notes for the [`Rpc::get_blockchain_info` method].
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Getters)]
pub struct GetBlockchainInfoResponse {
    /// Current network name as defined in BIP70 (main, test, regtest)
    chain: String,

    /// The current number of blocks processed in the server, numeric
    #[getter(copy)]
    blocks: Height,

    /// The current number of headers we have validated in the best chain, that is,
    /// the height of the best chain.
    #[getter(copy)]
    headers: Height,

    /// The estimated network solution rate in Sol/s.
    difficulty: f64,

    /// The verification progress relative to the estimated network chain tip.
    #[serde(rename = "verificationprogress")]
    verification_progress: f64,

    /// The total amount of work in the best chain, hex-encoded.
    #[serde(rename = "chainwork")]
    chain_work: u64,

    /// Whether this node is pruned, currently always false in Zebra.
    pruned: bool,

    /// The estimated size of the block and undo files on disk
    size_on_disk: u64,

    /// The current number of note commitments in the commitment tree
    commitments: u64,

    /// The hash of the currently best block, in big-endian order, hex-encoded
    #[serde(rename = "bestblockhash", with = "hex")]
    #[getter(copy)]
    best_block_hash: block::Hash,

    /// If syncing, the estimated height of the chain, else the current best height, numeric.
    ///
    /// In Zebra, this is always the height estimate, so it might be a little inaccurate.
    #[serde(rename = "estimatedheight")]
    #[getter(copy)]
    estimated_height: Height,

    /// Chain supply balance
    #[serde(rename = "chainSupply")]
    chain_supply: GetBlockchainInfoBalance,

    /// Value pool balances
    #[serde(rename = "valuePools")]
    value_pools: BlockchainValuePoolBalances,

    /// Status of network upgrades
    upgrades: IndexMap<ConsensusBranchIdHex, NetworkUpgradeInfo>,

    /// Branch IDs of the current and upcoming consensus rules
    #[getter(copy)]
    consensus: TipConsensusBranch,
}

impl Default for GetBlockchainInfoResponse {
    fn default() -> Self {
        Self {
            chain: "main".to_string(),
            blocks: Height(1),
            best_block_hash: block::Hash([0; 32]),
            estimated_height: Height(1),
            chain_supply: GetBlockchainInfoBalance::chain_supply(Default::default()),
            value_pools: GetBlockchainInfoBalance::zero_pools(),
            upgrades: IndexMap::new(),
            consensus: TipConsensusBranch {
                chain_tip: ConsensusBranchIdHex(ConsensusBranchId::default()),
                next_block: ConsensusBranchIdHex(ConsensusBranchId::default()),
            },
            headers: Height(1),
            difficulty: 0.0,
            verification_progress: 0.0,
            chain_work: 0,
            pruned: false,
            size_on_disk: 0,
            commitments: 0,
        }
    }
}

impl GetBlockchainInfoResponse {
    /// Creates a new [`GetBlockchainInfoResponse`] instance.
    // We don't use derive(new) because the method already existed but the arguments
    // have a different order. No reason to unnecessarily break existing code.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain: String,
        blocks: Height,
        best_block_hash: block::Hash,
        estimated_height: Height,
        chain_supply: GetBlockchainInfoBalance,
        value_pools: BlockchainValuePoolBalances,
        upgrades: IndexMap<ConsensusBranchIdHex, NetworkUpgradeInfo>,
        consensus: TipConsensusBranch,
        headers: Height,
        difficulty: f64,
        verification_progress: f64,
        chain_work: u64,
        pruned: bool,
        size_on_disk: u64,
        commitments: u64,
    ) -> Self {
        Self {
            chain,
            blocks,
            best_block_hash,
            estimated_height,
            chain_supply,
            value_pools,
            upgrades,
            consensus,
            headers,
            difficulty,
            verification_progress,
            chain_work,
            pruned,
            size_on_disk,
            commitments,
        }
    }
}

/// A wrapper type with a list of transparent address strings.
///
/// This is used for the input parameter of [`RpcServer::get_address_balance`],
/// [`RpcServer::get_address_tx_ids`] and [`RpcServer::get_address_utxos`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(from = "DAddressStrings")]
pub struct AddressStrings {
    /// A list of transparent address strings.
    addresses: Vec<String>,
}

impl From<DAddressStrings> for AddressStrings {
    fn from(address_strings: DAddressStrings) -> Self {
        match address_strings {
            DAddressStrings::Addresses { addresses } => AddressStrings { addresses },
            DAddressStrings::Address(address) => AddressStrings {
                addresses: vec![address],
            },
        }
    }
}

/// An intermediate type used to deserialize [`AddressStrings`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Deserialize)]
#[serde(untagged)]
enum DAddressStrings {
    /// A list of address strings.
    Addresses { addresses: Vec<String> },
    /// A single address string.
    Address(String),
}

/// A request to get the transparent balance of a set of addresses.
pub type GetAddressBalanceRequest = AddressStrings;

impl AddressStrings {
    /// Creates a new `AddressStrings` given a vector.
    pub fn new(addresses: Vec<String>) -> AddressStrings {
        AddressStrings { addresses }
    }

    /// Creates a new [`AddressStrings`] from a given vector, returns an error if any addresses are incorrect.
    #[deprecated(
        note = "Use `AddressStrings::new` instead. Validity will be checked by the server."
    )]
    pub fn new_valid(addresses: Vec<String>) -> Result<AddressStrings> {
        let address_strings = Self { addresses };
        address_strings.clone().valid_addresses()?;
        Ok(address_strings)
    }

    /// Given a list of addresses as strings:
    /// - check if provided list have all valid transparent addresses.
    /// - return valid addresses as a set of `Address`.
    pub fn valid_addresses(self) -> Result<HashSet<Address>> {
        // Reference for the legacy error code:
        // <https://github.com/zcash/zcash/blob/99ad6fdc3a549ab510422820eea5e5ce9f60a5fd/src/rpc/misc.cpp#L783-L784>
        let valid_addresses: HashSet<Address> = self
            .addresses
            .into_iter()
            .map(|address| {
                address
                    .parse()
                    .map_error(server::error::LegacyCode::InvalidAddressOrKey)
            })
            .collect::<Result<_>>()?;

        Ok(valid_addresses)
    }

    /// Given a list of addresses as strings:
    /// - check if provided list have all valid transparent addresses.
    /// - return valid addresses as a vec of strings.
    pub fn valid_address_strings(self) -> Result<Vec<String>> {
        self.clone().valid_addresses()?;
        Ok(self.addresses)
    }
}

/// The transparent balance of a set of addresses.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    Getters,
    new,
)]
pub struct GetAddressBalanceResponse {
    /// The total transparent balance.
    balance: u64,
    /// The total received balance, including change.
    pub received: u64,
}

#[deprecated(note = "Use `GetAddressBalanceResponse` instead.")]
pub use self::GetAddressBalanceResponse as AddressBalance;

/// A hex-encoded [`ConsensusBranchId`] string.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ConsensusBranchIdHex(#[serde(with = "hex")] ConsensusBranchId);

impl ConsensusBranchIdHex {
    /// Returns a new instance of ['ConsensusBranchIdHex'].
    pub fn new(consensus_branch_id: u32) -> Self {
        ConsensusBranchIdHex(consensus_branch_id.into())
    }

    /// Returns the value of the ['ConsensusBranchId'].
    pub fn inner(&self) -> u32 {
        self.0.into()
    }
}

/// Information about [`NetworkUpgrade`] activation.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NetworkUpgradeInfo {
    /// Name of upgrade, string.
    ///
    /// Ignored by lightwalletd, but useful for debugging.
    name: NetworkUpgrade,

    /// Block height of activation, numeric.
    #[serde(rename = "activationheight")]
    activation_height: Height,

    /// Status of upgrade, string.
    status: NetworkUpgradeStatus,
}

impl NetworkUpgradeInfo {
    /// Constructs [`NetworkUpgradeInfo`] from its constituent parts.
    pub fn from_parts(
        name: NetworkUpgrade,
        activation_height: Height,
        status: NetworkUpgradeStatus,
    ) -> Self {
        Self {
            name,
            activation_height,
            status,
        }
    }

    /// Returns the contents of ['NetworkUpgradeInfo'].
    pub fn into_parts(self) -> (NetworkUpgrade, Height, NetworkUpgradeStatus) {
        (self.name, self.activation_height, self.status)
    }
}

/// The activation status of a [`NetworkUpgrade`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum NetworkUpgradeStatus {
    /// The network upgrade is currently active.
    ///
    /// Includes all network upgrades that have previously activated,
    /// even if they are not the most recent network upgrade.
    #[serde(rename = "active")]
    Active,

    /// The network upgrade does not have an activation height.
    #[serde(rename = "disabled")]
    Disabled,

    /// The network upgrade has an activation height, but we haven't reached it yet.
    #[serde(rename = "pending")]
    Pending,
}

/// The [`ConsensusBranchId`]s for the tip and the next block.
///
/// These branch IDs are different when the next block is a network upgrade activation block.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TipConsensusBranch {
    /// Branch ID used to validate the current chain tip, big-endian, hex-encoded.
    #[serde(rename = "chaintip")]
    chain_tip: ConsensusBranchIdHex,

    /// Branch ID used to validate the next block, big-endian, hex-encoded.
    #[serde(rename = "nextblock")]
    next_block: ConsensusBranchIdHex,
}

impl TipConsensusBranch {
    /// Constructs [`TipConsensusBranch`] from its constituent parts.
    pub fn from_parts(chain_tip: u32, next_block: u32) -> Self {
        Self {
            chain_tip: ConsensusBranchIdHex::new(chain_tip),
            next_block: ConsensusBranchIdHex::new(next_block),
        }
    }

    /// Returns the contents of ['TipConsensusBranch'].
    pub fn into_parts(self) -> (u32, u32) {
        (self.chain_tip.inner(), self.next_block.inner())
    }
}

/// Response to a `sendrawtransaction` RPC request.
///
/// Contains the hex-encoded hash of the sent transaction.
///
/// See the notes for the [`Rpc::send_raw_transaction` method].
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SendRawTransactionResponse(#[serde(with = "hex")] transaction::Hash);

#[deprecated(note = "Use `SendRawTransactionResponse` instead")]
pub use self::SendRawTransactionResponse as SentTransactionHash;

impl Default for SendRawTransactionResponse {
    fn default() -> Self {
        Self(transaction::Hash::from([0; 32]))
    }
}

impl SendRawTransactionResponse {
    /// Constructs a new [`SentTransactionHash`].
    pub fn new(hash: transaction::Hash) -> Self {
        SendRawTransactionResponse(hash)
    }

    /// Returns the contents of ['SentTransactionHash'].
    #[deprecated(note = "Use `SentTransactionHash::hash` instead")]
    pub fn inner(&self) -> transaction::Hash {
        self.hash()
    }

    /// Returns the contents of ['SentTransactionHash'].
    pub fn hash(&self) -> transaction::Hash {
        self.0
    }
}

/// Response to a `getblock` RPC request.
///
/// See the notes for the [`RpcServer::get_block`] method.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GetBlockResponse {
    /// The request block, hex-encoded.
    Raw(#[serde(with = "hex")] SerializedBlock),
    /// The block object.
    Object(Box<BlockObject>),
}

#[deprecated(note = "Use `GetBlockResponse` instead")]
pub use self::GetBlockResponse as GetBlock;

impl Default for GetBlockResponse {
    fn default() -> Self {
        GetBlockResponse::Object(Box::new(BlockObject {
            hash: block::Hash([0; 32]),
            confirmations: 0,
            height: None,
            time: None,
            tx: Vec::new(),
            trees: GetBlockTrees::default(),
            size: None,
            version: None,
            merkle_root: None,
            block_commitments: None,
            final_sapling_root: None,
            final_orchard_root: None,
            nonce: None,
            bits: None,
            difficulty: None,
            chain_supply: None,
            value_pools: None,
            previous_block_hash: None,
            next_block_hash: None,
            solution: None,
        }))
    }
}

/// A Block object returned by the `getblock` RPC request.
#[allow(clippy::too_many_arguments)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Getters, new)]
pub struct BlockObject {
    /// The hash of the requested block.
    #[getter(copy)]
    #[serde(with = "hex")]
    hash: block::Hash,

    /// The number of confirmations of this block in the best chain,
    /// or -1 if it is not in the best chain.
    confirmations: i64,

    /// The block size. TODO: fill it
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    size: Option<i64>,

    /// The height of the requested block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    height: Option<Height>,

    /// The version field of the requested block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    version: Option<u32>,

    /// The merkle root of the requested block.
    #[serde(with = "opthex", rename = "merkleroot")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    merkle_root: Option<block::merkle::Root>,

    /// The blockcommitments field of the requested block. Its interpretation changes
    /// depending on the network and height.
    #[serde(with = "opthex", rename = "blockcommitments")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    block_commitments: Option<[u8; 32]>,

    // `authdataroot` would be here. Undocumented. TODO: decide if we want to support it
    //
    /// The root of the Sapling commitment tree after applying this block.
    #[serde(with = "opthex", rename = "finalsaplingroot")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    final_sapling_root: Option<[u8; 32]>,

    /// The root of the Orchard commitment tree after applying this block.
    #[serde(with = "opthex", rename = "finalorchardroot")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    final_orchard_root: Option<[u8; 32]>,

    // `chainhistoryroot` would be here. Undocumented. TODO: decide if we want to support it
    //
    /// List of transactions in block order, hex-encoded if verbosity=1 or
    /// as objects if verbosity=2.
    tx: Vec<GetBlockTransaction>,

    /// The height of the requested block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    time: Option<i64>,

    /// The nonce of the requested block header.
    #[serde(with = "opthex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    nonce: Option<[u8; 32]>,

    /// The Equihash solution in the requested block header.
    /// Note: presence of this field in getblock is not documented in zcashd.
    #[serde(with = "opthex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    solution: Option<Solution>,

    /// The difficulty threshold of the requested block header displayed in compact form.
    #[serde(with = "opthex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    bits: Option<CompactDifficulty>,

    /// Floating point number that represents the difficulty limit for this block as a multiple
    /// of the minimum difficulty for the network.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    difficulty: Option<f64>,

    // `chainwork` would be here, but we don't plan on supporting it
    // `anchor` would be here. Not planned to be supported.
    //
    /// Chain supply balance
    #[serde(rename = "chainSupply")]
    #[serde(skip_serializing_if = "Option::is_none")]
    chain_supply: Option<GetBlockchainInfoBalance>,

    /// Value pool balances
    #[serde(rename = "valuePools")]
    #[serde(skip_serializing_if = "Option::is_none")]
    value_pools: Option<BlockchainValuePoolBalances>,

    /// Information about the note commitment trees.
    #[getter(copy)]
    trees: GetBlockTrees,

    /// The previous block hash of the requested block header.
    #[serde(rename = "previousblockhash", skip_serializing_if = "Option::is_none")]
    #[serde(with = "opthex")]
    #[getter(copy)]
    previous_block_hash: Option<block::Hash>,

    /// The next block hash after the requested block header.
    #[serde(rename = "nextblockhash", skip_serializing_if = "Option::is_none")]
    #[serde(with = "opthex")]
    #[getter(copy)]
    next_block_hash: Option<block::Hash>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
/// The transaction list in a `getblock` call. Can be a list of transaction
/// IDs or the full transaction details depending on verbosity.
pub enum GetBlockTransaction {
    /// The transaction hash, hex-encoded.
    Hash(#[serde(with = "hex")] transaction::Hash),
    /// The block object.
    Object(Box<TransactionObject>),
}

/// Response to a `getblockheader` RPC request.
///
/// See the notes for the [`RpcServer::get_block_header`] method.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GetBlockHeaderResponse {
    /// The request block header, hex-encoded.
    Raw(hex_data::HexData),

    /// The block header object.
    Object(Box<BlockHeaderObject>),
}

#[deprecated(note = "Use `GetBlockHeaderResponse` instead")]
pub use self::GetBlockHeaderResponse as GetBlockHeader;

#[allow(clippy::too_many_arguments)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Getters, new)]
/// Verbose response to a `getblockheader` RPC request.
///
/// See the notes for the [`RpcServer::get_block_header`] method.
pub struct BlockHeaderObject {
    /// The hash of the requested block.
    #[serde(with = "hex")]
    #[getter(copy)]
    hash: block::Hash,

    /// The number of confirmations of this block in the best chain,
    /// or -1 if it is not in the best chain.
    confirmations: i64,

    /// The height of the requested block.
    #[getter(copy)]
    height: Height,

    /// The version field of the requested block.
    version: u32,

    /// The merkle root of the requesteed block.
    #[serde(with = "hex", rename = "merkleroot")]
    #[getter(copy)]
    merkle_root: block::merkle::Root,

    /// The blockcommitments field of the requested block. Its interpretation changes
    /// depending on the network and height.
    #[serde(with = "hex", rename = "blockcommitments")]
    #[getter(copy)]
    block_commitments: [u8; 32],

    /// The root of the Sapling commitment tree after applying this block.
    #[serde(with = "hex", rename = "finalsaplingroot")]
    #[getter(copy)]
    final_sapling_root: [u8; 32],

    /// The number of Sapling notes in the Sapling note commitment tree
    /// after applying this block. Used by the `getblock` RPC method.
    #[serde(skip)]
    sapling_tree_size: u64,

    /// The block time of the requested block header in non-leap seconds since Jan 1 1970 GMT.
    time: i64,

    /// The nonce of the requested block header.
    #[serde(with = "hex")]
    #[getter(copy)]
    nonce: [u8; 32],

    /// The Equihash solution in the requested block header.
    #[serde(with = "hex")]
    #[getter(copy)]
    solution: Solution,

    /// The difficulty threshold of the requested block header displayed in compact form.
    #[serde(with = "hex")]
    #[getter(copy)]
    bits: CompactDifficulty,

    /// Floating point number that represents the difficulty limit for this block as a multiple
    /// of the minimum difficulty for the network.
    difficulty: f64,

    /// The previous block hash of the requested block header.
    #[serde(rename = "previousblockhash")]
    #[serde(with = "hex")]
    #[getter(copy)]
    previous_block_hash: block::Hash,

    /// The next block hash after the requested block header.
    #[serde(rename = "nextblockhash", skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    #[serde(with = "opthex")]
    next_block_hash: Option<block::Hash>,
}

#[deprecated(note = "Use `BlockHeaderObject` instead")]
pub use BlockHeaderObject as GetBlockHeaderObject;

impl Default for GetBlockHeaderResponse {
    fn default() -> Self {
        GetBlockHeaderResponse::Object(Box::default())
    }
}

impl Default for BlockHeaderObject {
    fn default() -> Self {
        let difficulty: ExpandedDifficulty = zebra_chain::work::difficulty::U256::one().into();

        BlockHeaderObject {
            hash: block::Hash([0; 32]),
            confirmations: 0,
            height: Height::MIN,
            version: 4,
            merkle_root: block::merkle::Root([0; 32]),
            block_commitments: Default::default(),
            final_sapling_root: Default::default(),
            sapling_tree_size: Default::default(),
            time: 0,
            nonce: [0; 32],
            solution: Solution::for_proposal(),
            bits: difficulty.to_compact(),
            difficulty: 1.0,
            previous_block_hash: block::Hash([0; 32]),
            next_block_hash: Some(block::Hash([0; 32])),
        }
    }
}

/// Response to a `getbestblockhash` and `getblockhash` RPC request.
///
/// Contains the hex-encoded hash of the requested block.
///
/// Also see the notes for the [`RpcServer::get_best_block_hash`] and `get_block_hash` methods.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct GetBlockHashResponse(#[serde(with = "hex")] pub(crate) block::Hash);

impl GetBlockHashResponse {
    /// Constructs a new [`GetBlockHashResponse`] from a block hash.
    pub fn new(hash: block::Hash) -> Self {
        GetBlockHashResponse(hash)
    }

    /// Returns the contents of [`GetBlockHashResponse`].
    pub fn hash(&self) -> block::Hash {
        self.0
    }
}

#[deprecated(note = "Use `GetBlockHashResponse` instead")]
pub use self::GetBlockHashResponse as GetBlockHash;

/// A block hash used by this crate that encodes as hex by default.
pub type Hash = GetBlockHashResponse;

/// Response to a `getbestblockheightandhash` RPC request.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Getters, new)]
pub struct GetBlockHeightAndHashResponse {
    /// The best chain tip block height
    #[getter(copy)]
    height: block::Height,
    /// The best chain tip block hash
    #[getter(copy)]
    hash: block::Hash,
}

#[deprecated(note = "Use `GetBlockHeightAndHashResponse` instead.")]
pub use GetBlockHeightAndHashResponse as GetBestBlockHeightAndHash;

impl Default for GetBlockHeightAndHashResponse {
    fn default() -> Self {
        Self {
            height: block::Height::MIN,
            hash: block::Hash([0; 32]),
        }
    }
}

impl Default for GetBlockHashResponse {
    fn default() -> Self {
        GetBlockHashResponse(block::Hash([0; 32]))
    }
}

/// Response to a `getrawtransaction` RPC request.
///
/// See the notes for the [`Rpc::get_raw_transaction` method].
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GetRawTransactionResponse {
    /// The raw transaction, encoded as hex bytes.
    Raw(#[serde(with = "hex")] SerializedTransaction),
    /// The transaction object.
    Object(Box<TransactionObject>),
}

#[deprecated(note = "Use `GetRawTransactionResponse` instead")]
pub use self::GetRawTransactionResponse as GetRawTransaction;

impl Default for GetRawTransactionResponse {
    fn default() -> Self {
        Self::Object(Box::default())
    }
}

/// Response to a `getaddressutxos` RPC request.
pub type GetAddressUtxosResponse = Vec<Utxo>;

/// A UTXO returned by the `getaddressutxos` RPC request.
///
/// See the notes for the [`Rpc::get_address_utxos` method].
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, Getters, new)]
pub struct Utxo {
    /// The transparent address, base58check encoded
    address: transparent::Address,

    /// The output txid, in big-endian order, hex-encoded
    #[serde(with = "hex")]
    #[getter(copy)]
    txid: transaction::Hash,

    /// The transparent output index, numeric
    #[serde(rename = "outputIndex")]
    #[getter(copy)]
    output_index: OutputIndex,

    /// The transparent output script, hex encoded
    #[serde(with = "hex")]
    script: transparent::Script,

    /// The amount of zatoshis in the transparent output
    satoshis: u64,

    /// The block height, numeric.
    ///
    /// We put this field last, to match the zcashd order.
    #[getter(copy)]
    height: Height,
}

#[deprecated(note = "Use `Utxo` instead")]
pub use self::Utxo as GetAddressUtxos;

impl Default for Utxo {
    fn default() -> Self {
        Self {
            address: transparent::Address::from_pub_key_hash(
                zebra_chain::parameters::NetworkKind::default(),
                [0u8; 20],
            ),
            txid: transaction::Hash::from([0; 32]),
            output_index: OutputIndex::from_u64(0),
            script: transparent::Script::new(&[0u8; 10]),
            satoshis: u64::default(),
            height: Height(0),
        }
    }
}

impl Utxo {
    /// Constructs a new instance of [`GetAddressUtxos`].
    #[deprecated(note = "Use `Utxo::new` instead")]
    pub fn from_parts(
        address: transparent::Address,
        txid: transaction::Hash,
        output_index: OutputIndex,
        script: transparent::Script,
        satoshis: u64,
        height: Height,
    ) -> Self {
        Utxo {
            address,
            txid,
            output_index,
            script,
            satoshis,
            height,
        }
    }

    /// Returns the contents of [`GetAddressUtxos`].
    pub fn into_parts(
        &self,
    ) -> (
        transparent::Address,
        transaction::Hash,
        OutputIndex,
        transparent::Script,
        u64,
        Height,
    ) {
        (
            self.address.clone(),
            self.txid,
            self.output_index,
            self.script.clone(),
            self.satoshis,
            self.height,
        )
    }
}

/// A struct to use as parameter of the `getaddresstxids`.
///
/// See the notes for the [`Rpc::get_address_tx_ids` method].
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Getters, new)]
pub struct GetAddressTxIdsRequest {
    // A list of addresses to get transactions from.
    addresses: Vec<String>,
    // The height to start looking for transactions.
    start: Option<u32>,
    // The height to end looking for transactions.
    end: Option<u32>,
}

impl GetAddressTxIdsRequest {
    /// Constructs [`GetAddressTxIdsRequest`] from its constituent parts.
    #[deprecated(note = "Use `GetAddressTxIdsRequest::new` instead.")]
    pub fn from_parts(addresses: Vec<String>, start: u32, end: u32) -> Self {
        GetAddressTxIdsRequest {
            addresses,
            start: Some(start),
            end: Some(end),
        }
    }

    /// Returns the contents of [`GetAddressTxIdsRequest`].
    pub fn into_parts(&self) -> (Vec<String>, u32, u32) {
        (
            self.addresses.clone(),
            self.start.unwrap_or(0),
            self.end.unwrap_or(0),
        )
    }
}

/// Information about the sapling and orchard note commitment trees if any.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GetBlockTrees {
    #[serde(skip_serializing_if = "SaplingTrees::is_empty")]
    sapling: SaplingTrees,
    #[serde(skip_serializing_if = "OrchardTrees::is_empty")]
    orchard: OrchardTrees,
}

impl Default for GetBlockTrees {
    fn default() -> Self {
        GetBlockTrees {
            sapling: SaplingTrees { size: 0 },
            orchard: OrchardTrees { size: 0 },
        }
    }
}

impl GetBlockTrees {
    /// Constructs a new instance of ['GetBlockTrees'].
    pub fn new(sapling: u64, orchard: u64) -> Self {
        GetBlockTrees {
            sapling: SaplingTrees { size: sapling },
            orchard: OrchardTrees { size: orchard },
        }
    }

    /// Returns sapling data held by ['GetBlockTrees'].
    pub fn sapling(self) -> u64 {
        self.sapling.size
    }

    /// Returns orchard data held by ['GetBlockTrees'].
    pub fn orchard(self) -> u64 {
        self.orchard.size
    }
}

/// Sapling note commitment tree information.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SaplingTrees {
    size: u64,
}

impl SaplingTrees {
    fn is_empty(&self) -> bool {
        self.size == 0
    }
}

/// Orchard note commitment tree information.
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OrchardTrees {
    size: u64,
}

impl OrchardTrees {
    fn is_empty(&self) -> bool {
        self.size == 0
    }
}

/// Build a valid height range from the given optional start and end numbers.
///
/// # Parameters
///
/// - `start`: Optional starting height. If not provided, defaults to 0.
/// - `end`: Optional ending height. A value of 0 or absence of a value indicates to use `chain_height`.
/// - `chain_height`: The maximum permissible height.
///
/// # Returns
///
/// A `RangeInclusive<Height>` from the clamped start to the clamped end.
///
/// # Errors
///
/// Returns an error if the computed start is greater than the computed end.
fn build_height_range(
    start: Option<u32>,
    end: Option<u32>,
    chain_height: Height,
) -> Result<RangeInclusive<Height>> {
    // Convert optional values to Height, using 0 (as Height(0)) when missing.
    // If start is above chain_height, clamp it to chain_height.
    let start = Height(start.unwrap_or(0)).min(chain_height);

    // For `end`, treat a zero value or missing value as `chain_height`:
    let end = match end {
        Some(0) | None => chain_height,
        Some(val) => Height(val).min(chain_height),
    };

    if start > end {
        return Err(ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            format!("start {start:?} must be less than or equal to end {end:?}"),
            None::<()>,
        ));
    }

    Ok(start..=end)
}

/// Given a potentially negative index, find the corresponding `Height`.
///
/// This function is used to parse the integer index argument of `get_block_hash`.
/// This is based on zcashd's implementation:
/// <https://github.com/zcash/zcash/blob/c267c3ee26510a974554f227d40a89e3ceb5bb4d/src/rpc/blockchain.cpp#L589-L618>
//
// TODO: also use this function in `get_block` and `z_get_treestate`
#[allow(dead_code)]
pub fn height_from_signed_int(index: i32, tip_height: Height) -> Result<Height> {
    if index >= 0 {
        let height = index.try_into().expect("Positive i32 always fits in u32");
        if height > tip_height.0 {
            return Err(ErrorObject::borrowed(
                ErrorCode::InvalidParams.code(),
                "Provided index is greater than the current tip",
                None,
            ));
        }
        Ok(Height(height))
    } else {
        // `index + 1` can't overflow, because `index` is always negative here.
        let height = i32::try_from(tip_height.0)
            .expect("tip height fits in i32, because Height::MAX fits in i32")
            .checked_add(index + 1);

        let sanitized_height = match height {
            None => {
                return Err(ErrorObject::borrowed(
                    ErrorCode::InvalidParams.code(),
                    "Provided index is not valid",
                    None,
                ));
            }
            Some(h) => {
                if h < 0 {
                    return Err(ErrorObject::borrowed(
                        ErrorCode::InvalidParams.code(),
                        "Provided negative index ends up with a negative height",
                        None,
                    ));
                }
                let h: u32 = h.try_into().expect("Positive i32 always fits in u32");
                if h > tip_height.0 {
                    return Err(ErrorObject::borrowed(
                        ErrorCode::InvalidParams.code(),
                        "Provided index is greater than the current tip",
                        None,
                    ));
                }

                h
            }
        };

        Ok(Height(sanitized_height))
    }
}

/// A helper module to serialize and deserialize `Option<T: ToHex>` as a hex string.
pub mod opthex {
    use hex::{FromHex, ToHex};
    use serde::{de, Deserialize, Deserializer, Serializer};

    #[allow(missing_docs)]
    pub fn serialize<S, T>(data: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: ToHex,
    {
        match data {
            Some(data) => {
                let s = data.encode_hex::<String>();
                serializer.serialize_str(&s)
            }
            None => serializer.serialize_none(),
        }
    }

    #[allow(missing_docs)]
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromHex,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(s) => T::from_hex(&s)
                .map(Some)
                .map_err(|_e| de::Error::custom("failed to convert hex string")),
            None => Ok(None),
        }
    }
}

/// A helper module to serialize and deserialize `[u8; N]` as a hex string.
pub mod arrayhex {
    use serde::{Deserializer, Serializer};
    use std::fmt;

    #[allow(missing_docs)]
    pub fn serialize<S, const N: usize>(data: &[u8; N], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string = hex::encode(data);
        serializer.serialize_str(&hex_string)
    }

    #[allow(missing_docs)]
    pub fn deserialize<'de, D, const N: usize>(deserializer: D) -> Result<[u8; N], D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HexArrayVisitor<const N: usize>;

        impl<const N: usize> serde::de::Visitor<'_> for HexArrayVisitor<N> {
            type Value = [u8; N];

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a hex string representing exactly {N} bytes")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let vec = hex::decode(v).map_err(E::custom)?;
                vec.clone().try_into().map_err(|_| {
                    E::invalid_length(vec.len(), &format!("expected {N} bytes").as_str())
                })
            }
        }

        deserializer.deserialize_str(HexArrayVisitor::<N>)
    }
}

/// Returns the proof-of-work difficulty as a multiple of the minimum difficulty.
pub async fn chain_tip_difficulty<State>(
    network: Network,
    mut state: State,
    should_use_default: bool,
) -> Result<f64>
where
    State: Service<
            zebra_state::ReadRequest,
            Response = zebra_state::ReadResponse,
            Error = zebra_state::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    State::Future: Send,
{
    let request = ReadRequest::ChainInfo;

    // # TODO
    // - add a separate request like BestChainNextMedianTimePast, but skipping the
    //   consistency check, because any block's difficulty is ok for display
    // - return 1.0 for a "not enough blocks in the state" error, like `zcashd`:
    // <https://github.com/zcash/zcash/blob/7b28054e8b46eb46a9589d0bdc8e29f9fa1dc82d/src/rpc/blockchain.cpp#L40-L41>
    let response = state
        .ready()
        .and_then(|service| service.call(request))
        .await;

    let response = match (should_use_default, response) {
        (_, Ok(res)) => res,
        (true, Err(_)) => {
            return Ok((U256::from(network.target_difficulty_limit()) >> 128).as_u128() as f64);
        }
        (false, Err(error)) => return Err(ErrorObject::owned(0, error.to_string(), None::<()>)),
    };

    let chain_info = match response {
        ReadResponse::ChainInfo(info) => info,
        _ => unreachable!("unmatched response to a chain info request"),
    };

    // This RPC is typically used for display purposes, so it is not consensus-critical.
    // But it uses the difficulty consensus rules for its calculations.
    //
    // Consensus:
    // https://zips.z.cash/protocol/protocol.pdf#nbits
    //
    // The zcashd implementation performs to_expanded() on f64,
    // and then does an inverse division:
    // https://github.com/zcash/zcash/blob/d6e2fada844373a8554ee085418e68de4b593a6c/src/rpc/blockchain.cpp#L46-L73
    //
    // But in Zebra we divide the high 128 bits of each expanded difficulty. This gives
    // a similar result, because the lower 128 bits are insignificant after conversion
    // to `f64` with a 53-bit mantissa.
    //
    // `pow_limit >> 128 / difficulty >> 128` is the same as the work calculation
    // `(2^256 / pow_limit) / (2^256 / difficulty)`, but it's a bit more accurate.
    //
    // To simplify the calculation, we don't scale for leading zeroes. (Bitcoin's
    // difficulty currently uses 68 bits, so even it would still have full precision
    // using this calculation.)

    // Get expanded difficulties (256 bits), these are the inverse of the work
    let pow_limit: U256 = network.target_difficulty_limit().into();
    let Some(difficulty) = chain_info.expected_difficulty.to_expanded() else {
        return Ok(0.0);
    };

    // Shift out the lower 128 bits (256 bits, but the top 128 are all zeroes)
    let pow_limit = pow_limit >> 128;
    let difficulty = U256::from(difficulty) >> 128;

    // Convert to u128 then f64.
    // We could also convert U256 to String, then parse as f64, but that's slower.
    let pow_limit = pow_limit.as_u128() as f64;
    let difficulty = difficulty.as_u128() as f64;

    // Invert the division to give approximately: `work(difficulty) / work(pow_limit)`
    Ok(pow_limit / difficulty)
}

/// Commands for the `addnode` RPC method.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AddNodeCommand {
    /// Add a node to the address book.
    #[serde(rename = "add")]
    Add,
}
