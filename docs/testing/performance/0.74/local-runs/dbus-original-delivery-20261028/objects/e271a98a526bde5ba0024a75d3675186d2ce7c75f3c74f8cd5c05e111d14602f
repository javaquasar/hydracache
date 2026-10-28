//! Pure CString baseline and complete child-body model; no live filesystem call.
use super::*;
use crate::diagnostic_worker_policy::trusted_input::TrustInputError;
use std::cell::RefCell;
use std::ffi::{CStr, CString};

fn frozen_child(
    parent: i32,
    name: &str,
    directory: bool,
    open: impl FnOnce(i32, &CStr, i32) -> i32,
    last_error: impl FnOnce() -> std::io::Error,
) -> Result<i32, TrustInputError> {
    let name = CString::new(name).map_err(|_| TrustInputError::Invalid)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = open(parent, &name, flags);
    if fd < 0 {
        return Err(last_error().into());
    }
    Ok(fd)
}

fn valid_cases() -> Vec<(String, String, bool)> {
    let mut cases = [
        "etc",
        "hydracache-perf",
        "worker-trust-074.json",
        "controller-trust-074.json",
        "held-runtime-trust-074.json",
        "oidc-trust-074.json",
    ]
    .into_iter()
    .map(|name| (name.into(), name.into(), true))
    .collect::<Vec<_>>();
    for length in [0, 1, 30, 31, 32, 63, 64, 255, 256, 4096] {
        cases.push((format!("length-{length}"), "a".repeat(length), false));
    }
    for (label, name) in [
        ("unicode-cyrillic", "ключ"),
        ("unicode-path", "é/名"),
        ("parent-component", "../x"),
        ("dot", "."),
        ("absolute-name", "/x"),
    ] {
        cases.push((label.into(), name.into(), false));
    }
    cases
}

#[test]
fn trust_component_baseline_valid_requests() {
    for (label, name, production) in valid_cases() {
        let (_, counts) = capture(|| {
            for _ in 0..256 {
                let value = CString::new(std::hint::black_box(name.as_str())).unwrap();
                let bytes = std::hint::black_box(value.as_bytes_with_nul());
                assert_eq!(&bytes[..name.len()], name.as_bytes());
                assert_eq!(bytes.len(), name.len() + 1);
                assert_eq!(bytes[name.len()], 0);
            }
        });
        assert_eq!(counts.allocations, 256);
        assert_eq!(counts.reallocations, 0);
        assert_eq!(counts.requested_bytes, (name.len() as u64 + 1) * 256);
        println!(
            "HC074-TRUST-COMPONENT-BASELINE {}",
            serde_json::json!({
                "label":label,"length":name.len(),"production":production,"iterations":256,
                "counts":counts,"rust_requests_only":true,"timing_claim":false,
                "profile":false,"server":false,"product":false
            })
        );
    }
}

