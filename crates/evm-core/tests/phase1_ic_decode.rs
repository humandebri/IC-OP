//! どこで: Phase1テスト / 何を: IcSyntheticデコード / なぜ: 仕様固定のため

use evm_core::tx_decode::{decode_ic_synthetic, DecodeError};
use evm_db::chain_data::constants::MAX_TX_SIZE;
use revm::primitives::{address, U256};

#[test]
fn decode_ic_tx_roundtrip() {
    let caller = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let bytes = build_ic_tx(7, vec![1, 2, 3]);

    let tx = decode_ic_synthetic(caller, &bytes).expect("decode");
    assert_eq!(tx.caller, caller);
    assert_eq!(tx.value, U256::from_be_bytes([0x22u8; 32]));
    assert_eq!(tx.gas_limit, 21_000);
    assert_eq!(tx.nonce, 7);
    assert_eq!(tx.data.as_ref(), [1u8, 2, 3]);
    assert_eq!(tx.gas_price, 30);
    assert_eq!(tx.gas_priority_fee, Some(2));
}

#[test]
fn decode_ic_tx_rejects_version() {
    let caller = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let mut bytes = vec![0u8; 1 + 20 + 32 + 8 + 8 + 16 + 16 + 4];
    bytes[0] = 1;
    let err = decode_ic_synthetic(caller, &bytes).err();
    assert_eq!(err, Some(DecodeError::InvalidVersion));
}

#[test]
fn decode_ic_tx_rejects_short_header() {
    let caller = address!("0xcccccccccccccccccccccccccccccccccccccccc");
    let bytes = vec![2u8; 10];
    let err = decode_ic_synthetic(caller, &bytes).err();
    assert_eq!(err, Some(DecodeError::InvalidLength));
}

#[test]
fn decode_ic_tx_rejects_data_length_mismatch() {
    let caller = address!("0xdddddddddddddddddddddddddddddddddddddddd");
    let mut bytes = build_ic_tx(1, vec![0xaa, 0xbb, 0xcc]);
    let len_pos = 1 + 20 + 32 + 8 + 8 + 16 + 16;
    bytes[len_pos..len_pos + 4].copy_from_slice(&5u32.to_be_bytes());
    let err = decode_ic_synthetic(caller, &bytes).err();
    assert_eq!(err, Some(DecodeError::InvalidLength));
}

#[test]
fn decode_ic_tx_rejects_oversized_data() {
    let caller = address!("0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    let bytes = build_ic_tx(2, vec![0u8; MAX_TX_SIZE.saturating_add(1)]);
    let err = decode_ic_synthetic(caller, &bytes).err();
    assert_eq!(err, Some(DecodeError::DataTooLarge));
}

fn build_ic_tx(nonce: u64, data: Vec<u8>) -> Vec<u8> {
    let to = [0x11u8; 20];
    let value = [0x22u8; 32];
    let gas = 21_000u64.to_be_bytes();
    let nonce = nonce.to_be_bytes();
    let max_fee = 30u128.to_be_bytes();
    let max_priority = 2u128.to_be_bytes();
    let data_len = (data.len() as u32).to_be_bytes();

    let mut bytes = Vec::new();
    bytes.push(2u8);
    bytes.extend_from_slice(&to);
    bytes.extend_from_slice(&value);
    bytes.extend_from_slice(&gas);
    bytes.extend_from_slice(&nonce);
    bytes.extend_from_slice(&max_fee);
    bytes.extend_from_slice(&max_priority);
    bytes.extend_from_slice(&data_len);
    bytes.extend_from_slice(&data);
    bytes
}
