//! Validators for the D-Bus name types.
//!
//! The rules are those of the [D-Bus specification][spec] and the error messages are the ones
//! the name types report to their users.
//!
//! [spec]: https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names

/// Which kind of bus name a string turned out to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusNameKind {
    /// A unique name, handed out by the bus.
    Unique,
    /// A well-known name, requested by a service.
    WellKnown,
}

/// Validate a unique bus name.
pub fn validate_unique_name(bytes: &[u8]) -> Result<(), &'static str> {
    parse_unique_name(bytes).map_err(|()| UNIQUE_NAME_ERROR)
}

/// Validate a well-known bus name.
pub fn validate_well_known_name(bytes: &[u8]) -> Result<(), &'static str> {
    parse_well_known_name(bytes).map_err(|()| WELL_KNOWN_NAME_ERROR)
}

/// Validate a bus name, reporting which of the two kinds it is.
///
/// Unique names are tried first, since theirs is the cheaper parse.
pub fn validate_bus_name(bytes: &[u8]) -> Result<BusNameKind, &'static str> {
    if parse_unique_name(bytes).is_ok() {
        Ok(BusNameKind::Unique)
    } else if parse_well_known_name(bytes).is_ok() {
        Ok(BusNameKind::WellKnown)
    } else {
        Err(BUS_NAME_ERROR)
    }
}

/// Validate an interface name.
pub fn validate_interface_name(bytes: &[u8]) -> Result<(), &'static str> {
    parse_interface_name(bytes).map_err(|()| INTERFACE_NAME_ERROR)
}

/// Validate an error name.
pub fn validate_error_name(bytes: &[u8]) -> Result<(), &'static str> {
    // Error names follow the same rules as interface names.
    parse_interface_name(bytes).map_err(|()| ERROR_NAME_ERROR)
}

/// Validate a member (method or signal) name.
pub fn validate_member_name(bytes: &[u8]) -> Result<(), &'static str> {
    parse_member_name(bytes).map_err(|()| MEMBER_NAME_ERROR)
}

/// Validate a property name.
pub fn validate_property_name(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.is_empty() {
        return Err(PROPERTY_NAME_EMPTY_ERROR);
    } else if bytes.len() > 255 {
        return Err(PROPERTY_NAME_TOO_LONG_ERROR);
    }

    Ok(())
}

fn parse_unique_name(bytes: &[u8]) -> Result<(), ()> {
    use winnow::{
        Parser,
        combinator::{alt, separated},
        stream::AsChar,
        token::take_while,
    };
    // Rules
    //
    // * Only ASCII alphanumeric, `_` or '-'
    // * Must begin with a `:`.
    // * Must contain at least one `.`.
    // * Each element must be 1 character (so name must be minimum 4 characters long).
    // * <= 255 characters.
    let element = take_while::<_, _, ()>(1.., (AsChar::is_alphanum, b'_', b'-'));
    let peer_name = (b':', (separated(2.., element, b'.'))).map(|_: (_, ())| ());
    let bus_name = b"org.freedesktop.DBus".map(|_| ());
    let mut unique_name = alt((bus_name, peer_name));

    unique_name.parse(bytes).map_err(|_| ()).and_then(|_: ()| {
        // Least likely scenario so we check this last.
        if bytes.len() > 255 {
            return Err(());
        }

        Ok(())
    })
}

fn parse_well_known_name(bytes: &[u8]) -> Result<(), ()> {
    use winnow::{
        Parser,
        combinator::separated,
        stream::AsChar,
        token::{one_of, take_while},
    };
    // Rules
    //
    // * Only ASCII alphanumeric, `_` or '-'.
    // * Must not begin with a `.`.
    // * Must contain at least one `.`.
    // * Each element must:
    //  * not begin with a digit.
    //  * be 1 character (so name must be minimum 3 characters long).
    // * <= 255 characters.
    let first_element_char = one_of((AsChar::is_alpha, b'_', b'-'));
    let subsequent_element_chars = take_while::<_, _, ()>(0.., (AsChar::is_alphanum, b'_', b'-'));
    let element = (first_element_char, subsequent_element_chars);
    let mut well_known_name = separated(2.., element, b'.');

    well_known_name
        .parse(bytes)
        .map_err(|_| ())
        .and_then(|_: ()| {
            // Least likely scenario so we check this last.
            if bytes.len() > 255 {
                return Err(());
            }

            Ok(())
        })
}

