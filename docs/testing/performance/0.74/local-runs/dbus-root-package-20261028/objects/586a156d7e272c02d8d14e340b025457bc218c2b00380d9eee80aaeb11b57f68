//! Invocation-local terminated bytes only; no observation, authority or path rule.
use std::ffi::{c_char, CString};

enum Storage {
    Short { bytes: [u8; 32], used: usize },
    Heap(CString),
}
pub(super) struct Name(Storage);
impl Name {
    pub(super) fn new(name: &str) -> Result<Self, ()> {
        if name.len() < 32 {
            if name.as_bytes().contains(&0) {
                return Err(());
            }
            let mut bytes = [0; 32];
            bytes[..name.len()].copy_from_slice(name.as_bytes());
            Ok(Self(Storage::Short {
                bytes,
                used: name.len() + 1,
            }))
        } else {
            CString::new(name)
                .map(|name| Self(Storage::Heap(name)))
                .map_err(|_| ())
        }
    }
    pub(super) fn bytes_with_nul(&self) -> &[u8] {
        match &self.0 {
            Storage::Short { bytes, used } => &bytes[..*used],
            Storage::Heap(name) => name.as_bytes_with_nul(),
        }
    }
    pub(super) fn as_ptr(&self) -> *const c_char {
        self.bytes_with_nul().as_ptr().cast()
    }
}
