//! どこで: Phase1テスト / 何を: produce_block の drop_code / なぜ: 失敗理由の可視化を固定するため

use evm_core::chain::{self, ChainError};
use evm_db::chain_data::constants::DROP_CODE_DECODE;
use evm_db::chain_data::TxLocKind;
use evm_db::stable_state::init_stable_state;

#[test]
fn produce_block_marks_decode_drop() {
    init_stable_state();

    let bad_tx = vec![0x01];
    let tx_id = chain::submit_tx(evm_db::chain_data::TxKind::EthSigned, bad_tx)
        .expect("submit");

    let err = chain::produce_block(1).expect_err("produce_block should fail");
    assert_eq!(err, ChainError::NoExecutableTx);

    let loc = chain::get_tx_loc(&tx_id).expect("tx_loc");
    assert_eq!(loc.kind, TxLocKind::Dropped);
    assert_eq!(loc.drop_code, DROP_CODE_DECODE);
}
