//! どこで: chain_data 共通codec補助 / 何を: Bound防波堤とdecode方針補助 / なぜ: 破損時の扱いを統一するため

use crate::corrupt_log::record_corrupt;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodeOverflow;

pub fn encode_guarded<'a>(
    label: &'static [u8],
    bytes: Cow<'a, [u8]>,
    max_size: u32,
) -> Result<Cow<'a, [u8]>, EncodeOverflow> {
    if !ensure_encoded_within_bound(label, bytes.len(), max_size) {
        return Err(EncodeOverflow);
    }
    Ok(bytes)
}

pub fn ensure_encoded_within_bound(label: &'static [u8], encoded_len: usize, max_size: u32) -> bool {
    if encoded_len > max_size as usize {
        record_corrupt(label);
    }
    encoded_len <= max_size as usize
}

pub fn mark_decode_failure(label: &'static [u8], fail_closed: bool) {
    record_corrupt(label);
    if fail_closed {
        crate::meta::set_needs_migration(true);
    }
}

#[cfg(test)]
mod tests {
    use super::encode_guarded;
    use std::borrow::Cow;

    #[test]
    fn encode_guarded_accepts_borrowed() {
        let buf = [0x11u8; 4];
        let encoded = encode_guarded(b"borrowed", Cow::Borrowed(&buf), 4).expect("encode_guarded");
        assert_eq!(encoded.as_ref(), &buf);
    }
}
