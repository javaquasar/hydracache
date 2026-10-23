//! Differential oracle is the exact separately retained pre-candidate source.
use super::*;

pub(in crate::diagnostic_kernel_admission) fn legacy_source(raw: &str) -> String {
    let raw = raw.replace('\r', "");
    assert_eq!(
        crate::sha256_hex(raw.as_bytes()),
        "f5a362becbccfe9575a36d08362109bf007e757265ae05a84d01b2fcfffdea93"
    );
    assert_eq!(
        crate::sha256_hex(
            include_str!("diagnostic_projection_fields.rs")
                .replace('\r', "")
                .as_bytes()
        ),
        "0bd6b4246b7d14151f378fda0090523fdc3485be254cc05df33eaa97264cd6ae"
    );
    let mut result = raw.replacen("#[path = \"diagnostic_projection_fields.rs\"]\nmod projection_fields;\n", "", 1)
        .replacen("\n#[cfg(test)]\n#[path = \"diagnostic_projection_fields_tests.rs\"]\nmod projection_fields_tests;\n", "", 1);
    let start = result
        .find("#[derive(Debug, PartialEq, Eq)]\nstruct Projection {")
        .unwrap();
    let end = result[start..].find("\nstruct Sender {").unwrap() + start;
    let old = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/testing/performance/0.74/local-runs/status-field-scan-baseline-20261024/objects/fb77cd5dae690c838652f358e7520d7d454ba202b8526bfd1678ca0315d20b97"));
    result.replace_range(start..end, old.trim_end());
    assert_eq!(
        crate::sha256_hex(result.as_bytes()),
        "5f77f86c3c29a42156d63ffa913583f8a8c6419562f6c2bea5c34e75937260fa"
    );
    // This older binding predates the separately accepted inline group owner.
    // Restore that one exact expression only after both complete files matched.
    let group = ".collect::<Result<projection_groups::Values, _>>()?\n            .into_vec();";
    assert_eq!(result.matches(group).count(), 1);
    result.replacen(group, ".collect::<Result<Vec<_>, _>>()?;", 1)
}

mod baseline {
    use super::*;
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/testing/performance/0.74/local-runs/status-field-scan-baseline-20261024/objects/fb77cd5dae690c838652f358e7520d7d454ba202b8526bfd1678ca0315d20b97"));
    pub(super) fn run(bytes: &[u8], credentials: &libc::ucred) -> Result<String, KernelError> {
        projection(bytes, credentials).map(|value| format!("{value:?}"))
    }
}
fn compare(bytes: &[u8], credentials: libc::ucred) {
    use crate::diagnostic_kernel_admission::refusal_trace::capture;
    let (old, old_event) = capture(|| baseline::run(bytes, &credentials));
    let (new, new_event) =
        capture(|| projection(bytes, &credentials).map(|value| format!("{value:?}")));
    match (old, new) {
        (Ok(a), Ok(b)) => assert_eq!(a, b),
        (Err(KernelError::Invalid), Err(KernelError::Invalid)) => (),
        pair => panic!("different parser outcomes for {bytes:?}: {pair:?}"),
    }
    assert_eq!(
        old_event.map(|e| e.site),
        new_event.map(|e| e.site),
        "{bytes:?}"
    );
}
fn credentials() -> libc::ucred {
    libc::ucred {
        pid: 4242,
        uid: 1000,
        gid: 1001,
    }
}
fn lines() -> Vec<String> {
    [
        "Pid:\t4242",
        "Tgid:\t4242",
        "Uid:\t1000 1000 1000 1000",
        "Gid:\t1001 1001 1001 1001",
        "Groups:\t4 27 1001",
    ]
    .map(str::to_owned)
    .into()
}
#[test]
fn status_field_scan_differential_first_refusal_seeded_order_duplicates_and_shapes() {
    let mut seed = 0x0740_5ca1_u64;
    for case in 0..4096 {
        let mut fields = lines();
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let slot = seed as usize % 5;
        match case % 8 {
            0 => {
                fields.remove(slot);
            }
            1 => fields.push(fields[slot].clone()),
            2 => fields[slot].push_str(" invalid"),
            3 => fields[slot] = fields[slot].split(':').next().unwrap().to_owned() + ":",
            4 => fields[slot].push_str(" 4294967296"),
            5 => {
                fields.push("Groups: invalid".into());
                fields[0] = "Pid: bad".into();
            }
            6 => fields.push("Ignored: é Pid: 55 Uid: bad".into()),
            _ => (),
        }
        for index in (1..fields.len()).rev() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            fields.swap(index, seed as usize % (index + 1));
        }
        let separator = if case % 3 == 0 { "\r\n" } else { "\n" };
        let bytes = fields.join(separator);
        compare(bytes.as_bytes(), credentials());
        let mut bytes = bytes.into_bytes();
        if case % 17 == 0 {
            bytes.insert(0, 0xff);
        }
        compare(&bytes, credentials());
    }
}
#[test]
fn status_field_scan_keeps_token_caps_credentials_unknown_lines_and_live_trace_count() {
    for field in 0..5 {
        for count in 0..=35 {
            for suffix in ["", " invalid", "\0"] {
                let mut data = lines();
                let prefix = data[field].split(':').next().unwrap().to_owned();
                data[field] = prefix + ": " + &vec!["1"; count].join(" ") + suffix;
                compare(data.join("\n").as_bytes(), credentials());
            }
        }
    }
    let good = lines().join("\n");
    for credential in [
        libc::ucred {
            pid: -1,
            ..credentials()
        },
        libc::ucred {
            uid: 0,
            ..credentials()
        },
        libc::ucred {
            gid: 0,
            ..credentials()
        },
        credentials(),
    ] {
        compare(good.as_bytes(), credential);
    }
    for tail in [
        "\nPidExtra: bad",
        "\nGroupsExtra: 1",
        "\npid: 0",
        "\n Pid: 0",
        "\n\0Pid: 0",
    ] {
        compare((good.clone() + tail).as_bytes(), credentials());
    }
    let (result, report) = crate::diagnostic_ipc::validation_trace::kernel_walk::capture(|| {
        projection(good.as_bytes(), &credentials())
    });
    result.unwrap();
    assert_eq!(report.nodes[3].calls, 1);
    assert_eq!(report.nodes.iter().map(|node| node.calls).sum::<u64>(), 1);
}
