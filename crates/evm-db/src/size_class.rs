//! どこで: BlobStoreのサイズクラス / 何を: クラス選択と上限 / なぜ: 再利用単位を固定して安定化するため

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SizeClassError {
    ZeroLength,
    TooLarge,
}

pub const CLASS_64K: u32 = 64 * 1024;
pub const CLASS_128K: u32 = 128 * 1024;
pub const CLASS_256K: u32 = 256 * 1024;
pub const CLASS_512K: u32 = 512 * 1024;
pub const CLASS_1M: u32 = 1024 * 1024;
pub const CLASS_2M: u32 = 2 * 1024 * 1024;
pub const CLASS_4M: u32 = 4 * 1024 * 1024;

const CLASSES: [u32; 7] = [
    CLASS_64K,
    CLASS_128K,
    CLASS_256K,
    CLASS_512K,
    CLASS_1M,
    CLASS_2M,
    CLASS_4M,
];

pub fn smallest_class(len: usize) -> Result<u32, SizeClassError> {
    if len == 0 {
        return Err(SizeClassError::ZeroLength);
    }
    let len_u32 = u32::try_from(len).map_err(|_| SizeClassError::TooLarge)?;
    for class in CLASSES.iter() {
        if len_u32 <= *class {
            return Ok(*class);
        }
    }
    Err(SizeClassError::TooLarge)
}
