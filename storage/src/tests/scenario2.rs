use super::tools::*;
use crate::start_storage;
use std::collections::HashMap;

#[tokio::test]
async fn test_add() {
    let (cfg, serialization_context) = get_test_config();
    let (command_sender, manager) = start_storage(cfg, serialization_context).unwrap();
    assert_eq!(0, command_sender.len().await.unwrap());
    let hash = get_test_block_id();
    let block = get_test_block();
    command_sender.add_block(hash, block).await.unwrap();
    assert!(command_sender.contains(hash).await.unwrap());
    assert_eq!(1, command_sender.len().await.unwrap());
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_find_operation() {
    let (cfg, serialization_context) = get_test_config();
    let (command_sender, manager) = start_storage(cfg, serialization_context.clone()).unwrap();
    assert_eq!(0, command_sender.len().await.unwrap());
    let (block, id, op) = get_block_with_op(&serialization_context);
    command_sender.add_block(id, block).await.unwrap();
    let (out_id, out_idx, _out_op) = command_sender.get_operation(op).await.unwrap().unwrap();
    assert_eq!((out_id, out_idx), (id, 0));
    let mut op2 = create_operation(&serialization_context);
    op2.content.fee = 42;
    let id2 = op2.get_operation_id(&serialization_context).unwrap();
    assert!(command_sender.get_operation(id2).await.unwrap().is_none());
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_add_multiple() {
    let (cfg, serialization_context) = get_test_config();
    let (command_sender, manager) = start_storage(cfg, serialization_context).unwrap();
    let hash = get_test_block_id();
    let block = get_test_block();
    let mut map = HashMap::new();
    map.insert(hash, block);
    command_sender.add_block_batch(map).await.unwrap();
    assert!(command_sender.contains(hash).await.unwrap());
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_get() {
    // stderrlog::new()
    //     .verbosity(2)
    //     .timestamp(stderrlog::Timestamp::Millisecond)
    //     .init()
    //     .unwrap();
    let (cfg, serialization_context) = get_test_config();
    let (command_sender, manager) = start_storage(cfg, serialization_context.clone()).unwrap();
    assert_eq!(0, command_sender.len().await.unwrap());
    let hash = get_test_block_id();
    let block = get_test_block();
    command_sender.add_block(hash, block.clone()).await.unwrap();
    let retrived = command_sender.get_block(hash).await.unwrap().unwrap();

    assert_eq!(
        retrived
            .header
            .content
            .compute_hash(&serialization_context)
            .unwrap(),
        block
            .header
            .content
            .compute_hash(&serialization_context)
            .unwrap()
    );

    assert!(command_sender
        .get_block(get_another_test_block_id())
        .await
        .unwrap()
        .is_none());
    //command_sender.clear().await.unwrap();
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_contains() {
    let (cfg, serialization_context) = get_test_config();
    let (command_sender, manager) = start_storage(cfg, serialization_context).unwrap();
    //test in an empty db that the contains return false.
    assert!(!command_sender
        .contains(get_another_test_block_id())
        .await
        .unwrap());

    assert_eq!(0, command_sender.len().await.unwrap());
    let hash = get_test_block_id();
    let block = get_test_block();
    command_sender.add_block(hash, block.clone()).await.unwrap();

    //test the block is predent in db
    assert!(command_sender.contains(hash).await.unwrap());

    //test that another block isn't present
    assert!(!command_sender
        .contains(get_another_test_block_id())
        .await
        .unwrap());

    manager.stop().await.unwrap();
}
