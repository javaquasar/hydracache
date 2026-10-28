#[cfg(unix)]
use crate::OwnedFd;
use std::{borrow::Cow, io::Cursor, num::NonZeroU32, sync::Arc};

use crate::{
    DynamicType, Error, ObjectPath, Result, Signature,
    message::{EndianSig, Fields, Flags, Header, Message, PrimaryHeader, Sequence, Type},
    names::{BusName, ErrorName, InterfaceName, MemberName, UniqueName},
    utils::padding_for_8_bytes,
    wire::{Endian, serialized, serialized::Context},
};

use crate::message::header::MAX_MESSAGE_SIZE;

macro_rules! dbus_context {
    ($self:ident, $n_bytes_before: expr) => {
        Context::new($self.header.primary().endian_sig().into(), $n_bytes_before)
    };
}

/// Convert `$value` into the header field `$field`, recording a failure to convert it.
macro_rules! set_field {
    ($self:ident, $field:ident, $value:expr) => {{
        match $value.try_into() {
            Ok(value) => $self.header.fields_mut().$field = Some(value),
            Err(e) => $self.record(e.into()),
        }

        $self
    }};
}

/// A builder for a [`Message`].
///
/// The setters take the same loosely typed values as the rest of the API and convert them right
/// away, but they never fail: the first error a setter hits is recorded — a later call doesn't
/// clear it — and reported by [`Builder::build`].
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    header: Header<'a>,
    error: Option<Error>,
}

impl<'a> Builder<'a> {
    pub(super) fn new(msg_type: Type) -> Self {
        let primary = PrimaryHeader::new(msg_type, 0);
        let fields = Fields::new();
        let header = Header::new(primary, fields);
        Self {
            header,
            error: None,
        }
    }

    /// Add flags to the message, on top of any already added.
    ///
    /// See [`Flags`] documentation for the meaning of the flags. Combine them with `|` to add
    /// several in one call.
    ///
    /// Flags that are invalid for the message type are reported by [`Builder::build`].
    #[must_use]
    pub fn with_flags(mut self, flags: Flags) -> Self {
        if self.header.message_type() != Type::MethodCall
            && flags.contains(Flags::NO_REPLY_EXPECTED)
        {
            self.record(Error::InvalidField);

            return self;
        }
        let flags = self.header.primary().flags() | flags;
        self.header.primary_mut().set_flags(flags);

        self
    }

    /// Set the unique name of the sending connection.
    ///
    /// An invalid sender name is reported by [`Builder::build`].
    #[must_use]
    pub fn sender<'s: 'a, S>(mut self, sender: S) -> Self
    where
        S: TryInto<UniqueName<'s>>,
        S::Error: Into<Error>,
    {
        set_field!(self, sender, sender)
    }

    /// Set the object to send a call to, or the object a signal is emitted from.
    ///
    /// An invalid path is reported by [`Builder::build`].
    #[must_use]
    pub fn path<'p: 'a, P>(mut self, path: P) -> Self
    where
        P: TryInto<ObjectPath<'p>>,
        P::Error: Into<Error>,
    {
        set_field!(self, path, path)
    }

    /// Set the interface to invoke a method call on, or that a signal is emitted from.
    ///
    /// An invalid interface name is reported by [`Builder::build`].
    #[must_use]
    pub fn interface<'i: 'a, I>(mut self, interface: I) -> Self
    where
        I: TryInto<InterfaceName<'i>>,
        I::Error: Into<Error>,
    {
        set_field!(self, interface, interface)
    }

    /// Set the member, either the method name or signal name.
    ///
    /// An invalid member name is reported by [`Builder::build`].
    #[must_use]
    pub fn member<'m: 'a, M>(mut self, member: M) -> Self
    where
        M: TryInto<MemberName<'m>>,
        M::Error: Into<Error>,
    {
        set_field!(self, member, member)
    }

    /// Set the name of the error this message reports.
    ///
    /// An invalid error name is reported by [`Builder::build`].
    #[must_use]
    pub(super) fn error_name<'e: 'a, E>(mut self, error: E) -> Self
    where
        E: TryInto<ErrorName<'e>>,
        E::Error: Into<Error>,
    {
        set_field!(self, error_name, error)
    }

