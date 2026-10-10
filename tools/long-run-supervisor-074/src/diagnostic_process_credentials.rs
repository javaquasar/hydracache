//! Numeric assertions checked against retained kernel status; not NSS enrollment.

use super::{number, Document, ProcessError, ProcessRead, DOCUMENT_BYTES};
use thiserror::Error;

const GROUPS: usize = 32;
const FIELDS: [&[u8]; 11] = [
    b"Pid",
    b"Tgid",
    b"Uid",
    b"Gid",
    b"Groups",
    b"CapInh",
    b"CapPrm",
    b"CapEff",
    b"CapBnd",
    b"CapAmb",
    b"NoNewPrivs",
];

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("asserted worker policy or bounded credential projection is invalid")]
    Invalid,
    #[error("worker credential projection exceeded its bound")]
    Budget,
    #[error("original worker credentials differ from the assertion or drifted")]
    Drift,
    #[error("original credential guard is permanently refused")]
    Refused,
    #[error("original process observation failed: {0}")]
    Process(#[from] ProcessError),
}

/// Numeric caller assertions only: this type authenticates no account name.
pub struct AssertedWorkerCredentials {
    uid: u32,
    gid: u32,
    groups: Vec<u32>,
}
impl AssertedWorkerCredentials {
    pub fn new(uid: u32, gid: u32, groups: Vec<u32>) -> Result<Self, CredentialError> {
        if matches!(uid, 0 | u32::MAX) || matches!(gid, 0 | u32::MAX) {
            return Err(CredentialError::Invalid);
        }
        valid_groups(&groups)?;
        Ok(Self { uid, gid, groups })
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Projection {
    pid: u32,
    tgid: u32,
    uid: [u32; 4],
    gid: [u32; 4],
    groups: Vec<u32>,
    active: [u64; 4],
    bounding: u64,
    no_new_privs: bool,
}
impl Projection {
    fn matches(&self, pid: u32, policy: &AssertedWorkerCredentials) -> Result<(), CredentialError> {
        if self.pid != pid
            || self.tgid != pid
            || self.uid != [policy.uid; 4]
            || self.gid != [policy.gid; 4]
            || self.groups != policy.groups
            || self.active != [0; 4]
            || !self.no_new_privs
        {
            return Err(CredentialError::Drift);
        }
        Ok(())
    }
}

trait CredentialProbe {
    fn process(&self) -> Result<(), CredentialError>;
    fn status(&self) -> Result<Projection, CredentialError>;
}
struct OriginalProbe<'a> {
    process: &'a ProcessRead,
    document: &'a Document,
}
impl CredentialProbe for OriginalProbe<'_> {
    fn process(&self) -> Result<(), CredentialError> {
        self.process.revalidate().map_err(Into::into)
    }
    fn status(&self) -> Result<Projection, CredentialError> {
        parse_status(&self.document.read(&self.process.probe.files.directory)?)
    }
}

struct Gate {
    pid: u32,
    policy: AssertedWorkerCredentials,
    original: Option<Projection>,
    refused: bool,
}
impl Gate {
    fn new(pid: u32, policy: AssertedWorkerCredentials) -> Self {
        Self {
            pid,
            policy,
            original: None,
            refused: false,
        }
    }
    fn observe(&mut self, probe: &impl CredentialProbe) -> Result<(), CredentialError> {
        if self.refused {
            return Err(CredentialError::Refused);
        }
        let result = (|| {
            probe.process()?;
            let before = probe.status()?;
            before.matches(self.pid, &self.policy)?;
            if self
                .original
                .as_ref()
                .is_some_and(|original| original != &before)
            {
                return Err(CredentialError::Drift);
            }
            probe.process()?;
            let after = probe.status()?;
            if before != after {
                return Err(CredentialError::Drift);
            }
            probe.process()?;
            Ok(before)
        })();
        match result {
            Ok(before) => {
                if self.original.is_none() {
                    self.original = Some(before);
                }
                Ok(())
            }
            Err(error) => {
                self.refused = true;
                Err(error)
            }
        }
    }
}

