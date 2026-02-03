//! どこで: Phase1のbase_fee更新 / 何を: EIP-1559更新式をalloyへ委譲 / なぜ: 参照実装との差分を減らすため

use alloy_eips::eip1559::{calc_next_block_base_fee, BaseFeeParams};

pub fn compute_next_base_fee(base_fee: u64, gas_used: u64, block_gas_limit: u64) -> u64 {
    let params = BaseFeeParams::ethereum();
    let elasticity = params.elasticity_multiplier as u64;
    let gas_target = block_gas_limit / elasticity;
    if gas_target == 0 {
        return base_fee;
    }
    calc_next_block_base_fee(gas_used, block_gas_limit, base_fee, params)
}

#[cfg(test)]
mod tests {
    use super::compute_next_base_fee;

    #[test]
    fn base_fee_updates_up_down_and_flat() {
        let base_fee = 100u64;
        let block_gas_limit = 8u64;

        let same = compute_next_base_fee(base_fee, 4, block_gas_limit);
        assert_eq!(same, 100);

        let up = compute_next_base_fee(base_fee, 8, block_gas_limit);
        assert_eq!(up, 112);

        let down = compute_next_base_fee(base_fee, 0, block_gas_limit);
        assert_eq!(down, 88);
    }
}
