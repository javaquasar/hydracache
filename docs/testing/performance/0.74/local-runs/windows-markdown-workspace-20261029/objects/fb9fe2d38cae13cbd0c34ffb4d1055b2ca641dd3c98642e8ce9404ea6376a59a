//! Pure adapter to the actual owned name and complete child body, not openat.
use super::{component_name::Name, TrustInputError};
use std::ffi::CStr;

fn view(name: &Name) -> &CStr {
    let value = CStr::from_bytes_with_nul(name.bytes_with_nul()).expect("constructed name");
    assert_eq!(name.as_ptr(), value.as_ptr());
    value
}
pub(crate) fn with_name<T>(input: &str, inspect: impl FnOnce(&CStr) -> T) -> Result<T, ()> {
    let name = Name::new(input)?;
    Ok(inspect(view(&name)))
}
pub(crate) fn owned_after_input_mutation(input: &str) -> Vec<u8> {
    let mut original = input.to_owned();
    let name = Name::new(&original).unwrap();
    original.clear();
    original.push_str("changed");
    let moved = name;
    view(&moved).to_bytes_with_nul().to_vec()
}
pub(crate) fn child_model(
    parent: i32,
    name: &str,
    directory: bool,
    open: impl FnOnce(i32, &CStr, i32) -> i32,
    last_error: impl FnOnce() -> std::io::Error,
) -> Result<i32, TrustInputError> {
    let name = Name::new(name).map_err(|_| TrustInputError::Invalid)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = open(parent, view(&name), flags);
    if fd < 0 {
        return Err(last_error().into());
    }
    Ok(fd)
}