fn parse_interface_name(bytes: &[u8]) -> Result<(), ()> {
    use winnow::{
        Parser,
        combinator::separated,
        stream::AsChar,
        token::{one_of, take_while},
    };
    // Rules
    //
    // * Only ASCII alphanumeric and `_`
    // * Must not begin with a `.`.
    // * Must contain at least one `.`.
    // * Each element must:
    //  * not begin with a digit.
    //  * be 1 character (so name must be minimum 3 characters long).
    // * <= 255 characters.
    //
    // Note: A `-` not allowed, which is why we can't use the same parser as for `WellKnownName`.
    let first_element_char = one_of((AsChar::is_alpha, b'_'));
    let subsequent_element_chars = take_while::<_, _, ()>(0.., (AsChar::is_alphanum, b'_'));
    let element = (first_element_char, subsequent_element_chars);
    let mut interface_name = separated(2.., element, b'.');

    interface_name
        .parse(bytes)
        .map_err(|_| ())
        .and_then(|_: ()| {
            // Least likely scenario so we check this last.
            if bytes.len() > 255 {
                return Err(());
            }

            Ok(())
        })
}

fn parse_member_name(bytes: &[u8]) -> Result<(), ()> {
    use winnow::{
        Parser,
        stream::AsChar,
        token::{one_of, take_while},
    };
    // Rules
    //
    // * Only ASCII alphanumeric or `_`.
    // * Must not begin with a digit.
    // * Must contain at least 1 character.
    // * <= 255 characters.
    let first_element_char = one_of((AsChar::is_alpha, b'_'));
    let subsequent_element_chars = take_while::<_, _, ()>(0.., (AsChar::is_alphanum, b'_'));
    let mut member_name = (first_element_char, subsequent_element_chars);

    member_name.parse(bytes).map_err(|_| ()).and_then(|_| {
        // Least likely scenario so we check this last.
        if bytes.len() > 255 {
            return Err(());
        }

        Ok(())
    })
}

const UNIQUE_NAME_ERROR: &str = "Invalid unique name. See \
    https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-bus";

const WELL_KNOWN_NAME_ERROR: &str = "Invalid well-known name. See \
    https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-bus";

const BUS_NAME_ERROR: &str = "Invalid bus name. See \
    https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-bus";

const INTERFACE_NAME_ERROR: &str = "Invalid interface name. See \
    https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-interface";

const ERROR_NAME_ERROR: &str = "Invalid error name. See \
    https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-error";

const MEMBER_NAME_ERROR: &str = "Invalid member name. See \
    https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-member";

const PROPERTY_NAME_EMPTY_ERROR: &str =
    "Invalid property name. It has to be at least 1 character long.";

