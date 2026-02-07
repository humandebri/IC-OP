//! どこで: BlobStoreのポインタ / 何を: BlobPtrのStorable化 / なぜ: stable上の参照を固定長で持つため

use crate::corrupt_log::record_corrupt;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use std::borrow::Cow;
use zerocopy::byteorder::big_endian::{U32, U64};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlobPtr {
    pub offset: u64,
    pub len: u32,
    pub class: u32,
    pub gen: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct BlobPtrWire {
    offset: U64,
    len: U32,
    class: U32,
    gen: U32,
}

impl BlobPtrWire {
    fn new(ptr: &BlobPtr) -> Self {
        Self {
            offset: U64::new(ptr.offset),
            len: U32::new(ptr.len),
            class: U32::new(ptr.class),
            gen: U32::new(ptr.gen),
        }
    }
}

impl Storable for BlobPtr {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let wire = BlobPtrWire::new(self);
        Cow::Owned(wire.as_bytes().to_vec())
    }

    fn into_bytes(self) -> Vec<u8> {
        let wire = BlobPtrWire::new(&self);
        wire.as_bytes().to_vec()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let data = bytes.as_ref();
        if data.len() != 20 {
            record_corrupt(b"blob_ptr");
            return Self {
                offset: 0,
                len: 0,
                class: 0,
                gen: 0,
            };
        }
        let wire = match BlobPtrWire::read_from_bytes(data) {
            Ok(value) => value,
            Err(_) => {
                record_corrupt(b"blob_ptr");
                return Self {
                    offset: 0,
                    len: 0,
                    class: 0,
                    gen: 0,
                };
            }
        };
        Self {
            offset: wire.offset.get(),
            len: wire.len.get(),
            class: wire.class.get(),
            gen: wire.gen.get(),
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 20,
        is_fixed_size: true,
    };
}
