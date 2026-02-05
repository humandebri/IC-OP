//! どこで: state root 管理 / 何を: 差分更新のルート状態を保持 / なぜ: ブロック毎の全ステート再計算を避けるため

use crate::corrupt_log::record_corrupt;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use std::borrow::Cow;

pub const STATE_ROOT_META_SIZE_U32: u32 = 40;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateRootMetaV1 {
    pub schema_version: u32,
    pub initialized: bool,
    pub state_root: [u8; 32],
}

impl StateRootMetaV1 {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            initialized: false,
            state_root: [0u8; 32],
        }
    }
}

impl Default for StateRootMetaV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Storable for StateRootMetaV1 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut out = [0u8; 40];
        out[0..4].copy_from_slice(&self.schema_version.to_be_bytes());
        out[4] = if self.initialized { 1 } else { 0 };
        out[8..40].copy_from_slice(&self.state_root);
        Cow::Owned(out.to_vec())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let data = bytes.as_ref();
        if data.len() != 40 {
            record_corrupt(b"state_root_meta");
            return StateRootMetaV1::new();
        }
        let mut schema = [0u8; 4];
        schema.copy_from_slice(&data[0..4]);
        let initialized = data[4] == 1;
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(&data[8..40]);
        Self {
            schema_version: u32::from_be_bytes(schema),
            initialized,
            state_root,
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: STATE_ROOT_META_SIZE_U32,
        is_fixed_size: true,
    };
}
