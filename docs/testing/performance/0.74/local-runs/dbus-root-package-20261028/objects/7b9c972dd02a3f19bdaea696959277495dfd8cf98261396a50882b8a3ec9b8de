use crate::wire::Endian;

/// The encoding context to use with the [serialization] and [deserialization] API.
///
/// The encoding is dependent on the position of the encoding in the entire message and hence the
/// need to [specify] the byte position of the data being serialized or deserialized. Simply pass
/// `0` if serializing or deserializing to or from the beginning of message, or the preceding bytes
/// end on an 8-byte boundary.
///
/// # Examples
///
/// ```
/// use zbus::wire::Endian;
/// use zbus::wire::serialized::Context;
/// use zbus::wire::to_bytes;
///
/// let str_vec = vec!["Hello", "World"];
/// let ctxt = Context::new(Endian::Little, 0);
/// let encoded = to_bytes(ctxt, &str_vec).unwrap();
///
/// // Let's decode the 2nd element of the array only
/// let slice = encoded.slice(14..);
/// let decoded: &str = slice.deserialize().unwrap().0;
/// assert_eq!(decoded, "World");
/// ```
///
/// [serialization]: crate::wire#functions
/// [deserialization]: crate::wire::serialized::Data::deserialize
/// [specify]: Context::new
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Context {
    position: usize,
    endian: Endian,
}

impl Context {
    /// Create a new encoding context.
    pub fn new(endian: Endian, position: usize) -> Self {
        Self { position, endian }
    }

    /// Deprecated alias of [`Context::new`].
    ///
    /// The wire format is always D-Bus, so this says nothing [`Context::new`] does not. It is
    /// removed in zbus 7.0.
    #[deprecated(
        since = "6.0.0",
        note = "the wire format is always D-Bus; use `Context::new` instead. Removed in 7.0."
    )]
    pub fn new_dbus(endian: Endian, position: usize) -> Self {
        Self::new(endian, position)
    }

    /// The [`Endian`] of this context.
    pub fn endian(self) -> Endian {
        self.endian
    }

    /// The byte position of the value to be encoded or decoded, in the entire message.
    pub fn position(self) -> usize {
        self.position
    }
}
