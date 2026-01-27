//! どこで: Stable Memoryの割当 / 何を: MemoryIdの凍結とMemoryManager初期化 / なぜ: レイアウトを固定するため

use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::DefaultMemoryImpl;
use std::cell::RefCell;

pub type VMem = VirtualMemory<DefaultMemoryImpl>;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppMemoryId {
    Upgrades = 0,
    Meta = 1,
    Accounts = 2,
    Storage = 3,
    Codes = 4,
    StateAux = 5,
}

impl AppMemoryId {
    pub fn as_u8(self) -> u8 {
        match self {
            AppMemoryId::Upgrades => 0,
            AppMemoryId::Meta => 1,
            AppMemoryId::Accounts => 2,
            AppMemoryId::Storage => 3,
            AppMemoryId::Codes => 4,
            AppMemoryId::StateAux => 5,
        }
    }

    pub fn as_memory_id(self) -> MemoryId {
        MemoryId::new(self.as_u8())
    }
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

pub fn get_memory(id: AppMemoryId) -> VMem {
    MEMORY_MANAGER.with(|m| m.borrow().get(id.as_memory_id()))
}
