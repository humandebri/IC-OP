//! どこで: UPGRADES領域 / 何を: pre/post upgradeの最小退避 / なぜ: Phase0の基本運用

use crate::memory::{get_memory, AppMemoryId, VMem};
use ic_stable_structures::reader::Reader;
use ic_stable_structures::writer::Writer;
use ic_stable_structures::Memory;

const UPGRADE_STATE_VERSION: u32 = 1;

pub fn pre_upgrade() {
    let mut memory: VMem = get_memory(AppMemoryId::Upgrades);
    let mut writer = Writer::new(&mut memory, 0);
    let version_bytes = UPGRADE_STATE_VERSION.to_le_bytes();
    writer
        .write(&version_bytes)
        .unwrap_or_else(|_| ic_cdk::trap("upgrade: write version failed"));
}

pub fn post_upgrade() {
    let memory: VMem = get_memory(AppMemoryId::Upgrades);
    let version = read_version(&memory)
        .unwrap_or_else(|| ic_cdk::trap("upgrade: missing state"));
    if version != UPGRADE_STATE_VERSION {
        ic_cdk::trap("upgrade: version mismatch");
    }
}

fn read_version(memory: &VMem) -> Option<u32> {
    if memory.size() == 0 {
        return None;
    }
    let mut reader = Reader::new(memory, 0);
    let mut buf = [0u8; 4];
    let read = reader
        .read(&mut buf)
        .unwrap_or_else(|_| ic_cdk::trap("upgrade: read failed"));
    if read != 4 {
        return None;
    }
    Some(u32::from_le_bytes(buf))
}
