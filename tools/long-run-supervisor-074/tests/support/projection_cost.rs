//! Complete original Sender with one selectable projection representation.
use crate::diagnostic_kernel_admission::origin::{alive_read, DocumentBuffer};
use crate::diagnostic_kernel_admission::*;
#[path = "projection_candidate.rs"]
mod candidate;
#[path = "projection_reference.rs"]
mod reference;
impl Sender {
    pub(in crate::diagnostic_kernel_admission) fn projection_control(
        &self,
        origin: &Origin,
        parse: fn(&[u8], &libc::ucred) -> Result<Projection, KernelError>,
    ) -> Result<(), KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Sender,
        );
        origin.revalidate()?;
        let mut scratch = DocumentBuffer::new();
        alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;
        let directory = origin.child(self.credentials.pid, true)?;
        for file in [&directory, &self.directory] {
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.directory_id {
                return Err(kernel_invalid!("sender-directory"));
            }
        }
        let named = origin::fixed_child(&self.directory, c"status", false)?;
        for file in [&named, &self.status] {
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.status_id
                || parse(scratch.read(file)?, &self.credentials)? != self.projection
            {
                return Err(kernel_invalid!("sender-status-binding"));
            }
        }
        alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;
        origin.revalidate()
    }
}
pub(in crate::diagnostic_kernel_admission) fn compare(
    sender: &Sender,
    origin: &Origin,
    candidate: bool,
) -> Result<(), KernelError> {
    sender.projection_control(
        origin,
        if candidate {
            candidate::projection
        } else {
            reference::projection
        },
    )
}
fn outcome(
    run: impl FnOnce() -> Result<Projection, KernelError>,
) -> (String, Option<&'static str>) {
    let (value, event) = refusal_trace::capture(run);
    let value = match value {
        Ok(value) => format!("{value:?}"),
        Err(KernelError::Io(error)) => format!("io:{:?}", error.raw_os_error()),
        Err(error) => error.to_string(),
    };
    (value, event.map(|value| value.site))
}
#[test]
fn projection_fields_differential_seeded_boundaries_duplicates_and_error_order() {
    let credentials = libc::ucred {
        pid: 74,
        uid: 1000,
        gid: 1000,
    };
    let fields = ["Pid:", "Tgid:", "Uid:", "Gid:", "Groups:"];
    let healthy = [
        "74",
        "74",
        "1000 1000 1000 1000",
        "1000 1000 1000 1000",
        "1 2 1000",
    ];
    let document = |values: &[String; 5]| {
        fields
            .iter()
            .zip(values)
            .map(|(name, value)| format!("{name}\t{value}\n"))
            .collect::<String>()
            .into_bytes()
    };
    let mut cases = Vec::new();
    let healthy = healthy.map(str::to_owned);
    cases.push(document(&healthy));
    for index in 0..5 {
        for width in 0..=34 {
            for defect in ["none", "overflow", "invalid"] {
                let mut values = healthy.clone();
                let mut words = vec!["1000"; width];
                if let Some(last) = words.last_mut() {
                    *last = match defect {
                        "overflow" => "4294967296",
                        "invalid" => "-1",
                        _ => "1000",
                    };
                }
                values[index] = words.join(" ");
                cases.push(document(&values));
            }
        }
        let mut raw = document(&healthy);
        raw.extend_from_slice(format!("{}\t{}\n", fields[index], healthy[index]).as_bytes());
        cases.push(raw);
        let mut values = healthy.clone();
        values[index] = "bad 4294967296".into();
        // Multiple defects must retain the first original refusal.
        values[(index + 1) % 5] = "".into();
        cases.push(document(&values));
        let raw = String::from_utf8(document(&healthy)).unwrap();
        cases.push(
            raw.lines()
                .filter(|line| !line.starts_with(fields[index]))
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes(),
        );
    }
    for groups in [
        "",
        "0",
        "1 1",
        "2 1",
        "0 4294967295",
        "  +1\t+2\r",
        "1\u{00a0}2",
    ] {
        let mut values = healthy.clone();
        values[4] = groups.into();
        cases.push(document(&values));
    }
    let mut seed = 0x074f_1e1d_u64;
    for _ in 0..256 {
        let mut bytes = document(&healthy);
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let index = seed as usize % bytes.len();
        bytes[index] = (seed >> 32) as u8;
        cases.push(bytes);
    }
    for (case, bytes) in cases.iter().enumerate() {
        assert_eq!(
            outcome(|| reference::projection(bytes, &credentials)),
            outcome(|| candidate::projection(bytes, &credentials)),
            "seed=0x074f1e1d case={case}"
        );
    }
    println!(
        "HC074-PROJECTION-DIFFERENTIAL cases={} seed=0x074f1e1d server=false product=false",
        cases.len()
    );
}
#[test]
fn projection_fields_complete_sender_preserves_reads_counts_and_original_refusals() {
    use crate::diagnostic_ipc::validation_trace::kernel_walk;
    let (origin, sender) = super::document_scratch::current();
    let (old, a) = kernel_walk::capture(|| compare(&sender, &origin, false));
    let (new, b) = kernel_walk::capture(|| compare(&sender, &origin, true));
    old.unwrap();
    new.unwrap();
    assert_eq!(a.nodes.map(|s| s.calls), [0, 1, 2, 2, 2, 4, 8, 0, 0, 8]);
    assert_eq!(a.nodes.map(|s| s.calls), b.nodes.map(|s| s.calls));
    for case in 0..6 {
        let (origin, mut sender) = super::document_scratch::current();
        match case {
            0 => sender.credentials.pid += 1,
            1 => sender.credentials.uid ^= 1,
            2 => sender.directory_id.0 ^= 1,
            3 => sender.status_id.1 ^= 1,
            4 => sender.projection.groups.push(u32::MAX),
            _ => sender.pidfd = tempfile::tempfile().unwrap().into(),
        }
        let run = |candidate| {
            let (value, event) = refusal_trace::capture(|| compare(&sender, &origin, candidate));
            (
                value.err().map(|error| error.to_string()),
                event.map(|event| event.site),
            )
        };
        let old = run(false);
        assert!(old.0.is_some());
        assert_eq!(old, run(true), "case={case}");
    }
}
#[test]
fn projection_fields_runtime_binding_is_exact_frozen_candidate() {
    let runtime = include_str!("../../src/diagnostic_kernel_admission.rs").replace('\r', "");
    let runtime =
        crate::diagnostic_kernel_admission::projection_fields_tests::legacy_source(&runtime);
    let body = include_str!("projection_candidate.rs").replace('\r', "");
    let body = body
        .split_once(") -> Result<Projection, KernelError> {")
        .unwrap()
        .1;
    assert!(
        runtime.contains(&format!("fn projection(bytes: &[u8], credentials: &libc::ucred) -> Result<Projection, KernelError> {{{}", body.trim_end())),
        "test-first projection candidate not adopted"
    );
}
