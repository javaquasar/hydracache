use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

use crate::{CacheError, Result};

/// Serialization boundary for cached values.
///
/// Implement this trait to replace the default [`PostcardCodec`].
///
/// # Example
///
/// ```rust
/// use hydracache_core::{CacheCodec, PostcardCodec};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
/// struct User {
///     id: u64,
/// }
///
/// let codec = PostcardCodec;
/// let bytes = codec.encode(&User { id: 1 }).unwrap();
/// let decoded: User = codec.decode(&bytes).unwrap();
///
/// assert_eq!(decoded, User { id: 1 });
/// ```
pub trait CacheCodec: Clone + Send + Sync + 'static {
    /// Encode a typed value into bytes.
    fn encode<T>(&self, value: &T) -> Result<Bytes>
    where
        T: Serialize;

    /// Decode bytes back into a typed value.
    fn decode<T>(&self, bytes: &Bytes) -> Result<T>
    where
        T: DeserializeOwned;
}

/// Default compact binary codec for v0.
///
/// `PostcardCodec` is compact and works well for local cache values that derive
/// `serde::Serialize` and `serde::Deserialize`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PostcardCodec;

impl CacheCodec for PostcardCodec {
    fn encode<T>(&self, value: &T) -> Result<Bytes>
    where
        T: Serialize,
    {
        postcard::to_allocvec(value)
            .map(Bytes::from)
            .map_err(|source| CacheError::Encode(source.to_string()))
    }

    fn decode<T>(&self, bytes: &Bytes) -> Result<T>
    where
        T: DeserializeOwned,
    {
        postcard::from_bytes(bytes).map_err(|source| CacheError::Decode(source.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use serde::ser::{Serialize, Serializer};

    use super::*;

    struct FailingSerialize;

    impl Serialize for FailingSerialize {
        fn serialize<S>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(serde::ser::Error::custom("intentional encode failure"))
        }
    }

    #[test]
    fn postcard_codec_reports_encode_errors() {
        let error = PostcardCodec.encode(&FailingSerialize).unwrap_err();

        assert!(matches!(error, CacheError::Encode(_)));
    }

    #[test]
    fn postcard_codec_default_is_available() {
        let codec = PostcardCodec;
        let default_codec = PostcardCodec::default();

        assert_eq!(format!("{codec:?}"), format!("{default_codec:?}"));
    }
}
