//! Fixed-name NSS observations, never trusted host enrollment or launch authority.

use super::{failure, restrict_worker_resources, ManagerClient, WorkerFailure, WorkerFailureKind};
use super::{OPERATION_TIMEOUT, REQUEST_BYTES};
use crate::canonical_json;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::process::Command;

const ACCOUNT: &str = "hydracache-perf";
const REQUEST: &[u8] = br#"{"schema_version":1}"#;
const SNAPSHOT_BYTES: usize = 4096;
const NSS_BYTES: usize = 16_384;
const MEMBERSHIPS: usize = 32;

#[path = "diagnostic_worker_binding.rs"]
mod binding;
pub use binding::{bind_asserted_worker, AssertedWorkerBindingRead, WorkerBindingError};

/// Local NSS projection only: not authenticated host policy or process credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkerAccountSnapshot {
    schema_version: u32,
    account: String,
    group: String,
    uid: u32,
    gid: u32,
    membership_gids: Vec<u32>,
}
impl WorkerAccountSnapshot {
    fn decode(bytes: &[u8]) -> Result<Self, ()> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            schema_version: u32,
            account: String,
            group: String,
            uid: u32,
            gid: u32,
            membership_gids: Vec<u32>,
        }
        if bytes.is_empty() || bytes.len() > SNAPSHOT_BYTES {
            return Err(());
        }
        let wire: Wire = serde_json::from_slice(bytes).map_err(|_| ())?;
        if wire.schema_version != 1
            || wire.account != ACCOUNT
            || wire.group != ACCOUNT
            || !nonroot_id(wire.uid)
            || !nonroot_id(wire.gid)
            || canonical_memberships(wire.membership_gids.clone(), wire.gid)?
                != wire.membership_gids
        {
            return Err(());
        }
        let snapshot = Self {
            schema_version: wire.schema_version,
            account: wire.account,
            group: wire.group,
            uid: wire.uid,
            gid: wire.gid,
            membership_gids: wire.membership_gids,
        };
        if canonical_json(&snapshot).map_err(|_| ())? != bytes {
            return Err(());
        }
        Ok(snapshot)
    }
}

/// Original account observation; any failure latches refusal, with no refresh.
pub struct WorkerAccountRead {
    original: WorkerAccountSnapshot,
    refused: bool,
}
impl WorkerAccountRead {
    fn new(original: WorkerAccountSnapshot) -> Self {
        Self {
            original,
            refused: false,
        }
    }
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    pub(crate) fn refuse(&mut self) {
        self.refused = true;
    }
    /// Consistency of caller assertions only; no values are exported or adopted.
    pub(crate) fn matches_assertions(&self, uid: u32, gid: u32, groups: &[u32]) -> bool {
        if self.refused
            || uid != self.original.uid
            || gid != self.original.gid
            || groups.len() > MEMBERSHIPS
            || groups.iter().any(|group| !nonroot_id(*group))
            || groups.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return false;
        }
        // Exactly the asserted list, or that list without the separately checked
        // primary GID. Do not allocate a union or infer kernel supplementary IDs.
        groups == self.original.membership_gids.as_slice()
            || groups.iter().copied().eq(self
                .original
                .membership_gids
                .iter()
                .copied()
                .filter(|group| *group != gid))
    }
    pub fn revalidate(&mut self, client: &mut ManagerClient) -> Result<(), WorkerFailure> {
        if self.refused {
            return Err(WorkerFailure {
                cleanup_confirmed: client.pending.is_none(),
                ..failure(WorkerFailureKind::Invalid)
            });
        }
        self.observe(|| client.inspect_worker_account())
    }
    fn observe(
        &mut self,
        read: impl FnOnce() -> Result<WorkerAccountSnapshot, WorkerFailure>,
    ) -> Result<(), WorkerFailure> {
        if self.refused {
            return Err(failure(WorkerFailureKind::Invalid));
        }
        match read() {
            Ok(snapshot) if snapshot == self.original => Ok(()),
            Ok(snapshot) => {
                self.refused = true;
                Err(WorkerFailure {
                    stdout: canonical_json(&snapshot).unwrap_or_default(),
                    ..failure(WorkerFailureKind::Invalid)
                })
            }
            Err(error) => {
                self.refused = true;
                Err(error)
            }
        }
    }
}
impl ManagerClient {
    /// No selectors or retries; NSS calls execute only in the bounded helper.
    pub fn inspect_worker_account(&mut self) -> Result<WorkerAccountSnapshot, WorkerFailure> {
        let mut command = Command::new("/proc/self/exe");
        command.arg("diagnostic-worker-account-worker");
        let stdout = self.run(command, REQUEST, OPERATION_TIMEOUT)?;
        WorkerAccountSnapshot::decode(&stdout).map_err(|_| WorkerFailure {
            stdout,
            ..failure(WorkerFailureKind::Invalid)
        })
    }
    pub fn pin_worker_account(&mut self) -> Result<WorkerAccountRead, WorkerFailure> {
        self.inspect_worker_account().map(WorkerAccountRead::new)
    }
}