    /// Set the name of the connection this message is intended for.
    ///
    /// An invalid destination name is reported by [`Builder::build`].
    #[must_use]
    pub fn destination<'d: 'a, D>(mut self, destination: D) -> Self
    where
        D: TryInto<BusName<'d>>,
        D::Error: Into<Error>,
    {
        set_field!(self, destination, destination)
    }

    /// Override the generated or inherited serial.  This is a low level modification,
    /// generally you should not need to use this.
    pub fn serial(mut self, serial: NonZeroU32) -> Self {
        self.header.primary_mut().set_serial_num(serial);
        self
    }

    /// Override the reply serial. This is a low level modification, generally you should use
    /// `Message::method_return` instead.
    pub fn reply_serial(mut self, serial: Option<NonZeroU32>) -> Self {
        self.header.fields_mut().reply_serial = serial;
        self
    }

    #[must_use]
    pub(super) fn reply_to(mut self, reply_to: &Header<'_>) -> Self {
        let serial = reply_to.primary().serial_num();
        self.header.fields_mut().reply_serial = Some(serial);
        self = self.endian(reply_to.primary().endian_sig().into());

        match reply_to.sender() {
            Some(sender) => self.destination(sender.to_owned()),
            None => self,
        }
    }

    /// Set the endianness of the message.
    ///
    /// The default endianness is native.
    pub fn endian(mut self, endian: Endian) -> Self {
        let sig = EndianSig::from(endian);
        self.header.primary_mut().set_endian_sig(sig);

        self
    }

    /// Build the [`Message`] with the given body.
    ///
    /// You may pass `()` as the body if the message has no body.
    ///
    /// The caller is currently required to ensure that the resulting message contains the headers
    /// as compliant with the [specification]. Additional checks may be added to this builder over
    /// time as needed.
    ///
    /// [specification]:
    /// https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-header-fields
    ///
    /// # Errors
    ///
    /// Returns the first error recorded by a setter, then any error from serializing the body or
    /// from the message exceeding the maximum message size.
    pub fn build<B>(mut self, body: &B) -> Result<Message>
    where
        B: serde::ser::Serialize + DynamicType,
    {
        if let Some(error) = self.error.take() {
            return Err(error);
        }

        let ctxt = dbus_context!(self, 0);
        let signature = body.signature();

        // The header carries the body's length and FD count, so the body is serialized first,
        // into its own buffer, and copied into place behind the header. One pass over the body
        // plus a copy of its bytes beats the extra serialization pass that measuring it first
        // would cost, and keeps this generic function down to the serialization itself.
        let mut body_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut body_bytes);
        // SAFETY: The FDs end up in the same Message as the body.
        let written =
            unsafe { crate::wire::to_writer_for_signature(&mut cursor, ctxt, &signature, body) }?;
        #[cfg(unix)]
        let fds = written.into_fds();
        #[cfg(not(unix))]
        let _ = written;

        self.build_from_bytes(
            signature,
            &body_bytes,
            #[cfg(unix)]
            fds,
        )
    }

    /// Create a new message from a raw slice of bytes to populate the body with, rather than by
    /// serializing a value. The message body will be the exact bytes.
    ///
    /// # Errors
    ///
    /// The same errors as [`Builder::build`], except that the body is taken as is instead of
    /// being serialized, and an invalid `signature` is reported here.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it can be used to build an invalid message.
    pub unsafe fn build_raw_body<S>(
        mut self,
        body_bytes: &[u8],
        signature: S,
        #[cfg(unix)] fds: Vec<OwnedFd>,
    ) -> Result<Message>
    where
        S: TryInto<Signature>,
        S::Error: Into<Error>,
    {
        if let Some(error) = self.error.take() {
            return Err(error);
        }

        let signature = signature.try_into().map_err(Into::into)?;

        self.build_from_bytes(
            signature,
            body_bytes,
            #[cfg(unix)]
            fds,
        )
    }

    /// Build the message around an already serialized body.
    fn build_from_bytes(
        self,
        signature: Signature,
        body: &[u8],
        #[cfg(unix)] fds: Vec<OwnedFd>,
    ) -> Result<Message> {
        let ctxt = dbus_context!(self, 0);
        let mut header = self.header;

        header.fields_mut().signature = Cow::Owned(signature);

        let body_len_u32 = body.len().try_into().map_err(|_| Error::ExcessData)?;
        header.primary_mut().set_body_len(body_len_u32);

        #[cfg(unix)]
        {
            let fds_len: u32 = fds.len().try_into().map_err(|_| Error::ExcessData)?;
            if fds_len != 0 {
                header.fields_mut().unix_fds = Some(fds_len);
            }
        }

        let mut bytes = Vec::new();
        let mut cursor = Cursor::new(&mut bytes);
        // SAFETY: There are no FDs involved.
        unsafe { crate::wire::to_writer(&mut cursor, ctxt, &header) }?;
        let hdr_len = bytes.len();
        // We need to align the body to 8-byte boundary.
        let body_offset = hdr_len + padding_for_8_bytes(hdr_len);
        let total_len = body_offset + body.len();
        if total_len > MAX_MESSAGE_SIZE {
            return Err(Error::ExcessData);
        }
        bytes.reserve_exact(total_len - hdr_len);
        bytes.resize(body_offset, 0);
        bytes.extend_from_slice(body);

        let primary_header = header.into_primary();
        #[cfg(unix)]
        let bytes = serialized::Data::new_fds(bytes, ctxt, fds);
        #[cfg(not(unix))]
        let bytes = serialized::Data::new(bytes, ctxt);

        Ok(Message {
            inner: Arc::new(super::Inner {
                primary_header,
                quick_fields: std::sync::OnceLock::new(),
                bytes,
                body_offset,
                recv_seq: Sequence::default(),
            }),
        })
    }

    /// Record `error`, unless an earlier setter already recorded one.
    fn record(&mut self, error: Error) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
}

