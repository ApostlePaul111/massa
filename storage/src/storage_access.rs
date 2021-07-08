use crate::{
    block_storage::{BlockStorage, StorageCleaner},
    config::StorageConfig,
    error::StorageError,
};
use logging::debug;
use models::{Block, BlockId, Operation, OperationId, SerializationContext, Slot};
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;

pub fn start_storage(
    cfg: StorageConfig,
    serialization_context: SerializationContext,
) -> Result<(StorageAccess, StorageManager), StorageError> {
    debug!("starting storage controller");
    let sled_config = sled::Config::default()
        .path(&cfg.path)
        .cache_capacity(cfg.cache_capacity)
        .flush_every_ms(cfg.flush_interval.map(|v| v.to_millis()));
    let db = sled_config.open()?;

    if cfg.reset_at_startup {
        db.drop_tree("hash_to_block")?;
        db.drop_tree("slot_to_hash")?;
        db.drop_tree("op_to_block")?;
    }
    let hash_to_block = db.open_tree("hash_to_block")?;
    let slot_to_hash = db.open_tree("slot_to_hash")?;
    let op_to_block = db.open_tree("op_to_block")?;

    let block_count = Arc::new(AtomicUsize::new(hash_to_block.len()));
    let notify = Arc::new(Notify::new());
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
    let db = BlockStorage::open(
        cfg.clone(),
        serialization_context.clone(),
        hash_to_block.clone(),
        slot_to_hash.clone(),
        op_to_block.clone(),
        block_count.clone(),
        notify.clone(),
    )?;

    let storage_cleaner = StorageCleaner::new(
        cfg.max_stored_blocks,
        notify.clone(),
        shutdown_rx,
        hash_to_block,
        slot_to_hash,
        op_to_block,
        block_count,
        serialization_context,
    )?;
    let join_handle = tokio::spawn(async move {
        let res = storage_cleaner.run_loop().await;
        match res {
            Err(err) => {
                error!("Storage cleaner crashed: {:?}", err);
                Err(err)
            }
            Ok(v) => {
                info!("Storage cleaner finished cleanly");
                Ok(v)
            }
        }
    });
    Ok((
        StorageAccess(db.clone()),
        StorageManager {
            shutdown_tx,
            join_handle,
        },
    ))
}

#[derive(Clone)]
pub struct StorageAccess(pub BlockStorage);

impl StorageAccess {
    pub async fn len(&self) -> Result<usize, StorageError> {
        self.0.len().await
    }
    pub async fn add_block(&self, hash: BlockId, block: Block) -> Result<(), StorageError> {
        self.0.add_block(hash, block).await
    }
    pub async fn add_block_batch(
        &self,
        blocks: HashMap<BlockId, Block>,
    ) -> Result<(), StorageError> {
        self.0.add_block_batch(blocks).await
    }

    pub async fn get_block(&self, hash: BlockId) -> Result<Option<Block>, StorageError> {
        self.0.get_block(hash)
    }

    pub async fn contains(&self, hash: BlockId) -> Result<bool, StorageError> {
        self.0.contains(hash).await
    }

    pub async fn get_slot_range(
        &self,
        start: Option<Slot>,
        end: Option<Slot>,
    ) -> Result<HashMap<BlockId, Block>, StorageError> {
        self.0.get_slot_range(start, end).await
    }

    /// returns Some(tuple) if found, or None if not found. Tuple:
    ///  * the BlockId in which the op is included
    ///  * its index in the block
    ///  * the operation itself
    pub async fn get_operation(
        &self,
        id: OperationId,
    ) -> Result<Option<(BlockId, usize, Operation)>, StorageError> {
        self.0.get_operation(id).await
    }
}

pub struct StorageManager {
    shutdown_tx: mpsc::Sender<()>,
    join_handle: JoinHandle<Result<(), StorageError>>,
}

impl StorageManager {
    pub async fn stop(self) -> Result<(), StorageError> {
        drop(self.shutdown_tx);
        self.join_handle.await?
    }
}
