//! どこで: Phase1テスト / 何を: OpDeposit wire decode検証 / なぜ: 失敗理由の安定化のため

use evm_core::chain::{self, ChainError, TxIn};
use evm_core::tx_decode::{decode_tx, DecodeError, DepositInvalidReason};
use evm_db::chain_data::TxKind;
use evm_db::stable_state::init_stable_state;
use revm::primitives::Address;

#[test]
fn op_deposit_wire_roundtrip_decode_succeeds() {
    let caller = Address::from([0u8; 20]);
    let bytes = build_wire(
        1,
        [0x11u8; 32],
        [0x22u8; 20],
        Some([0x33u8; 20]),
        [0u8; 32],
        [0x44u8; 32],
        100_000,
        false,
        vec![0xaa, 0xbb],
    );
    let tx = decode_tx(TxKind::OpDeposit, caller, &bytes).expect("decode op deposit");
    assert_eq!(tx.tx_type, 0x7e);
    assert_eq!(tx.nonce, 0);
}

#[test]
fn op_deposit_source_hash_zero_rejected() {
    let caller = Address::from([0u8; 20]);
    let bytes = build_wire(
        1,
        [0u8; 32],
        [0x22u8; 20],
        Some([0x33u8; 20]),
        [0u8; 32],
        [0x44u8; 32],
        100_000,
        false,
        Vec::new(),
    );
    let err = decode_tx(TxKind::OpDeposit, caller, &bytes).unwrap_err();
    assert_eq!(
        err,
        DecodeError::DepositInvalid(DepositInvalidReason::SourceHashZero)
    );
}

#[test]
fn op_deposit_bad_to_flag_rejected() {
    let caller = Address::from([0u8; 20]);
    let mut bytes = Vec::new();
    bytes.push(1);
    bytes.extend_from_slice(&[0x11u8; 32]);
    bytes.extend_from_slice(&[0x22u8; 20]);
    bytes.push(2); // invalid to_flag
    bytes.extend_from_slice(&[0u8; 32]); // mint
    bytes.extend_from_slice(&[0x44u8; 32]); // value
    bytes.extend_from_slice(&100_000u64.to_be_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&0u32.to_be_bytes());
    let err = decode_tx(TxKind::OpDeposit, caller, &bytes).unwrap_err();
    assert_eq!(
        err,
        DecodeError::DepositInvalid(DepositInvalidReason::BadToFlag)
    );
}

#[test]
fn op_deposit_length_mismatch_rejected() {
    let caller = Address::from([0u8; 20]);
    let mut bytes = build_wire(
        1,
        [0x11u8; 32],
        [0x22u8; 20],
        Some([0x33u8; 20]),
        [0u8; 32],
        [0x44u8; 32],
        100_000,
        false,
        vec![1, 2, 3],
    );
    let len_pos = 1 + 32 + 20 + 1 + 20 + 32 + 32 + 8 + 1;
    bytes[len_pos..len_pos + 4].copy_from_slice(&10u32.to_be_bytes());
    let err = decode_tx(TxKind::OpDeposit, caller, &bytes).unwrap_err();
    assert_eq!(
        err,
        DecodeError::DepositInvalid(DepositInvalidReason::LengthMismatch)
    );
}

#[test]
fn op_deposit_version_mismatch_rejected() {
    let caller = Address::from([0u8; 20]);
    let mut bytes = build_wire(
        1,
        [0x11u8; 32],
        [0x22u8; 20],
        Some([0x33u8; 20]),
        [0u8; 32],
        [0x44u8; 32],
        100_000,
        false,
        Vec::new(),
    );
    bytes[0] = 2;
    let err = decode_tx(TxKind::OpDeposit, caller, &bytes).unwrap_err();
    assert_eq!(
        err,
        DecodeError::DepositInvalid(DepositInvalidReason::VersionMismatch)
    );
}

#[test]
fn op_deposit_mint_too_large_rejected() {
    let caller = Address::from([0u8; 20]);
    let bytes = build_wire(
        1,
        [0x11u8; 32],
        [0x22u8; 20],
        Some([0x33u8; 20]),
        [0xffu8; 32],
        [0x44u8; 32],
        100_000,
        false,
        Vec::new(),
    );
    let err = decode_tx(TxKind::OpDeposit, caller, &bytes).unwrap_err();
    assert_eq!(
        err,
        DecodeError::DepositInvalid(DepositInvalidReason::MintTooLargeForOpRevm)
    );
}

#[test]
fn op_deposit_bad_is_system_flag_rejected() {
    let caller = Address::from([0u8; 20]);
    let mut bytes = build_wire(
        1,
        [0x11u8; 32],
        [0x22u8; 20],
        Some([0x33u8; 20]),
        [0u8; 32],
        [0x44u8; 32],
        100_000,
        false,
        Vec::new(),
    );
    let is_system_pos = 1 + 32 + 20 + 1 + 20 + 32 + 32 + 8;
    bytes[is_system_pos] = 2;
    let err = decode_tx(TxKind::OpDeposit, caller, &bytes).unwrap_err();
    assert_eq!(
        err,
        DecodeError::DepositInvalid(DepositInvalidReason::BadIsSystemFlag)
    );
}

#[test]
fn op_deposit_is_system_one_rejected() {
    let caller = Address::from([0u8; 20]);
    let mut bytes = build_wire(
        1,
        [0x11u8; 32],
        [0x22u8; 20],
        Some([0x33u8; 20]),
        [0u8; 32],
        [0x44u8; 32],
        100_000,
        false,
        Vec::new(),
    );
    let is_system_pos = 1 + 32 + 20 + 1 + 20 + 32 + 32 + 8;
    bytes[is_system_pos] = 1;
    let err = decode_tx(TxKind::OpDeposit, caller, &bytes).unwrap_err();
    assert_eq!(
        err,
        DecodeError::DepositInvalid(DepositInvalidReason::BadIsSystemFlag)
    );
}

#[test]
fn submit_tx_in_op_deposit_stays_unsupported() {
    init_stable_state();
    let err = chain::submit_tx_in(TxIn::OpDeposit(vec![1, 2, 3])).unwrap_err();
    assert_eq!(err, ChainError::UnsupportedTxKind);
}

fn build_wire(
    version: u8,
    source_hash: [u8; 32],
    from: [u8; 20],
    to: Option<[u8; 20]>,
    mint: [u8; 32],
    value: [u8; 32],
    gas_limit: u64,
    is_system: bool,
    data: Vec<u8>,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(version);
    out.extend_from_slice(&source_hash);
    out.extend_from_slice(&from);
    match to {
        Some(addr) => {
            out.push(1);
            out.extend_from_slice(&addr);
        }
        None => out.push(0),
    }
    out.extend_from_slice(&mint);
    out.extend_from_slice(&value);
    out.extend_from_slice(&gas_limit.to_be_bytes());
    out.push(if is_system { 1 } else { 0 });
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&data);
    out
}
