use crate::{
    Message, Result, Signature, Type,
    wire::serialized::{self, Data},
};

/// The body of a message.
///
/// This contains the bytes and the signature of the body.
#[derive(Clone, Debug)]
pub struct Body {
    data: Data<'static, 'static>,
    msg: Message,
}

impl Body {
    pub(super) fn new(data: Data<'static, 'static>, msg: Message) -> Self {
        Self { data, msg }
    }

    /// Deserialize the body using the contained signature.
    pub fn deserialize<'s, B>(&'s self) -> Result<B>
    where
        B: crate::wire::DynamicDeserialize<'s>,
    {
        let body_sig = self.signature();

        self.data
            .deserialize_for_dynamic_signature(body_sig)
            .map(|b| b.0)
    }

    /// Deserialize the body, ignoring its signature.
    ///
    /// Unlike [`Body::deserialize`], the signature of the body is not used to guide the
    /// deserialization: `B`'s own signature is assumed instead. This is useful when the body's
    /// signature is already known to match `B`, or when `B` deserializes from a signature that
    /// differs from the body's but is compatible with its encoding.
    ///
    /// Deserialization still fails with an error if the encoded bytes do not match what `B`
    /// expects, so this is not an unsafe or unchecked operation.
    pub fn deserialize_ignore_signature<'d, 'm: 'd, B>(&'m self) -> Result<B>
    where
        B: serde::de::Deserialize<'d> + Type,
    {
        self.data.deserialize().map(|b| b.0)
    }

    /// The signature of the body.
    pub fn signature(&self) -> &Signature {
        self.msg.quick_fields().signature()
    }

    /// The length of the body in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the body is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get a reference to the underlying byte encoding of the message.
    pub fn data(&self) -> &serialized::Data<'static, 'static> {
        &self.data
    }

    /// Reference to the message this body belongs to.
    pub fn message(&self) -> &Message {
        &self.msg
    }
}
