//! どこで: state rootコミット境界 / 何を: prepare/apply/migrationの単一入口化 / なぜ: 経路統一で整合性を守るため

use crate::revm_exec::StateDiff;
use crate::state_root::{
    apply_state_root_commit, prepare_state_root_commit, run_migration_tick, PreparedStateRoot,
    TouchedSummary,
};
use evm_db::stable_state::StableState;

pub(crate) fn prepare(
    state: &mut StableState,
    state_diffs: &[StateDiff],
    touched_addrs: &[[u8; 20]],
    touched: TouchedSummary,
    block_number: u64,
    parent_hash: [u8; 32],
    timestamp: u64,
) -> Result<PreparedStateRoot, &'static str> {
    prepare_state_root_commit(
        state,
        state_diffs,
        touched_addrs,
        touched,
        block_number,
        parent_hash,
        timestamp,
    )
}

pub(crate) fn apply(state: &mut StableState, prepared: PreparedStateRoot) {
    apply_state_root_commit(state, prepared);
}

pub(crate) fn migration_tick(state: &mut StableState, max_steps: u32) -> bool {
    run_migration_tick(state, max_steps)
}