/// Read-only original-process observation; sequential, runtime-only refusal.
pub struct ProcessCredentialRead<'a> {
    process: &'a ProcessRead,
    document: Document,
    gate: Gate,
}
impl ProcessCredentialRead<'_> {
    pub fn revalidate(&mut self) -> Result<(), CredentialError> {
        self.gate.observe(&OriginalProbe {
            process: self.process,
            document: &self.document,
        })
    }
    pub fn is_refused(&self) -> bool {
        self.gate.refused
    }
}

/// Accepts only the already retained original process, never a second PID/path.
pub fn pin_asserted_worker_credentials(
    process: &ProcessRead,
    policy: AssertedWorkerCredentials,
) -> Result<ProcessCredentialRead<'_>, CredentialError> {
    process.revalidate()?;
    let document = Document::open(&process.probe.files.directory, "status")?;
    let mut result = ProcessCredentialRead {
        process,
        document,
        gate: Gate::new(process.observation.expected.pid, policy),
    };
    result.revalidate()?;
    Ok(result)
}

fn valid_groups(groups: &[u32]) -> Result<(), CredentialError> {
    if groups.len() > GROUPS {
        return Err(CredentialError::Budget);
    }
    if groups.contains(&u32::MAX) || groups.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(CredentialError::Invalid);
    }
    Ok(())
}
fn tokens(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes
        .split(|b| matches!(b, b' ' | b'\t'))
        .filter(|part| !part.is_empty())
}
fn id(bytes: &[u8]) -> Result<u32, CredentialError> {
    u32::try_from(number(bytes).map_err(|_| CredentialError::Invalid)?)
        .map_err(|_| CredentialError::Invalid)
}
fn ids(bytes: &[u8]) -> Result<[u32; 4], CredentialError> {
    let mut result = [0; 4];
    let mut input = tokens(bytes);
    for value in &mut result {
        *value = id(input.next().ok_or(CredentialError::Invalid)?)?;
    }
    if input.next().is_some() {
        return Err(CredentialError::Invalid);
    }
    Ok(result)
}
fn single(bytes: &[u8]) -> Result<&[u8], CredentialError> {
    let mut input = tokens(bytes);
    let value = input.next().ok_or(CredentialError::Invalid)?;
    if input.next().is_some() {
        return Err(CredentialError::Invalid);
    }
    Ok(value)
}
fn mask(bytes: &[u8]) -> Result<u64, CredentialError> {
    let value = single(bytes)?;
    if value.len() != 16
        || !value
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err(CredentialError::Invalid);
    }
    value.iter().try_fold(0u64, |sum, byte| {
        sum.checked_mul(16)
            .and_then(|n| {
                n.checked_add(if byte.is_ascii_digit() {
                    (byte - b'0') as u64
                } else {
                    (byte - b'a' + 10) as u64
                })
            })
            .ok_or(CredentialError::Invalid)
    })
}
fn parse_status(bytes: &[u8]) -> Result<Projection, CredentialError> {
    if bytes.len() > DOCUMENT_BYTES {
        return Err(CredentialError::Budget);
    }
    if !bytes.ends_with(b"\n") || bytes.contains(&0) || bytes.contains(&b'\r') {
        return Err(CredentialError::Invalid);
    }
    let mut fields: [Option<&[u8]>; 11] = [None; 11];
    for line in bytes[..bytes.len() - 1].split(|b| *b == b'\n') {
        let Some(colon) = line.iter().position(|b| *b == b':') else {
            continue;
        };
        let key = &line[..colon];
        let Some(index) = FIELDS.iter().position(|field| *field == key) else {
            continue;
        };
        let value = line[colon + 1..]
            .strip_prefix(b"\t")
            .ok_or(CredentialError::Invalid)?;
        if fields[index].replace(value).is_some() {
            return Err(CredentialError::Invalid);
        }
    }
    let values = fields
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(CredentialError::Invalid)?;
    let mut groups = Vec::new();
    for token in tokens(values[4]) {
        if groups.len() == GROUPS {
            return Err(CredentialError::Budget);
        }
        groups.push(id(token)?);
    }
    valid_groups(&groups)?;
    let no_new_privs = match single(values[10])? {
        b"0" => false,
        b"1" => true,
        _ => return Err(CredentialError::Invalid),
    };
    let pid = id(single(values[0])?)?;
    let tgid = id(single(values[1])?)?;
    if pid == 0 || tgid == 0 || pid > i32::MAX as u32 || tgid > i32::MAX as u32 {
        return Err(CredentialError::Invalid);
    }
    Ok(Projection {
        pid,
        tgid,
        uid: ids(values[2])?,
        gid: ids(values[3])?,
        groups,
        active: [
            mask(values[5])?,
            mask(values[6])?,
            mask(values[7])?,
            mask(values[9])?,
        ],
        bounding: mask(values[8])?,
        no_new_privs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_process::pin_owned_test_helper;
    use std::cell::{Cell, RefCell};
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Stdio};

    fn policy() -> AssertedWorkerCredentials {
        AssertedWorkerCredentials::new(1000, 1000, vec![4, 1000]).unwrap()
    }
    fn status() -> Vec<u8> {
        b"Name:\tcat\xff\nTgid:\t17\nPid:\t17\nUid:\t1000\t1000\t1000\t1000\nGid:\t1000\t1000\t1000\t1000\nGroups:\t4 1000 \nCapInh:\t0000000000000000\nCapPrm:\t0000000000000000\nCapEff:\t0000000000000000\nCapBnd:\t000001ffffffffff\nCapAmb:\t0000000000000000\nNoNewPrivs:\t1\n".to_vec()
    }
    fn replace(bytes: &[u8], key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            if line.starts_with(key) {
                out.extend_from_slice(key);
                out.extend_from_slice(value);
                out.push(b'\n');
            } else {
                out.extend_from_slice(line);
            }
        }
        out
    }
    #[test]
    fn credential_policy_is_nonroot_canonical_and_bounded() {
        for (uid, gid, groups) in [
            (0, 1000, vec![]),
            (1000, 0, vec![]),
            (u32::MAX, 1000, vec![]),
            (1000, u32::MAX, vec![]),
            (1000, 1000, vec![4, 4]),
            (1000, 1000, vec![1000, 4]),
            (1000, 1000, vec![u32::MAX]),
            (1000, 1000, (0..33).collect()),
        ] {
            assert!(AssertedWorkerCredentials::new(uid, gid, groups).is_err());
        }
        AssertedWorkerCredentials::new(1000, 1000, vec![]).unwrap();
        AssertedWorkerCredentials::new(1000, 1000, (0..32).collect()).unwrap();
    }
    #[test]
    fn status_projection_ignores_nonutf8_name_but_checks_all_credentials() {
        let selected = parse_status(&status()).unwrap();
        selected.matches(17, &policy()).unwrap();
        assert_ne!(selected.bounding, 0);
        let empty = replace(&status(), b"Groups:\t", b"");
        parse_status(&empty)
            .unwrap()
            .matches(
                17,
                &AssertedWorkerCredentials::new(1000, 1000, vec![]).unwrap(),
            )
            .unwrap();
    }
    #[test]
    fn status_projection_refuses_missing_duplicate_and_malformed_fields() {
        for key in FIELDS {
            let base = status();
            let line = base
                .split_inclusive(|b| *b == b'\n')
                .find(|line| line.starts_with(key) && line[key.len()] == b':')
                .unwrap();
            let missing = base
                .split_inclusive(|b| *b == b'\n')
                .filter(|candidate| *candidate != line)
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            assert!(parse_status(&missing).is_err(), "missing {key:?}");
            let mut duplicate = base.clone();
            duplicate.extend_from_slice(line);
            assert!(parse_status(&duplicate).is_err(), "duplicate {key:?}");
            let mut bad_key = key.to_vec();
            bad_key.extend_from_slice(b":\t");
            assert!(
                parse_status(&replace(&base, &bad_key, b"?")).is_err(),
                "bad {key:?}"
            );
        }
        let mut truncated = status();
        truncated.pop();
        assert!(parse_status(&truncated).is_err());
        assert!(parse_status(&[b'X'; DOCUMENT_BYTES + 1]).is_err());
        assert!(parse_status(&replace(&status(), b"Uid:\t", b"1000\r1000 1000 1000")).is_err());
    }
    #[test]
    fn status_numeric_tokens_and_group_limits_fail_closed() {
        for bad in [
            b"01".as_slice(),
            b"+1",
            b"-1",
            b"4294967296",
            b"18446744073709551616",
            b"",
            b"1 2",
            b"1\xff",
        ] {
            assert!(parse_status(&replace(&status(), b"Pid:\t", bad)).is_err());
        }
        for bad in ["4 4", "1000 4", "4294967295", "4294967296", "04 1000"] {
            assert!(parse_status(&replace(&status(), b"Groups:\t", bad.as_bytes())).is_err());
        }
        let groups = (0..33).map(|n| n.to_string()).collect::<Vec<_>>().join(" ");
        assert!(parse_status(&replace(&status(), b"Groups:\t", groups.as_bytes())).is_err());
        for bad in [
            "0",
            "000000000000000A",
            "00000000000000000",
            "-000000000000001",
        ] {
            assert!(parse_status(&replace(&status(), b"CapEff:\t", bad.as_bytes())).is_err());
        }
        assert!(parse_status(&replace(&status(), b"Uid:\t", b"1000 1000 1000")).is_err());
        assert!(parse_status(&replace(&status(), b"Gid:\t", b"1000 1000 1000 1000 1000")).is_err());
    }
    #[test]
    fn every_uid_gid_slot_and_leader_identity_are_checked() {
        for key in [b"Uid:\t".as_slice(), b"Gid:\t"] {
            for slot in 0..4 {
                let mut ids = [1000; 4];
                ids[slot] = 1001;
                let value = ids.map(|n| n.to_string()).join(" ");
                assert!(parse_status(&replace(&status(), key, value.as_bytes()))
                    .unwrap()
                    .matches(17, &policy())
                    .is_err());
            }
        }
        for key in [b"Pid:\t".as_slice(), b"Tgid:\t", b"Groups:\t"] {
            assert!(parse_status(&replace(&status(), key, b"18"))
                .unwrap()
                .matches(17, &policy())
                .is_err());
        }
    }
    #[test]
    fn active_capabilities_and_missing_no_new_privs_refuse() {
        for key in [
            b"CapInh:\t".as_slice(),
            b"CapPrm:\t",
            b"CapEff:\t",
            b"CapAmb:\t",
        ] {
            assert!(parse_status(&replace(&status(), key, b"0000000000000001"))
                .unwrap()
                .matches(17, &policy())
                .is_err());
        }
        assert!(parse_status(&replace(&status(), b"NoNewPrivs:\t", b"0"))
            .unwrap()
            .matches(17, &policy())
            .is_err());
        assert!(parse_status(&replace(&status(), b"NoNewPrivs:\t", b"2")).is_err());
    }
    struct Fake {
        calls: Cell<usize>,
        fail: Cell<Option<usize>>,
        order: RefCell<Vec<&'static str>>,
        documents: RefCell<Vec<Vec<u8>>>,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                calls: Cell::new(0),
                fail: Cell::new(None),
                order: RefCell::new(vec![]),
                documents: RefCell::new(vec![]),
            }
        }
        fn step(&self, name: &'static str) -> Result<(), CredentialError> {
            let n = self.calls.get();
            self.calls.set(n + 1);
            self.order.borrow_mut().push(name);
            if self.fail.get() == Some(n) {
                Err(CredentialError::Invalid)
            } else {
                Ok(())
            }
        }
    }
    impl CredentialProbe for Fake {
        fn process(&self) -> Result<(), CredentialError> {
            self.step("process")
        }
        fn status(&self) -> Result<Projection, CredentialError> {
            self.step("status")?;
            let mut docs = self.documents.borrow_mut();
            parse_status(&if docs.is_empty() {
                status()
            } else {
                docs.remove(0)
            })
        }
    }
    #[test]
    fn credential_observation_order_and_bounding_drift_are_sticky() {
        let fake = Fake::new();
        let mut gate = Gate::new(17, policy());
        gate.observe(&fake).unwrap();
        assert_eq!(
            *fake.order.borrow(),
            ["process", "status", "process", "status", "process"]
        );
        fake.documents
            .borrow_mut()
            .push(replace(&status(), b"CapBnd:\t", b"0000000000000000"));
        assert!(matches!(gate.observe(&fake), Err(CredentialError::Drift)));
        let calls = fake.calls.get();
        assert!(matches!(gate.observe(&fake), Err(CredentialError::Refused)));
        assert_eq!(fake.calls.get(), calls);
    }
    #[test]
    fn every_observation_failure_stops_and_preserves_first_error() {
        for failure in 0..5 {
            let fake = Fake::new();
            let mut gate = Gate::new(17, policy());
            gate.observe(&fake).unwrap();
            fake.fail.set(Some(5 + failure));
            assert!(matches!(gate.observe(&fake), Err(CredentialError::Invalid)));
            assert!(gate.refused);
            assert_eq!(fake.calls.get(), 6 + failure);
            fake.fail.set(None);
            assert!(matches!(gate.observe(&fake), Err(CredentialError::Refused)));
            assert_eq!(fake.calls.get(), 6 + failure);
        }
    }
    #[test]
    fn second_status_drift_cannot_update_original_projection() {
        let fake = Fake::new();
        let mut gate = Gate::new(17, policy());
        fake.documents.borrow_mut().extend([
            status(),
            replace(&status(), b"CapBnd:\t", b"0000000000000000"),
        ]);
        assert!(matches!(gate.observe(&fake), Err(CredentialError::Drift)));
        assert!(gate.original.is_none());
        assert!(gate.refused);
    }
    #[test]
    fn seeded_single_field_mutations_never_adopt_changed_credentials() {
        let seed = 0x074c_2026_u64;
        eprintln!("credential mutation seed={seed:#x}");
        let mut rng = seed;
        for _ in 0..256 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let key = FIELDS[rng as usize % FIELDS.len()];
            let mut prefix = key.to_vec();
            prefix.extend_from_slice(b":\t");
            let value: &[u8] = match key {
                b"Pid" | b"Tgid" => b"18",
                b"Uid" | b"Gid" => b"1000 1001 1000 1000",
                b"Groups" => b"4 1001",
                b"NoNewPrivs" => b"0",
                b"CapBnd" => b"0000000000000000",
                _ => b"0000000000000001",
            };
            let changed = replace(&status(), &prefix, value);
            parse_status(&changed).unwrap();
            let fake = Fake::new();
            let mut gate = Gate::new(17, policy());
            gate.observe(&fake).unwrap();
            fake.documents.borrow_mut().push(changed);
            assert!(matches!(gate.observe(&fake), Err(CredentialError::Drift)));
            assert!(gate.refused);
        }
    }
    #[test]
    fn status_zero_leader_nul_separator_and_full_group_boundary_refuse_or_bind_exactly() {
        for key in [b"Pid:\t".as_slice(), b"Tgid:\t"] {
            for value in [b"0".as_slice(), b"2147483648"] {
                assert!(parse_status(&replace(&status(), key, value)).is_err());
            }
        }
        assert!(parse_status(&replace(&status(), b"Uid:\t", b"1000\0 1000 1000 1000")).is_err());
        let mut malformed = status();
        let tab = malformed.windows(5).position(|w| w == b"Pid:\t").unwrap() + 4;
        malformed[tab] = b' ';
        assert!(parse_status(&malformed).is_err());
        let groups = (0..32).map(|n| n.to_string()).collect::<Vec<_>>().join(" ");
        parse_status(&replace(&status(), b"Groups:\t", groups.as_bytes()))
            .unwrap()
            .matches(
                17,
                &AssertedWorkerCredentials::new(1000, 1000, (0..32).collect()).unwrap(),
            )
            .unwrap();
        let all_bits = replace(&status(), b"CapBnd:\t", b"ffffffffffffffff");
        parse_status(&all_bits)
            .unwrap()
            .matches(17, &policy())
            .unwrap();
        assert!(parse_status(&[]).is_err());
    }
    #[test]
    fn independent_credential_guards_keep_refusal_and_assertions_separate() {
        let fake = Fake::new();
        let mut first = Gate::new(17, policy());
        let mut second = Gate::new(17, policy());
        first.observe(&fake).unwrap();
        second.observe(&fake).unwrap();
        fake.documents
            .borrow_mut()
            .push(replace(&status(), b"Gid:\t", b"1000 1000 1001 1000"));
        assert!(first.observe(&fake).is_err());
        second.observe(&fake).unwrap();
        assert!(first.refused);
        assert!(!second.refused);
    }
    struct Owned(Child);
    impl Owned {
        fn new(nnp: bool) -> Self {
            let mut command = Command::new("/bin/cat");
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if nnp {
                // SAFETY: pre_exec calls only async-signal-safe prctl; no allocation.
                unsafe {
                    command.pre_exec(|| {
                        if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                        Ok(())
                    });
                }
            }
            Self(command.spawn().unwrap())
        }
        fn finish(&mut self) {
            self.0.stdin.take();
            self.0.wait().unwrap();
        }
    }
    impl Drop for Owned {
        fn drop(&mut self) {
            self.0.stdin.take();
            let _ = self.0.wait();
        }
    }
    fn actual_policy() -> AssertedWorkerCredentials {
        // SAFETY: scalar identity queries and properly bounded writable group buffer.
        let (uid, gid, count) = unsafe {
            (
                libc::geteuid(),
                libc::getegid(),
                libc::getgroups(0, std::ptr::null_mut()),
            )
        };
        assert!((0..=32).contains(&count));
        let mut groups = vec![0; count as usize];
        assert_eq!(
            unsafe { libc::getgroups(count, groups.as_mut_ptr()) },
            count
        );
        groups.sort_unstable();
        AssertedWorkerCredentials::new(uid, gid, groups).unwrap()
    }
    #[test]
    fn owned_no_new_privs_helper_binds_original_status_and_parallel_reads() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut guard = pin_asserted_worker_credentials(&process, actual_policy()).unwrap();
        for _ in 0..5 {
            guard.revalidate().unwrap();
        }
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    let mut independent =
                        pin_asserted_worker_credentials(&process, actual_policy()).unwrap();
                    for _ in 0..5 {
                        independent.revalidate().unwrap();
                    }
                });
            }
        });
        assert!(!guard.is_refused());
    }
    #[test]
    fn owned_helper_wrong_assertions_and_unhardened_status_refuse() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut wrong = actual_policy();
        wrong.uid += 1;
        assert!(pin_asserted_worker_credentials(&process, wrong).is_err());
        let child = Owned::new(false);
        let process = pin_owned_test_helper(child.0.id());
        // Refusal is asserted only when the parent has not already inherited NNP.
        // This local test requires that baseline; it does not silently skip it.
        assert_eq!(
            unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) },
            0
        );
        assert!(pin_asserted_worker_credentials(&process, actual_policy()).is_err());
    }
    #[test]
    fn original_helper_exit_permanently_refuses_without_replacement() {
        let mut child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut guard = pin_asserted_worker_credentials(&process, actual_policy()).unwrap();
        child.finish();
        assert!(matches!(
            guard.revalidate(),
            Err(CredentialError::Process(
                super::super::ProcessError::NotLive
            ))
        ));
        assert!(guard.is_refused());
        assert!(matches!(guard.revalidate(), Err(CredentialError::Refused)));
        assert!(matches!(
            pin_asserted_worker_credentials(&process, actual_policy()),
            Err(CredentialError::Process(
                super::super::ProcessError::NotLive
            ))
        ));
    }

    #[test]
    fn original_status_descriptor_substitution_latches_even_after_restoration() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut guard = pin_asserted_worker_credentials(&process, actual_policy()).unwrap();
        let replacement = std::fs::File::open("/dev/null").unwrap();
        let original = std::mem::replace(&mut guard.document.file, replacement);
        assert!(matches!(
            guard.revalidate(),
            Err(CredentialError::Process(ProcessError::Drift))
        ));
        guard.document.file = original;
        assert!(matches!(guard.revalidate(), Err(CredentialError::Refused)));
    }
}