pub fn account_worker_main() -> u8 {
    let result = (|| -> Result<(), ()> {
        restrict_worker_resources().map_err(|_| ())?;
        let mut request = Vec::new();
        std::io::stdin()
            .take((REQUEST_BYTES + 1) as u64)
            .read_to_end(&mut request)
            .map_err(|_| ())?;
        check_request(&request)?;
        let bytes = canonical_json(&capture(&LibcLookup)?).map_err(|_| ())?;
        if bytes.len() > SNAPSHOT_BYTES {
            return Err(());
        }
        std::io::stdout().write_all(&bytes).map_err(|_| ())
    })();
    if result.is_ok() {
        0
    } else {
        eprintln!("diagnostic worker account observation refused");
        9
    }
}
fn check_request(bytes: &[u8]) -> Result<(), ()> {
    if bytes == REQUEST {
        Ok(())
    } else {
        Err(())
    }
}
fn nonroot_id(id: u32) -> bool {
    id != 0 && id != u32::MAX
}
#[derive(Clone)]
struct User {
    name: Vec<u8>,
    uid: u32,
    gid: u32,
}
#[derive(Clone)]
struct Group {
    name: Vec<u8>,
    gid: u32,
}
trait Lookup {
    fn user(&self, uid: Option<u32>) -> Result<User, ()>;
    fn group(&self, gid: Option<u32>) -> Result<Group, ()>;
    fn memberships(&self, gid: u32) -> Result<Vec<u32>, ()>;
}
struct LibcLookup;
impl Lookup for LibcLookup {
    fn user(&self, uid: Option<u32>) -> Result<User, ()> {
        let mut buffer = [0u8; NSS_BYTES];
        // SAFETY: zero initialization is valid for this C pointer/integer record.
        let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
        let mut result = std::ptr::null_mut();
        // SAFETY: writable record, buffer and result pointer live through the call;
        // fixed NUL-terminated name, no returned strings are dereferenced here.
        let rc = unsafe {
            match uid {
                Some(uid) => libc::getpwuid_r(
                    uid,
                    &mut entry,
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                ),
                None => libc::getpwnam_r(
                    c"hydracache-perf".as_ptr(),
                    &mut entry,
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                ),
            }
        };
        check_result(rc, &mut entry, result)?;
        Ok(User {
            name: bounded_name(&buffer, entry.pw_name)?,
            uid: entry.pw_uid,
            gid: entry.pw_gid,
        })
    }
    fn group(&self, gid: Option<u32>) -> Result<Group, ()> {
        let mut buffer = [0u8; NSS_BYTES];
        // SAFETY: zero initialization is valid for this C pointer/integer record.
        let mut entry: libc::group = unsafe { std::mem::zeroed() };
        let mut result = std::ptr::null_mut();
        // SAFETY: caller-owned record/buffer/result and fixed C string are valid.
        let rc = unsafe {
            match gid {
                Some(gid) => libc::getgrgid_r(
                    gid,
                    &mut entry,
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                ),
                None => libc::getgrnam_r(
                    c"hydracache-perf".as_ptr(),
                    &mut entry,
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                ),
            }
        };
        check_result(rc, &mut entry, result)?;
        Ok(Group {
            name: bounded_name(&buffer, entry.gr_name)?,
            gid: entry.gr_gid,
        })
    }
    fn memberships(&self, gid: u32) -> Result<Vec<u32>, ()> {
        let mut groups = [0u32; MEMBERSHIPS];
        let mut count = MEMBERSHIPS as libc::c_int;
        // SAFETY: count is the exact writable array capacity; fixed account only.
        // NSS-internal allocation is not bounded by this caller array.
        let rc = unsafe {
            libc::getgrouplist(
                c"hydracache-perf".as_ptr(),
                gid,
                groups.as_mut_ptr(),
                &mut count,
            )
        };
        let count = membership_count(rc, count)?;
        canonical_memberships(groups[..count].to_vec(), gid)
    }
}
fn check_result<T>(rc: i32, expected: *mut T, actual: *mut T) -> Result<(), ()> {
    if rc == 0 && !actual.is_null() && actual == expected {
        Ok(())
    } else {
        Err(())
    }
}
fn bounded_name(buffer: &[u8], name: *const libc::c_char) -> Result<Vec<u8>, ()> {
    // Only use the address to locate a slice; never dereference an NSS pointer.
    let offset = (name as usize)
        .checked_sub(buffer.as_ptr() as usize)
        .ok_or(())?;
    let suffix = buffer.get(offset..).ok_or(())?;
    let end = suffix.iter().position(|byte| *byte == 0).ok_or(())?;
    Ok(suffix[..end].to_vec())
}
fn membership_count(rc: i32, count: i32) -> Result<usize, ()> {
    if count > 0 && count as usize <= MEMBERSHIPS && rc == count {
        Ok(count as usize)
    } else {
        Err(())
    }
}
fn canonical_memberships(mut groups: Vec<u32>, gid: u32) -> Result<Vec<u32>, ()> {
    if groups.is_empty() || groups.len() > MEMBERSHIPS || groups.contains(&u32::MAX) {
        return Err(());
    }
    groups.sort_unstable();
    if groups.windows(2).any(|pair| pair[0] == pair[1]) || groups.binary_search(&gid).is_err() {
        return Err(());
    }
    Ok(groups)
}
fn validate_mapping(u: &User, g: &Group, r: &User, s: &Group) -> Result<(), ()> {
    if [
        u.name.as_slice(),
        g.name.as_slice(),
        r.name.as_slice(),
        s.name.as_slice(),
    ]
    .iter()
    .any(|name| *name != ACCOUNT.as_bytes())
        || !nonroot_id(u.uid)
        || !nonroot_id(u.gid)
        || u.gid != g.gid
        || u.uid != r.uid
        || u.gid != r.gid
        || g.gid != s.gid
    {
        Err(())
    } else {
        Ok(())
    }
}
fn round(lookup: &impl Lookup) -> Result<WorkerAccountSnapshot, ()> {
    let user = lookup.user(None)?;
    let group = lookup.group(None)?;
    let reverse_user = lookup.user(Some(user.uid))?;
    let reverse_group = lookup.group(Some(group.gid))?;
    validate_mapping(&user, &group, &reverse_user, &reverse_group)?;
    Ok(WorkerAccountSnapshot {
        schema_version: 1,
        account: ACCOUNT.into(),
        group: ACCOUNT.into(),
        uid: user.uid,
        gid: user.gid,
        membership_gids: canonical_memberships(lookup.memberships(user.gid)?, user.gid)?,
    })
}
fn capture(lookup: &impl Lookup) -> Result<WorkerAccountSnapshot, ()> {
    let first = round(lookup)?;
    if first != round(lookup)? {
        return Err(());
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn snapshot() -> WorkerAccountSnapshot {
        WorkerAccountSnapshot {
            schema_version: 1,
            account: ACCOUNT.into(),
            group: ACCOUNT.into(),
            uid: 1000,
            gid: 1001,
            membership_gids: vec![4, 1001],
        }
    }
    struct Fake {
        calls: Cell<usize>,
        fail: Cell<Option<usize>>,
        drift: Cell<bool>,
        order: RefCell<Vec<&'static str>>,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                calls: Cell::new(0),
                fail: Cell::new(None),
                drift: Cell::new(false),
                order: RefCell::new(vec![]),
            }
        }
        fn step(&self, name: &'static str) -> Result<(), ()> {
            let call = self.calls.get();
            self.calls.set(call + 1);
            self.order.borrow_mut().push(name);
            if self.fail.get() == Some(call) {
                Err(())
            } else {
                Ok(())
            }
        }
    }
    impl Lookup for Fake {
        fn user(&self, uid: Option<u32>) -> Result<User, ()> {
            self.step(if uid.is_some() {
                "user-id"
            } else {
                "user-name"
            })?;
            Ok(User {
                name: ACCOUNT.as_bytes().to_vec(),
                uid: if self.drift.get() && self.calls.get() > 5 {
                    1002
                } else {
                    1000
                },
                gid: 1001,
            })
        }
        fn group(&self, gid: Option<u32>) -> Result<Group, ()> {
            self.step(if gid.is_some() {
                "group-id"
            } else {
                "group-name"
            })?;
            Ok(Group {
                name: ACCOUNT.as_bytes().to_vec(),
                gid: 1001,
            })
        }
        fn memberships(&self, _: u32) -> Result<Vec<u32>, ()> {
            self.step("memberships")?;
            Ok(vec![1001, 4])
        }
    }
    #[test]
    fn fixed_account_request_refuses_selectors_schema_duplicates_and_trailing_bytes() {
        check_request(REQUEST).unwrap();
        for bad in [
            b"".as_slice(),
            b"{}",
            b"{\"schema_version\":2}",
            b"{\"schema_version\":1,\"schema_version\":1}",
            b"{\"schema_version\":1,\"user\":\"root\"}",
            b"{\"schema_version\":1}\n",
        ] {
            assert!(check_request(bad).is_err());
        }
    }
    #[test]
    fn account_two_complete_rounds_bind_forward_reverse_and_memberships() {
        let fake = Fake::new();
        let observed = capture(&fake).unwrap();
        assert_eq!(observed, snapshot());
        assert_eq!(
            *fake.order.borrow(),
            [
                "user-name",
                "group-name",
                "user-id",
                "group-id",
                "memberships",
                "user-name",
                "group-name",
                "user-id",
                "group-id",
                "memberships"
            ]
        );
    }
    #[test]
    fn every_nss_failure_stops_without_fallback_or_retry() {
        for index in 0..10 {
            let fake = Fake::new();
            fake.fail.set(Some(index));
            assert!(capture(&fake).is_err());
            assert_eq!(fake.calls.get(), index + 1);
        }
        let fake = Fake::new();
        fake.drift.set(true);
        assert!(capture(&fake).is_err());
    }
    #[test]
    fn account_mapping_checks_each_forward_reverse_name_and_id() {
        let user = User {
            name: ACCOUNT.as_bytes().to_vec(),
            uid: 1000,
            gid: 1001,
        };
        let group = Group {
            name: ACCOUNT.as_bytes().to_vec(),
            gid: 1001,
        };
        validate_mapping(&user, &group, &user, &group).unwrap();
        for slot in 0..12 {
            let (mut u, mut g, mut r, mut s) =
                (user.clone(), group.clone(), user.clone(), group.clone());
            match slot {
                0 => u.name = b"foreign".to_vec(),
                1 => u.uid = 0,
                2 => u.uid = u32::MAX,
                3 => u.gid = 0,
                4 => g.gid = u32::MAX,
                5 => g.name = b"foreign".to_vec(),
                6 => r.name = b"foreign".to_vec(),
                7 => r.uid += 1,
                8 => r.gid += 1,
                9 => s.gid += 1,
                10 => s.name = b"foreign".to_vec(),
                _ => g.gid = 0,
            }
            assert!(validate_mapping(&u, &g, &r, &s).is_err());
        }
    }
    #[test]
    fn membership_capacity_duplicates_sentinel_and_primary_are_exact() {
        assert_eq!(
            canonical_memberships(vec![1001, 4], 1001).unwrap(),
            vec![4, 1001]
        );
        for list in [
            vec![],
            vec![4],
            vec![1001, 1001],
            vec![1001, u32::MAX],
            (0..33).collect(),
        ] {
            assert!(canonical_memberships(list, 1001).is_err());
        }
        let groups = (1..=32).collect();
        canonical_memberships(groups, 1).unwrap();
    }
    #[test]
    fn account_snapshot_wire_refuses_invalid_identity_and_noncanonical_input() {
        let bytes = canonical_json(&snapshot()).unwrap();
        WorkerAccountSnapshot::decode(&bytes).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        for bad in [
            text.replace("\"schema_version\":1", "\"schema_version\":2"),
            text.replacen('{', "{\"account\":\"hydracache-perf\",", 1),
            text.replacen('{', "{\"shell\":\"/bin/sh\",", 1),
            text.replace("\"uid\":1000", "\"uid\":0"),
            text.replace("\"account\":\"hydracache-perf\"", "\"account\":\"root\""),
            format!("{text}\n"),
            text.replace("[4,1001]", "[1001,4]"),
        ] {
            assert!(WorkerAccountSnapshot::decode(bad.as_bytes()).is_err());
        }
        assert!(WorkerAccountSnapshot::decode(&[b' '; SNAPSHOT_BYTES + 1]).is_err());
        assert!(WorkerAccountSnapshot::decode(b"{}").is_err());
    }
    #[test]
    fn nss_name_pointer_is_bounded_inside_its_original_buffer() {
        let buffer = b"hydracache-perf\0other";
        assert_eq!(
            bounded_name(buffer, buffer.as_ptr().cast()).unwrap(),
            ACCOUNT.as_bytes()
        );
        assert!(bounded_name(buffer, std::ptr::null()).is_err());
        assert!(bounded_name(buffer, buffer.as_ptr().wrapping_add(buffer.len()).cast()).is_err());
        let unterminated = b"bad";
        assert!(bounded_name(unterminated, unterminated.as_ptr().cast()).is_err());
    }
    #[test]
    fn nss_result_and_group_count_errors_are_never_partial_success() {
        for (rc, count) in [(-1, 33), (-1, 2), (2, 3), (0, 0), (33, 33), (1, -1)] {
            assert!(membership_count(rc, count).is_err());
        }
        assert_eq!(membership_count(32, 32).unwrap(), 32);
        let mut entry = 1u32;
        let pointer = &mut entry as *mut u32;
        check_result(0, pointer, pointer).unwrap();
        for rc in [libc::ERANGE, libc::EIO, libc::ENOENT] {
            assert!(check_result(rc, pointer, pointer).is_err());
        }
        assert!(check_result(0, &mut entry, std::ptr::null_mut()).is_err());
        let mut other = 2u32;
        assert!(check_result(0, &mut entry, &mut other).is_err());
    }
    #[test]
    fn account_guard_refuses_drift_and_never_observes_after_restoration() {
        let mut read = WorkerAccountRead::new(snapshot());
        let calls = Cell::new(0);
        read.observe(|| {
            calls.set(calls.get() + 1);
            Ok(snapshot())
        })
        .unwrap();
        let mut changed = snapshot();
        changed.uid += 1;
        assert!(read
            .observe(|| {
                calls.set(calls.get() + 1);
                Ok(changed)
            })
            .is_err());
        assert!(read.is_refused());
        let error = read
            .observe(|| {
                calls.set(calls.get() + 1);
                Ok(snapshot())
            })
            .unwrap_err();
        assert_eq!(error.kind, WorkerFailureKind::Invalid);
        assert_eq!(calls.get(), 2);
    }
    #[test]
    fn account_guard_preserves_typed_worker_error_and_independent_state() {
        let mut first = WorkerAccountRead::new(snapshot());
        let mut second = WorkerAccountRead::new(snapshot());
        let original = WorkerFailure {
            kind: WorkerFailureKind::Deadline,
            stdout: b"prefix".to_vec(),
            stderr: b"refusal".to_vec(),
            cleanup_confirmed: false,
        };
        let error = first.observe(|| Err(original)).unwrap_err();
        assert_eq!(error.kind, WorkerFailureKind::Deadline);
        assert_eq!(error.stdout, b"prefix");
        assert_eq!(error.stderr, b"refusal");
        assert!(!error.cleanup_confirmed);
        second.observe(|| Ok(snapshot())).unwrap();
        assert!(!second.is_refused());
    }
    #[test]
    fn refused_account_guard_preserves_pending_owned_helper_without_observation() {
        let child = Command::new("/bin/cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        let mut client = ManagerClient {
            pending: Some(child),
        };
        let mut read = WorkerAccountRead::new(snapshot());
        read.refused = true;
        let error = read.revalidate(&mut client).unwrap_err();
        let mut retained = client.pending.take().unwrap();
        let still_running = retained.try_wait().unwrap().is_none();
        retained.kill().unwrap();
        retained.wait().unwrap();
        assert_eq!(error.kind, WorkerFailureKind::Invalid);
        assert!(!error.cleanup_confirmed);
        assert_eq!(retained.id(), pid);
        assert!(still_running);
    }
    #[test]
    fn seeded_account_snapshot_mutations_cannot_adopt_another_identity() {
        let seed = 0x074a_2026_u64;
        eprintln!("account mutation seed={seed:#x}");
        let mut rng = seed;
        for _ in 0..256 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let mut changed = snapshot();
            match rng % 3 {
                0 => changed.uid += 1,
                1 => {
                    changed.gid += 1;
                    changed.membership_gids = vec![4, changed.gid];
                }
                _ => changed.membership_gids.push(1002),
            };
            // Drift must refuse even when the mutated projection is itself valid.
            let bytes = canonical_json(&changed).unwrap();
            assert_eq!(WorkerAccountSnapshot::decode(&bytes).unwrap(), changed);
            let mut read = WorkerAccountRead::new(snapshot());
            assert!(read.observe(|| Ok(changed)).is_err());
            assert!(read.is_refused());
        }
    }
}
