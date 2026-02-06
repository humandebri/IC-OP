//! どこで: dropped tx の保持管理 / 何を: 固定長リング状態を保持 / なぜ: tx_locs の無限増加を防ぐため

use crate::chain_data::codec::{encode_guarded, mark_decode_failure};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use std::borrow::Cow;
use zerocopy::byteorder::big_endian::{U32, U64};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

pub const DROPPED_RING_STATE_SIZE_U32: u32 = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DroppedRingStateV1 {
    pub schema_version: u32,
    pub next_seq: u64,
    pub len: u32,
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned,
)]
#[repr(C)]
struct DroppedRingStateWire {
    schema_version: U32,
    next_seq: U64,
    len: U32,
}

impl DroppedRingStateWire {
    fn new(schema_version: u32, next_seq: u64, len: u32) -> Self {
        Self {
            schema_version: U32::new(schema_version),
            next_seq: U64::new(next_seq),
            len: U32::new(len),
        }
    }
}

impl DroppedRingStateV1 {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            next_seq: 0,
            len: 0,
        }
    }
}

impl Default for DroppedRingStateV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Storable for DroppedRingStateV1 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let wire = DroppedRingStateWire::new(self.schema_version, self.next_seq, self.len);
        match encode_guarded(
            b"dropped_ring_state",
            Cow::Owned(wire.as_bytes().to_vec()),
            DROPPED_RING_STATE_SIZE_U32,
        ) {
            Ok(value) => value,
            Err(_) => Cow::Owned(vec![0u8; DROPPED_RING_STATE_SIZE_U32 as usize]),
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        let wire = DroppedRingStateWire::new(self.schema_version, self.next_seq, self.len);
        wire.as_bytes().to_vec()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let data = bytes.as_ref();
        if data.len() != 16 {
            mark_decode_failure(b"dropped_ring_state", false);
            return DroppedRingStateV1::new();
        }
        let wire = match DroppedRingStateWire::read_from_bytes(data) {
            Ok(value) => value,
            Err(_) => {
                mark_decode_failure(b"dropped_ring_state", false);
                return DroppedRingStateV1::new();
            }
        };
        Self {
            schema_version: wire.schema_version.get(),
            next_seq: wire.next_seq.get(),
            len: wire.len.get(),
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: DROPPED_RING_STATE_SIZE_U32,
        is_fixed_size: true,
    };
}
