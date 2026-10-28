//! Validator for D-Bus object paths.

/// Validate a D-Bus object path.
pub fn validate(bytes: &[u8]) -> Result<(), &'static str> {
    use winnow::{Parser, combinator::separated, stream::AsChar, token::take_while};
    // Rules
    //
    // * At least 1 character.
    // * First character must be `/`
    // * No trailing `/`
    // * No `//`
    // * Only ASCII alphanumeric, `_` or '/'

    let allowed_chars = (AsChar::is_alphanum, b'_');
    let name = take_while::<_, _, ()>(1.., allowed_chars);
    let mut full_path = (b'/', separated(0.., name, b'/')).map(|_: (u8, ())| ());

    full_path.parse(bytes).map_err(|_| OBJECT_PATH_ERROR)
}

const OBJECT_PATH_ERROR: &str = "Invalid object path";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_paths() {
        let valid: [&[u8]; 3] = [
            b"/",
            b"/Path/t0/0bject",
            b"/a/very/looooooooooooooooooooooooo0000o0ng/path",
        ];
        for path in valid {
            validate(path).unwrap();
        }
    }

    #[test]
    fn invalid_paths() {
        let invalid: [&[u8]; 5] = [
            b"",
            b"/double//slashes/",
            b".",
            b"/end/with/slash/",
            b"/ha.d",
        ];
        for path in invalid {
            assert_eq!(validate(path), Err(OBJECT_PATH_ERROR));
        }
    }

    // This message is part of the observable behaviour of the object-path type's error variant,
    // so pin it here rather than only comparing the constant against itself.
    #[test]
    fn error_message() {
        assert_eq!(OBJECT_PATH_ERROR, "Invalid object path");
    }
}
