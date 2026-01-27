//! どこで: chain_data のヘッダ状態 / 何を: 固定サイズのStableStateV1 / なぜ: upgrade耐性と最小メタ保持のため

use crate::chain_data::constants::CHAIN_STATE_SIZE_U32;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChainStateV1 {
    pub schema_version: u32,
    pub chain_id: u64,
    pub last_block_number: u64,
    pub last_block_time: u64,
    pub auto_mine_enabled: bool,
    pub is_producing: bool,
    pub mining_scheduled: bool,
    pub next_queue_seq: u64,
}

impl ChainStateV1 {
    pub fn new(chain_id: u64) -> Self {
        Self {
            schema_version: 1,
            chain_id,
            last_block_number: 0,
            last_block_time: 0,
            auto_mine_enabled: false,
            is_producing: false,
            mining_scheduled: false,
            next_queue_seq: 0,
        }
    }

    fn flags(&self) -> u8 {
        let mut out = 0u8;
        if self.auto_mine_enabled {
            out |= 1 << 0;
        }
        if self.is_producing {
            out |= 1 << 1;
        }
        if self.mining_scheduled {
            out |= 1 << 2;
        }
        out
    }

    fn apply_flags(&mut self, flags: u8) {
        self.auto_mine_enabled = (flags & (1 << 0)) != 0;
        self.is_producing = (flags & (1 << 1)) != 0;
        self.mining_scheduled = (flags & (1 << 2)) != 0;
    }
}

impl Storable for ChainStateV1 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut out = [0u8; 40];
        out[0..4].copy_from_slice(&self.schema_version.to_be_bytes());
        out[4..12].copy_from_slice(&self.chain_id.to_be_bytes());
        out[12..20].copy_from_slice(&self.last_block_number.to_be_bytes());
        out[20..28].copy_from_slice(&self.last_block_time.to_be_bytes());
        out[28] = self.flags();
        out[32..40].copy_from_slice(&self.next_queue_seq.to_be_bytes());
        Cow::Owned(out.to_vec())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let data = bytes.as_ref();
        if data.len() != 40 {
            ic_cdk::trap("chain_state: invalid length");
        }
        let mut schema = [0u8; 4];
        schema.copy_from_slice(&data[0..4]);
        let mut chain_id = [0u8; 8];
        chain_id.copy_from_slice(&data[4..12]);
        let mut last_number = [0u8; 8];
        last_number.copy_from_slice(&data[12..20]);
        let mut last_time = [0u8; 8];
        last_time.copy_from_slice(&data[20..28]);
        let flags = data[28];
        let mut next_queue_seq = [0u8; 8];
        next_queue_seq.copy_from_slice(&data[32..40]);
        let mut state = Self {
            schema_version: u32::from_be_bytes(schema),
            chain_id: u64::from_be_bytes(chain_id),
            last_block_number: u64::from_be_bytes(last_number),
            last_block_time: u64::from_be_bytes(last_time),
            auto_mine_enabled: false,
            is_producing: false,
            mining_scheduled: false,
            next_queue_seq: u64::from_be_bytes(next_queue_seq),
        };
        state.apply_flags(flags);
        state
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: CHAIN_STATE_SIZE_U32,
        is_fixed_size: true,
    };
}