#[test]
fn trust_component_baseline_nul_requests_preserve_error_bytes_and_position() {
    for (label, name) in [
        ("leading-nul", "\0abc"),
        ("interior-nul", "ab\0cd"),
        ("trailing-nul", "abc\0"),
        ("unicode-nul", "é\0名"),
    ] {
        let position = name.as_bytes().iter().position(|b| *b == 0).unwrap();
        let (_, counts) = capture(|| {
            for _ in 0..256 {
                let error = CString::new(std::hint::black_box(name)).unwrap_err();
                let value = std::hint::black_box(&error);
                assert_eq!(value.nul_position(), position);
                assert_eq!(error.into_vec().as_slice(), name.as_bytes());
            }
        });
        println!(
            "HC074-TRUST-COMPONENT-NUL-BASELINE {}",
            serde_json::json!({
                "label":label,"length":name.len(),"position":position,"iterations":256,
                "counts":counts,"rust_requests_only":true,"timing_claim":false,
                "profile":false,"server":false,"product":false
            })
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Open {
        parent: i32,
        bytes: Vec<u8>,
        flags: i32,
    },
    LastError,
}
fn class(result: Result<i32, TrustInputError>) -> String {
    match result {
        Ok(fd) => format!("fd:{fd}"),
        Err(TrustInputError::Invalid) => "invalid".into(),
        Err(TrustInputError::Io(error)) => format!("io:{:?}", error.raw_os_error()),
        Err(error) => panic!("unexpected pure error: {error}"),
    }
}
fn check_model(name: &str, parent: i32, directory: bool, fd: i32, errno: i32) {
    let events = RefCell::new(Vec::new());
    let result = frozen_child(
        parent,
        name,
        directory,
        |parent, bytes, flags| {
            events.borrow_mut().push(Event::Open {
                parent,
                bytes: bytes.to_bytes_with_nul().to_vec(),
                flags,
            });
            fd
        },
        || {
            events.borrow_mut().push(Event::LastError);
            std::io::Error::from_raw_os_error(errno)
        },
    );
    let mut expected = Vec::new();
    let expected_class = if name.as_bytes().contains(&0) {
        "invalid".to_owned()
    } else {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        let mut flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
        if directory {
            flags |= libc::O_DIRECTORY;
        }
        expected.push(Event::Open {
            parent,
            bytes,
            flags,
        });
        if fd < 0 {
            expected.push(Event::LastError);
            format!("io:{:?}", Some(errno))
        } else {
            format!("fd:{fd}")
        }
    };
    assert_eq!(class(result), expected_class);
    assert_eq!(events.into_inner(), expected);
}

#[test]
fn trust_component_baseline_explicit_flags_first_errors_and_generic_inputs() {
    for (_, name, _) in valid_cases() {
        for directory in [false, true] {
            for (fd, errno) in [
                (0, libc::EIO),
                (37, libc::EIO),
                (-1, libc::EINTR),
                (-2, libc::EACCES),
            ] {
                check_model(&name, -1, directory, fd, errno);
            }
        }
    }
    for name in ["\0", "a\0z", "abc\0", "\0\0", "é\0名"] {
        for directory in [false, true] {
            for fd in [-1, 0, 37] {
                check_model(name, -1, directory, fd, libc::EBADF);
            }
        }
        assert!(matches!(
            frozen_child(
                -1,
                name,
                false,
                |_, _, _| panic!("open after NUL refusal"),
                || panic!("last error after NUL refusal")
            ),
            Err(TrustInputError::Invalid)
        ));
    }
}

#[test]
fn trust_component_baseline_seeded_complete_child_schedule() {
    let mut seed = 0x074c_57a1_u64;
    for sample in 0..512 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let length = [0, 1, 30, 31, 32, 63, 64, 255, 256][sample % 9];
        let mut name = "a".repeat(length);
        if seed & 1 != 0 {
            name.push('名');
        }
        if seed & 2 != 0 {
            name.insert((seed as usize) % (length + 1), '\0');
        }
        let parent = [-1, 0, 17, i32::MAX][(seed >> 8) as usize % 4];
        let fd = [-2, -1, 0, 37][(seed >> 16) as usize % 4];
        let errno = [libc::EINTR, libc::EACCES, libc::ENOENT, libc::EIO][(seed >> 24) as usize % 4];
        check_model(&name, parent, seed & 4 != 0, fd, errno);
    }
    println!("HC074-TRUST-COMPONENT-MODEL seed=0x074c57a1 samples=512 model_only=true");
}

#[test]
fn trust_component_baseline_names_remain_independent_owned_values() {
    for length in [0, 1, 31, 32, 63, 64, 4096] {
        let mut first = "a".repeat(length);
        let outer = CString::new(first.as_str()).unwrap();
        first.push('z');
        let inner = CString::new(first.as_str()).unwrap();
        assert_eq!(outer.as_bytes(), "a".repeat(length).as_bytes());
        assert_eq!(inner.as_bytes(), first.as_bytes());
        drop(inner);
        assert_eq!(outer.as_bytes_with_nul().len(), length + 1);
    }
}