const PROPERTY_NAME_TOO_LONG_ERROR: &str =
    "Invalid property name. It can not be longer than 255 characters.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_names() {
        let valid: [&[u8]; 3] = [
            b":org.gnome.Service-for_you",
            b":a.very.loooooooooooooooooo-ooooooo_0000o0ng.Name",
            b"org.freedesktop.DBus",
        ];
        for name in valid {
            validate_unique_name(name).unwrap();
        }

        let invalid: [&[u8]; 6] = [
            b"",
            b"dont.start.with.a.colon",
            b":double..dots",
            b".",
            b".start.with.dot",
            b":no-dots",
        ];
        for name in invalid {
            assert_eq!(validate_unique_name(name), Err(UNIQUE_NAME_ERROR));
        }
    }

    #[test]
    fn well_known_names() {
        let valid: [&[u8]; 2] = [
            b"org.gnome.Service-for_you",
            b"a.very.loooooooooooooooooo-ooooooo_0000o0ng.Name",
        ];
        for name in valid {
            validate_well_known_name(name).unwrap();
        }

        let invalid: [&[u8]; 7] = [
            b"",
            b"double..dots",
            b".",
            b".start.with.dot",
            b"1st.element.starts.with.digit",
            b"the.2nd.element.starts.with.digit",
            b"no-dots",
        ];
        for name in invalid {
            assert_eq!(validate_well_known_name(name), Err(WELL_KNOWN_NAME_ERROR));
        }
    }

    #[test]
    fn bus_names() {
        assert_eq!(
            validate_bus_name(b":org.gnome.Service-for_you"),
            Ok(BusNameKind::Unique)
        );
        assert_eq!(
            validate_bus_name(b"org.gnome.Service-for_you"),
            Ok(BusNameKind::WellKnown)
        );
        // The bus's own name looks well-known but the spec classifies it as unique, which
        // only holds if unique names are tried before well-known ones.
        assert_eq!(
            validate_bus_name(b"org.freedesktop.DBus"),
            Ok(BusNameKind::Unique)
        );

        let invalid: [&[u8]; 6] = [
            b"",
            b"double..dots",
            b".",
            b".start.with.dot",
            b"1start.with.digit",
            b"no-dots",
        ];
        for name in invalid {
            assert_eq!(validate_bus_name(name), Err(BUS_NAME_ERROR));
        }
    }

    #[test]
    fn interface_names() {
        let valid: [&[u8]; 2] = [
            b"org.gnome.Interface_for_you",
            b"a.very.loooooooooooooooooo_ooooooo_0000o0ng.Name",
        ];
        for name in valid {
            validate_interface_name(name).unwrap();
        }

        let invalid: [&[u8]; 9] = [
            b"",
            b":start.with.a.colon",
            b"double..dots",
            b".",
            b".start.with.dot",
            b"no-dots",
            b"1st.element.starts.with.digit",
            b"the.2nd.element.starts.with.digit",
            b"contains.dashes-in.the.name",
        ];
        for name in invalid {
            assert_eq!(validate_interface_name(name), Err(INTERFACE_NAME_ERROR));
        }
    }

    #[test]
    fn error_names() {
        validate_error_name(b"org.gnome.Error_for_you").unwrap();
        assert_eq!(
            validate_error_name(b"contains.dashes-in.the.name"),
            Err(ERROR_NAME_ERROR)
        );
    }

    #[test]
    fn member_names() {
        let valid: [&[u8]; 3] = [
            b"Member_for_you",
            b"CamelCase101",
            b"a_very_loooooooooooooooooo_ooooooo_0000o0ngName",
        ];
        for name in valid {
            validate_member_name(name).unwrap();
        }

        let invalid: [&[u8]; 5] = [
            b"",
            b".",
            b"1startWith_a_Digit",
            b"contains.dots_in_the_name",
            b"contains-dashes-in_the_name",
        ];
        for name in invalid {
            assert_eq!(validate_member_name(name), Err(MEMBER_NAME_ERROR));
        }
    }

    #[test]
    fn property_names() {
        let valid: [&[u8]; 3] = [b"Property_for_you", b"CamelCase101", b"Property_for_you-1"];
        for name in valid {
            validate_property_name(name).unwrap();
        }

        assert_eq!(validate_property_name(b""), Err(PROPERTY_NAME_EMPTY_ERROR));
        assert_eq!(
            validate_property_name(&[b'a'; 256]),
            Err(PROPERTY_NAME_TOO_LONG_ERROR)
        );
    }

    #[test]
    fn too_long_names() {
        let long_unique = [&b":a."[..], &[b'b'; 253]].concat();
        assert_eq!(validate_unique_name(&long_unique), Err(UNIQUE_NAME_ERROR));

        let long_well_known = [&b"a."[..], &[b'b'; 254]].concat();
        assert_eq!(
            validate_well_known_name(&long_well_known),
            Err(WELL_KNOWN_NAME_ERROR)
        );

        let long_interface = [&b"a."[..], &[b'b'; 254]].concat();
        assert_eq!(
            validate_interface_name(&long_interface),
            Err(INTERFACE_NAME_ERROR)
        );

        assert_eq!(validate_member_name(&[b'a'; 256]), Err(MEMBER_NAME_ERROR));
    }

    // These messages are part of the observable behaviour of the name types, so pin
    // them here rather than only comparing constants against themselves.
    #[test]
    fn error_messages() {
        assert_eq!(
            UNIQUE_NAME_ERROR,
            "Invalid unique name. See https://dbus.freedesktop.org/doc/dbus-specification.html\
             #message-protocol-names-bus"
        );
        assert_eq!(
            WELL_KNOWN_NAME_ERROR,
            "Invalid well-known name. See https://dbus.freedesktop.org/doc/dbus-specification\
             .html#message-protocol-names-bus"
        );
        assert_eq!(
            BUS_NAME_ERROR,
            "Invalid bus name. See https://dbus.freedesktop.org/doc/dbus-specification.html\
             #message-protocol-names-bus"
        );
        assert_eq!(
            INTERFACE_NAME_ERROR,
            "Invalid interface name. See https://dbus.freedesktop.org/doc/dbus-specification\
             .html#message-protocol-names-interface"
        );
        assert_eq!(
            ERROR_NAME_ERROR,
            "Invalid error name. See https://dbus.freedesktop.org/doc/dbus-specification.html\
             #message-protocol-names-error"
        );
        assert_eq!(
            MEMBER_NAME_ERROR,
            "Invalid member name. See https://dbus.freedesktop.org/doc/dbus-specification.html\
             #message-protocol-names-member"
        );
        assert_eq!(
            PROPERTY_NAME_EMPTY_ERROR,
            "Invalid property name. It has to be at least 1 character long."
        );
        assert_eq!(
            PROPERTY_NAME_TOO_LONG_ERROR,
            "Invalid property name. It can not be longer than 255 characters."
        );
    }
}
