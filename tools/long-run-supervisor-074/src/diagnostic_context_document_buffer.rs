//! Invocation-local zeroed storage only. No observation or authority is retained.
pub(super) enum Scratch {
    Short { bytes: [u8; 38], used: usize },
    Heap(Vec<u8>),
}
impl Scratch {
    pub(super) fn new(length: usize) -> Self {
        if length < 38 {
            Self::Short {
                bytes: [0; 38],
                used: length + 1,
            }
        } else {
            // Keep the original generic allocation/arithmetic for larger shapes.
            Self::Heap(vec![0; length + 1])
        }
    }
}
impl std::ops::Deref for Scratch {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Short { bytes, used } => &bytes[..*used],
            Self::Heap(bytes) => bytes,
        }
    }
}
impl std::ops::DerefMut for Scratch {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Short { bytes, used } => &mut bytes[..*used],
            Self::Heap(bytes) => bytes,
        }
    }
}
