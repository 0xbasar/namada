use std::fmt::Debug;

use namada_core::ledger::storage::{DBIter, StorageHasher, WlStorage, DB};
use namada_core::ledger::storage_api;
use namada_core::types::storage::BlockHeight;
use thiserror::Error;

use crate::events::log::EventLog;
use crate::tendermint::merkle::proof::ProofOps;
/// A request context provides read-only access to storage and WASM compilation
/// caches to request handlers.
#[derive(Debug, Clone)]
pub struct RequestCtx<'shell, D, H, VpCache, TxCache>
where
    D: 'static + DB + for<'iter> DBIter<'iter> + Sync,
    H: 'static + StorageHasher + Sync,
{
    /// Reference to the ledger's [`WlStorage`].
    pub wl_storage: &'shell WlStorage<D, H>,
    /// Log of events emitted by `FinalizeBlock` ABCI calls.
    pub event_log: &'shell EventLog,
    /// Cache of VP wasm compiled artifacts.
    pub vp_wasm_cache: VpCache,
    /// Cache of transaction wasm compiled artifacts.
    pub tx_wasm_cache: TxCache,
    /// Taken from config `storage_read_past_height_limit`. When set, will
    /// limit the how many block heights in the past can the storage be
    /// queried for reading values.
    pub storage_read_past_height_limit: Option<u64>,
}

/// A `Router` handles parsing read-only query requests and dispatching them to
/// their handler functions. A valid query returns a borsh-encoded result.
pub trait Router {
    /// Handle a given request using the provided context. This must be invoked
    /// on the root `Router` to be able to match the `request.path` fully.
    fn handle<D, H, V, T>(
        &self,
        ctx: RequestCtx<'_, D, H, V, T>,
        request: &RequestQuery,
    ) -> storage_api::Result<EncodedResponseQuery>
    where
        D: 'static + DB + for<'iter> DBIter<'iter> + Sync,
        H: 'static + StorageHasher + Sync,
    {
        self.internal_handle(ctx, request, 0)
    }

    /// Internal method which shouldn't be invoked directly. Instead, you may
    /// want to call `self.handle()`.
    ///
    /// Handle a given request using the provided context, starting to
    /// try to match `request.path` against the `Router`'s patterns at the
    /// given `start` offset.
    fn internal_handle<D, H, V, T>(
        &self,
        ctx: RequestCtx<'_, D, H, V, T>,
        request: &RequestQuery,
        start: usize,
    ) -> storage_api::Result<EncodedResponseQuery>
    where
        D: 'static + DB + for<'iter> DBIter<'iter> + Sync,
        H: 'static + StorageHasher + Sync;
}

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Tendermint(#[from] tendermint_rpc::Error),
    #[error("Decoding error: {0}")]
    Decoding(#[from] std::io::Error),
    #[error("Info log: {0}, error code: {1}")]
    Query(String, u32),
    #[error("Invalid block height: {0} (overflown i64)")]
    InvalidHeight(BlockHeight),
}

/// Temporary domain-type for `tendermint_proto::abci::RequestQuery`, copied
/// from <https://github.com/informalsystems/tendermint-rs/pull/862>
/// until we are on a branch that has it included.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RequestQuery {
    /// Raw query bytes.
    ///
    /// Can be used with or in lieu of `path`.
    pub data: Vec<u8>,
    /// Path of the request, like an HTTP `GET` path.
    ///
    /// Can be used with or in lieu of `data`.
    ///
    /// Applications MUST interpret `/store` as a query by key on the
    /// underlying store. The key SHOULD be specified in the Data field.
    /// Applications SHOULD allow queries over specific types like
    /// `/accounts/...` or `/votes/...`.
    pub path: String,
    /// The block height for which the query should be executed.
    ///
    /// The default `0` returns data for the latest committed block. Note that
    /// this is the height of the block containing the application's Merkle
    /// root hash, which represents the state as it was after committing
    /// the block at `height - 1`.
    pub height: BlockHeight,
    /// Whether to return a Merkle proof with the response, if possible.
    pub prove: bool,
}

/// Generic response from a query
#[derive(Clone, Debug, Default)]
pub struct ResponseQuery<T> {
    /// Response data to be borsh encoded
    pub data: T,
    /// Non-deterministic log of the request execution
    pub info: String,
    /// Optional proof - used for storage value reads which request `prove`
    pub proof: Option<ProofOps>,
}

/// [`ResponseQuery`] with borsh-encoded `data` field
pub type EncodedResponseQuery = ResponseQuery<Vec<u8>>;

impl RequestQuery {
    /// Try to convert tendermint RequestQuery into our [`RequestQuery`]
    /// domain type. This tries to convert the block height into our
    /// [`BlockHeight`] type, where `0` is treated as a special value to signal
    /// to use the latest committed block height as per tendermint ABCI Query
    /// spec. A negative block height will cause an error.
    pub fn try_from_tm<D, H>(
        storage: &WlStorage<D, H>,
        crate::tendermint_proto::v0_37::abci::RequestQuery {
            data,
            path,
            height,
            prove,
        }: crate::tendermint_proto::v0_37::abci::RequestQuery,
    ) -> Result<Self, String>
    where
        D: DB + for<'iter> DBIter<'iter>,
        H: StorageHasher,
    {
        let height = match height {
            0 => {
                // `0` means last committed height
                storage.storage.get_last_block_height()
            }
            _ => BlockHeight(height.try_into().map_err(|_| {
                format!("Query height cannot be negative, got: {}", height)
            })?),
        };
        Ok(Self {
            data: data.to_vec(),
            path,
            height,
            prove,
        })
    }
}
