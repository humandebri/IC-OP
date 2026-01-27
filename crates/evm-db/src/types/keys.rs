//! どこで: StableBTreeMapのKey / 何を: 固定長キー定義 / なぜ: 決定的な順序を保証するため

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AccountKey(pub [u8; 21]);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StorageKey(pub [u8; 53]);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CodeKey(pub [u8; 33]);

pub fn make_account_key(addr20: [u8; 20]) -> AccountKey {
    let mut buf = [0u8; 21];
    buf[0] = 0x01;
    buf[1..21].copy_from_slice(&addr20);
    AccountKey(buf)
}

pub fn make_storage_key(addr20: [u8; 20], slot32: [u8; 32]) -> StorageKey {
    let mut buf = [0u8; 53];
    buf[0] = 0x02;
    buf[1..21].copy_from_slice(&addr20);
    buf[21..53].copy_from_slice(&slot32);
    StorageKey(buf)
}

pub fn make_code_key(code_hash32: [u8; 32]) -> CodeKey {
    let mut buf = [0u8; 33];
    buf[0] = 0x03;
    buf[1..33].copy_from_slice(&code_hash32);
    CodeKey(buf)
}
