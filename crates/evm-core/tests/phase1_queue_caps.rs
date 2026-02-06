//! どこで: Phase1テスト / 何を: mempoolのglobal cap拒否 / なぜ: 無限投入DoSを防ぐため

use evm_core::chain::{self, ChainError};
use evm_db::chain_data::constants::{DROP_CODE_REPLACED, MAX_PENDING_GLOBAL};
use evm_db::chain_data::{SenderNonceKey, TxId, TxLocKind};
use evm_db::stable_state::{init_stable_state, with_state_mut};

#[test]
fn submit_ic_tx_rejects_when_global_pending_cap_is_reached() {
    init_stable_state();
    with_state_mut(|state| {
        for i in 0..MAX_PENDING_GLOBAL {
            let mut sender = [0u8; 20];
            sender[18] = ((i >> 8) & 0xff) as u8;
            sender[19] = (i & 0xff) as u8;
            let key = SenderNonceKey::new(sender, 0);
            let mut tx_id = [0u8; 32];
            tx_id[28] = ((i >> 24) & 0xff) as u8;
            tx_id[29] = ((i >> 16) & 0xff) as u8;
            tx_id[30] = ((i >> 8) & 0xff) as u8;
            tx_id[31] = (i & 0xff) as u8;
            state.pending_by_sender_nonce.insert(key, TxId(tx_id));
        }
    });

    let err = chain::submit_ic_tx(
        vec![0x01],
        vec![0x02],
        build_ic_tx_bytes(0, 2_000_000_000, 1_000_000_000),
    )
    .expect_err("global cap should reject submit");
    assert_eq!(err, ChainError::QueueFull);
}

#[test]
fn replacement_is_allowed_even_when_global_pending_cap_is_reached() {
    init_stable_state();
    let caller = vec![0x42];
    let canister = vec![0x77];
    let first_tx = build_ic_tx_bytes(0, 2_000_000_000, 1_000_000_000);
    let first_tx_id =
        chain::submit_ic_tx(caller.clone(), canister.clone(), first_tx).expect("first submit");

    with_state_mut(|state| {
        // Keep the original sender entry, then fill up to the global cap with distinct senders.
        for i in 1..MAX_PENDING_GLOBAL {
            let mut sender = [0u8; 20];
            sender[18] = ((i >> 8) & 0xff) as u8;
            sender[19] = (i & 0xff) as u8;
            let key = SenderNonceKey::new(sender, 0);
            let mut tx_id = [0u8; 32];
            tx_id[28] = ((i >> 24) & 0xff) as u8;
            tx_id[29] = ((i >> 16) & 0xff) as u8;
            tx_id[30] = ((i >> 8) & 0xff) as u8;
            tx_id[31] = (i & 0xff) as u8;
            state.pending_by_sender_nonce.insert(key, TxId(tx_id));
        }
    });

    let replacement_tx = build_ic_tx_bytes(0, 3_000_000_000, 2_000_000_000);
    let replacement_tx_id = chain::submit_ic_tx(caller, canister, replacement_tx)
        .expect("replacement should be accepted");
    assert_ne!(replacement_tx_id, first_tx_id);
    let old_loc = chain::get_tx_loc(&first_tx_id).expect("old tx loc");
    assert_eq!(old_loc.kind, TxLocKind::Dropped);
    assert_eq!(old_loc.drop_code, DROP_CODE_REPLACED);
}

fn build_ic_tx_bytes(nonce: u64, max_fee_per_gas: u128, max_priority_fee_per_gas: u128) -> Vec<u8> {
    let to = [0x10u8; 20];
    let value = [0u8; 32];
    let gas_limit = 50_000u64.to_be_bytes();
    let nonce = nonce.to_be_bytes();
    let max_fee = max_fee_per_gas.to_be_bytes();
    let max_priority = max_priority_fee_per_gas.to_be_bytes();
    let data: Vec<u8> = Vec::new();
    let data_len = 0u32.to_be_bytes();
    let mut out = Vec::new();
    out.push(2u8);
    out.extend_from_slice(&to);
    out.extend_from_slice(&value);
    out.extend_from_slice(&gas_limit);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&max_fee);
    out.extend_from_slice(&max_priority);
    out.extend_from_slice(&data_len);
    out.extend_from_slice(&data);
    out
}
