//! どこで: StableBTreeMapの結線 / 何を: accounts/storage/codesの初期化 / なぜ: MemoryId凍結を反映するため

use crate::memory::{get_memory, AppMemoryId, VMem};
use crate::types::keys::{AccountKey, CodeKey, StorageKey};
use crate::types::values::{AccountVal, CodeVal, U256Val};
use ic_stable_structures::StableBTreeMap;
use std::cell::RefCell;

pub type Accounts = StableBTreeMap<AccountKey, AccountVal, VMem>;
pub type Storage = StableBTreeMap<StorageKey, U256Val, VMem>;
pub type Codes = StableBTreeMap<CodeKey, CodeVal, VMem>;

pub struct StableState {
    pub accounts: Accounts,
    pub storage: Storage,
    pub codes: Codes,
}

thread_local! {
    static STABLE_STATE: RefCell<Option<StableState>> = RefCell::new(None);
}

pub fn init_stable_state() {
    let accounts = StableBTreeMap::init(get_memory(AppMemoryId::Accounts));
    let storage = StableBTreeMap::init(get_memory(AppMemoryId::Storage));
    let codes = StableBTreeMap::init(get_memory(AppMemoryId::Codes));
    STABLE_STATE.with(|s| {
        *s.borrow_mut() = Some(StableState {
            accounts,
            storage,
            codes,
        });
    });
}

pub fn with_state<R>(f: impl FnOnce(&StableState) -> R) -> R {
    STABLE_STATE.with(|s| {
        let borrowed = s.borrow();
        let state = borrowed
            .as_ref()
            .unwrap_or_else(|| ic_cdk::trap("stable_state: not initialized"));
        f(state)
    })
}

pub fn with_state_mut<R>(f: impl FnOnce(&mut StableState) -> R) -> R {
    STABLE_STATE.with(|s| {
        let mut borrowed = s.borrow_mut();
        let state = borrowed
            .as_mut()
            .unwrap_or_else(|| ic_cdk::trap("stable_state: not initialized"));
        f(state)
    })
}
