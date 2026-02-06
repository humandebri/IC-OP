//! どこで: wrapper運用観測の補助セル / 何を: 実行警告メトリクスを保持 / なぜ: OpsStateの固定サイズを壊さないため

use crate::chain_data::codec::{encode_guarded, mark_decode_failure};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use std::borrow::Cow;
use zerocopy::byteorder::big_endian::U64;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

pub const OPS_METRICS_SIZE_U32: u32 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpsMetricsV1 {
    pub schema_version: u8,
    pub exec_halt_unknown_count: u64,
    pub last_exec_halt_unknown_warn_ts: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct OpsMetricsWire {
    schema_version: u8,
    _pad0: [u8; 7],
    exec_halt_unknown_count: U64,
    last_exec_halt_unknown_warn_ts: U64,
}

impl OpsMetricsWire {
    fn new(metrics: &OpsMetricsV1) -> Self {
        Self {
            schema_version: metrics.schema_version,
            _pad0: [0u8; 7],
            exec_halt_unknown_count: U64::new(metrics.exec_halt_unknown_count),
            last_exec_halt_unknown_warn_ts: U64::new(metrics.last_exec_halt_unknown_warn_ts),
        }
    }
}

impl OpsMetricsV1 {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            exec_halt_unknown_count: 0,
            last_exec_halt_unknown_warn_ts: 0,
        }
    }
}

impl Default for OpsMetricsV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Storable for OpsMetricsV1 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let wire = OpsMetricsWire::new(self);
        match encode_guarded(
            b"ops_metrics",
            Cow::Owned(wire.as_bytes().to_vec()),
            OPS_METRICS_SIZE_U32,
        ) {
            Ok(value) => value,
            Err(_) => Cow::Owned(vec![0u8; OPS_METRICS_SIZE_U32 as usize]),
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let data = bytes.as_ref();
        if data.len() != OPS_METRICS_SIZE_U32 as usize && data.len() != 40 {
            mark_decode_failure(b"ops_metrics", false);
            return Self::new();
        }
        if data.len() == 40 {
            let schema_version = data[0];
            let mut count_bytes = [0u8; 8];
            let mut ts_bytes = [0u8; 8];
            count_bytes.copy_from_slice(&data[8..16]);
            ts_bytes.copy_from_slice(&data[16..24]);
            return Self {
                schema_version,
                exec_halt_unknown_count: u64::from_be_bytes(count_bytes),
                last_exec_halt_unknown_warn_ts: u64::from_be_bytes(ts_bytes),
            };
        }
        let wire = match OpsMetricsWire::read_from_bytes(data) {
            Ok(value) => value,
            Err(_) => {
                mark_decode_failure(b"ops_metrics", false);
                return Self::new();
            }
        };
        Self {
            schema_version: wire.schema_version,
            exec_halt_unknown_count: wire.exec_halt_unknown_count.get(),
            last_exec_halt_unknown_warn_ts: wire.last_exec_halt_unknown_warn_ts.get(),
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: OPS_METRICS_SIZE_U32,
        is_fixed_size: true,
    };
}