impl<'m> From<Header<'m>> for Builder<'m> {
    fn from(mut header: Header<'m>) -> Self {
        // Signature and Fds are added by body* methods.
        let fields = header.fields_mut();
        fields.signature = Cow::Owned(Signature::Unit);
        fields.unix_fds = None;

        Self {
            header,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Endian, Message};
    use crate::{
        Error, ObjectPath,
        message::Flags,
        names::{BusName, InterfaceName},
    };
    use test_log::test;

    #[test]
    fn strings() {
        let message = Message::method_call("/org/zbus/Test", "Test")
            .destination("org.zbus.Test")
            .interface("org.zbus.Test")
            .build(&())
            .unwrap();
        assert_eq!(message.header().path().unwrap().as_str(), "/org/zbus/Test");

        // An invalid string is only reported by `build`.
        let error = Message::method_call("not a path", "Test")
            .build(&())
            .unwrap_err();
        assert_eq!(error, ObjectPath::try_from("not a path").unwrap_err());

        let error = Message::method_call("/org/zbus/Test", "Test")
            .destination("not a bus name")
            .build(&())
            .unwrap_err();
        assert_eq!(error, BusName::try_from("not a bus name").unwrap_err());
    }

    #[test]
    fn typed_values() {
        // No `Result` anywhere before `build`.
        let path = ObjectPath::try_from("/org/zbus/Test").unwrap();
        let interface = InterfaceName::try_from("org.zbus.Test").unwrap();
        let builder = Message::signal(&path, &interface, "Test").sender(":1.72");
        let message = builder.build(&()).unwrap();
        assert_eq!(message.header().interface().unwrap(), &interface);
    }

    #[test]
    fn a_later_valid_call_keeps_the_first_error() {
        // Setting the field again, even to a valid value, doesn't clear the error it recorded.
        let error = Message::method_call("/", "Test")
            .path("not a path")
            .path("/org/zbus/Test")
            .build(&())
            .unwrap_err();
        assert_eq!(error, ObjectPath::try_from("not a path").unwrap_err());

        // Same for a valid flag after an invalid one.
        let error = Message::signal("/", "org.zbus.Test", "Test")
            .with_flags(Flags::NO_REPLY_EXPECTED)
            .with_flags(Flags::NO_AUTO_START)
            .build(&())
            .unwrap_err();
        assert_eq!(error, Error::InvalidField);
    }

    #[test]
    fn raw() -> Result<(), Error> {
        let raw_body: &[u8] = &[16, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0];
        let message_builder = Message::signal("/", "test.test", "test");
        let message_builder = message_builder.endian(Endian::Little);
        let message = unsafe {
            message_builder.build_raw_body(
                raw_body,
                "ai",
                #[cfg(unix)]
                vec![],
            )?
        };

        let output: Vec<i32> = message.body().deserialize()?;
        assert_eq!(output, vec![1, 2, 3, 4]);

        Ok(())
    }
}
