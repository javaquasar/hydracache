use super::*;
const BOOT: &[u8; 37] = b"e81671d6-bf23-4ebf-b8fe-5dad1f45866f\n";

fn reference_value((raw, count): ([u8; 38], usize)) -> Result<String, KernelError> {
    if count == 0 {
        return Err(kernel_invalid!("proc-document-budget"));
    }
    if count != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(kernel_invalid!("boot-document"));
    }
    String::from_utf8(raw[..36].to_vec()).map_err(|_| kernel_invalid!("boot-utf8"))
}
fn pair(raw: ([u8; 38], usize)) {
    use crate::diagnostic_kernel_admission::refusal_trace::capture;
    let (reference, before) = capture(|| reference_value(raw));
    let (candidate, after) = capture(|| boot_comparison::view(&raw));
    assert_eq!(reference.is_ok(), candidate.is_ok());
    assert_eq!(before.map(|e| e.site), after.map(|e| e.site));
    if let Ok(value) = reference {
        assert_eq!(candidate.unwrap(), value);
        assert_eq!(boot_value(raw).unwrap(), value);
    }
}
#[test]
fn boot_comparison_byte_mutations_lengths_and_original_failure_sites() {
    let mut raw = [0; 38];
    raw[..37].copy_from_slice(BOOT);
    for count in (0..=39).chain([usize::MAX]) {
        pair((raw, count));
    }
    for index in 0..38 {
        for byte in 0..=255 {
            let mut changed = raw;
            changed[index] = byte;
            pair((changed, 37));
        }
    }
}
#[test]
fn boot_comparison_seeded_documents_and_single_read_faults() {
    let mut seed = 0x074c_b036_u64;
    for case in 0..1024 {
        let mut raw = [0; 38];
        for byte in &mut raw {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            *byte = seed as u8;
        }
        pair((raw, case % 40));
    }
    for errno in [libc::EINTR, libc::EAGAIN, libc::EIO, libc::EBADF] {
        let calls = std::cell::Cell::new(0);
        let result = boot_read(|_| {
            calls.set(calls.get() + 1);
            Err(std::io::Error::from_raw_os_error(errno))
        });
        assert_eq!(calls.get(), 1);
        assert!(matches!(result, Err(KernelError::Io(e)) if e.raw_os_error()==Some(errno)));
    }
    println!(
        "HC074-BOOT-COMPARE seed=0x074cb036 documents=1024 mutations=9728 unchanged_read=true"
    );
}
#[test]
fn boot_comparison_real_proc_and_filesystem_failures_match() {
    let origin = Origin::open().unwrap();
    assert!(boot_comparison::matches(&origin.root, &boot(&origin.root).unwrap()).unwrap());
    for wrong in ["", "wrong", "e81671d6-bf23-4ebf-b8fe-5dad1f45866f\n"] {
        assert!(!boot_comparison::matches(&origin.root, wrong).unwrap());
    }
    for file in [
        File::open(tempfile::tempdir().unwrap().path()).unwrap(),
        tempfile::tempfile().unwrap(),
    ] {
        let old = boot(&file).map(|_| ());
        let new = boot_comparison::matches(&file, &origin.boot).map(|_| ());
        assert_eq!(format!("{old:?}"), format!("{new:?}"));
        assert!(new.is_err());
    }
}
#[test]
fn boot_comparison_fresh_trace_counts_and_original_sticky_drift() {
    use crate::diagnostic_ipc::validation_trace::kernel_walk::{capture, Kind};
    let mut origin = Origin::open().unwrap();
    let (result, trace) = capture(|| origin.revalidate());
    result.unwrap();
    for (kind, count) in [
        (Kind::Origin, 1),
        (Kind::Boot, 2),
        (Kind::Document, 2),
        (Kind::Namespace, 4),
    ] {
        assert_eq!(trace.nodes[kind as usize].calls, count);
    }
    let (result, trace) = capture(|| origin.sample());
    result.unwrap();
    for (kind, count) in [
        (Kind::Origin, 2),
        (Kind::Boot, 4),
        (Kind::Document, 4),
        (Kind::Namespace, 8),
        (Kind::Sample, 1),
    ] {
        assert_eq!(trace.nodes[kind as usize].calls, count);
    }
    origin.boot = "wrong".into();
    let (result, refusal) =
        crate::diagnostic_kernel_admission::refusal_trace::capture(|| origin.revalidate());
    assert!(result.is_err());
    assert_eq!(refusal.unwrap().site, "proc-origin-or-boot");
    assert!(origin.sample().is_err());
}
