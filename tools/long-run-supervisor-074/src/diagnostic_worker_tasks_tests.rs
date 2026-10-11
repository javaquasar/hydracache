use super::*;
use crate::diagnostic_process::pin_owned_test_helper;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const HELPER: &str = r#"
import ctypes, os, sys, threading
if ctypes.CDLL(None).prctl(38, 1, 0, 0, 0) != 0: raise RuntimeError('NNP')
threads = []
def add():
    ready = threading.Event()
    end = threading.Event()
    def worker():
        ready.set()
        end.wait()
    thread = threading.Thread(target=worker)
    thread.start()
    ready.wait()
    threads.append((thread, end))
def remove():
    thread, end = threads.pop()
    tid = thread.native_id
    end.set()
    thread.join()
    # Fixture acknowledgment requires kernel task absence, not only Python join.
    # This waits for a logical condition; the production observer never retries.
    for _ in range(65536):
        if not os.path.exists('/proc/self/task/' + str(tid)): break
        os.sched_yield()
    else: raise RuntimeError('kernel task removal did not complete')
for _ in range(int(sys.argv[1])): add()
print('ready', flush=True)
for line in sys.stdin:
    if line.strip() == 'add': add()
    elif line.strip() == 'remove': remove()
    else: break
    print('ready', flush=True)
while threads: remove()
"#;
pub(crate) struct Owned {
    pub(crate) child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Owned {
    pub(crate) fn new(threads: usize) -> Self {
        let mut child = Command::new("python3")
            .args(["-u", "-c", HELPER, &threads.to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut owned = Self {
            child,
            input,
            output,
        };
        owned.ready();
        owned
    }
    fn ready(&mut self) {
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        assert_eq!(line, "ready\n");
    }
    pub(crate) fn command(&mut self, command: &str) {
        writeln!(self.input, "{command}").unwrap();
        self.input.flush().unwrap();
        self.ready();
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn policy() -> AssertedWorkerCredentials {
    // SAFETY: read-only scalar credential queries, no pointers.
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let mut groups = [0; 32];
    // SAFETY: valid bounded writable group buffer.
    let n = unsafe { libc::getgroups(groups.len() as i32, groups.as_mut_ptr()) };
    assert!(n >= 0);
    let mut groups = groups[..n as usize].to_vec();
    groups.sort_unstable();
    groups.dedup();
    AssertedWorkerCredentials::new(uid, gid, groups).unwrap()
}
fn status() -> Vec<u8> {
    b"Pid:\t18\nTgid:\t17\nUid:\t1000 1000 1000 1000\nGid:\t1000 1000 1000 1000\nGroups:\t4 1000\nCapInh:\t0000000000000000\nCapPrm:\t0000000000000000\nCapEff:\t0000000000000000\nCapBnd:\t000001ffffffffff\nCapAmb:\t0000000000000000\nNoNewPrivs:\t1\n".to_vec()
}
fn replace_status(bytes: &[u8], key: &str, value: &str) -> Vec<u8> {
    String::from_utf8(bytes.to_vec())
        .unwrap()
        .lines()
        .map(|line| {
            if line.starts_with(key) {
                format!("{key}{value}\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect::<String>()
        .into_bytes()
}
fn record(name: &[u8]) -> Vec<u8> {
    let size = (20 + name.len()).div_ceil(8) * 8;
    let mut bytes = vec![0; size];
    bytes[..8].copy_from_slice(&1_u64.to_ne_bytes());
    bytes[16..18].copy_from_slice(&(size as u16).to_ne_bytes());
    bytes[18] = libc::DT_DIR;
    bytes[19..19 + name.len()].copy_from_slice(name);
    bytes
}
fn listing(names: &[&[u8]]) -> Vec<u8> {
    names.iter().flat_map(|name| record(name)).collect()
}
fn scan(bytes: &[u8], leader: u32) -> Result<Vec<u32>, TaskError> {
    let mut first = true;
    enumerate(leader, |buffer| {
        if !first {
            return Ok(0);
        }
        first = false;
        buffer[..bytes.len()].copy_from_slice(bytes);
        Ok(bytes.len())
    })
}

#[test]
fn task_set_directory_records_are_strict_bounded_and_sorted() {
    assert_eq!(
        scan(&listing(&[b".", b"..", b"19", b"17", b"18"]), 17).unwrap(),
        [17, 18, 19]
    );
    for names in [
        vec![b"18".as_slice()],
        vec![b"17", b"17"],
        vec![],
        vec![b"."],
        vec![b"00"],
        vec![b"+17"],
        vec![b"0"],
        vec![b"2147483648"],
        vec![b"17/18"],
        vec![b"17\xff"],
    ] {
        assert!(scan(&listing(&names), 17).is_err(), "{names:?}");
    }
    for length in 0..24 {
        assert!(directory_record(&record(b"17")[..length]).is_err());
    }
    for index in [0, 16, 17, 18, 19] {
        let mut bytes = record(b"17");
        bytes[index] = if index == 17 { 255 } else { 0 };
        assert!(directory_record(&bytes).is_err(), "byte {index}");
    }
    let mut missing_nul = record(b"17");
    missing_nul[21..].fill(b'x');
    assert!(directory_record(&missing_nul).is_err());
    let mut unaligned = record(b"17");
    unaligned[16] = 23;
    assert!(directory_record(&unaligned).is_err());
    let names = (1..=MAX_TASKS).map(|id| id.to_string()).collect::<Vec<_>>();
    let refs = names.iter().map(|name| name.as_bytes()).collect::<Vec<_>>();
    assert_eq!(scan(&listing(&refs), 1).unwrap().len(), MAX_TASKS);
    let mut overflow = listing(&refs);
    overflow.extend(record(b"129"));
    assert!(matches!(scan(&overflow, 1), Err(TaskError::Budget)));
}

#[test]
fn task_set_enumeration_has_no_retry_and_exact_call_budget() {
    let mut calls = 0;
    assert!(matches!(
        enumerate(17, |_| {
            calls += 1;
            Err(std::io::Error::from_raw_os_error(libc::EINTR).into())
        }),
        Err(TaskError::Io(_))
    ));
    assert_eq!(calls, 1);
    let bytes = record(b"17");
    let mut calls = 0;
    assert!(matches!(
        enumerate(17, |buffer| {
            calls += 1;
            buffer[..bytes.len()].copy_from_slice(&bytes);
            Ok(bytes.len())
        }),
        Err(TaskError::Budget)
    ));
    assert_eq!(calls, MAX_DIRECTORY_CALLS);
    assert!(matches!(
        enumerate(17, |buffer| Ok(buffer.len() + 1)),
        Err(TaskError::Budget)
    ));
    // Byte exhaustion is separate from task count: a legal record can carry padding.
    let mut large = record(b".");
    large.resize(8192, 0);
    large[16..18].copy_from_slice(&8192_u16.to_ne_bytes());
    let mut calls = 0;
    assert!(matches!(
        enumerate(17, |buffer| {
            calls += 1;
            buffer.copy_from_slice(&large);
            Ok(large.len())
        }),
        Err(TaskError::Budget)
    ));
    assert_eq!(calls, 9);
    let mut calls = 0;
    let accepted = enumerate(17, |buffer| {
        calls += 1;
        if calls == 9 {
            return Ok(0);
        }
        let mut bytes = record(if calls == 1 { b"17" } else { b"." });
        bytes.resize(8192, 0);
        bytes[16..18].copy_from_slice(&8192_u16.to_ne_bytes());
        buffer.copy_from_slice(&bytes);
        Ok(bytes.len())
    })
    .unwrap();
    assert_eq!(accepted, [17]);
    assert_eq!(calls, 9);
}

#[test]
fn task_set_seeded_directory_mutations_never_panic_or_accept_a_selector() {
    let mut seed = 0x7622026_u64;
    eprintln!("task directory modeled seed={seed:#x}; mutations=512");
    for _ in 0..512 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bytes = record(b"17");
        let index = seed as usize % bytes.len();
        bytes[index] ^= ((seed >> 32) as u8) | 1;
        if let Ok((Some(id), size)) = directory_record(&bytes) {
            assert!(id > 0 && id <= i32::MAX as u32);
            assert!(size <= bytes.len() && size.is_multiple_of(8));
        }
    }
}

#[test]
fn task_set_status_checks_tgid_all_slots_groups_capabilities_and_nnp() {
    let policy = AssertedWorkerCredentials::new(1000, 1000, vec![4, 1000]).unwrap();
    let before = asserted_task_status(&status(), 18, 17, &policy).unwrap();
    assert_ne!(before, 0);
    for key in ["Uid:\t", "Gid:\t"] {
        for slot in 0..4 {
            let mut ids = [1000; 4];
            ids[slot] = 1001;
            let value = ids.map(|id| id.to_string()).join(" ");
            assert!(matches!(
                asserted_task_status(&replace_status(&status(), key, &value), 18, 17, &policy),
                Err(CredentialError::Drift)
            ));
        }
    }
    for (key, value) in [
        ("Pid:\t", "17"),
        ("Tgid:\t", "18"),
        ("Groups:\t", "4"),
        ("CapInh:\t", "0000000000000001"),
        ("CapPrm:\t", "0000000000000001"),
        ("CapEff:\t", "0000000000000001"),
        ("CapAmb:\t", "0000000000000001"),
        ("NoNewPrivs:\t", "0"),
    ] {
        assert!(
            matches!(
                asserted_task_status(&replace_status(&status(), key, value), 18, 17, &policy),
                Err(CredentialError::Drift)
            ),
            "{key}"
        );
    }
    assert!(asserted_task_status(
        &replace_status(&status(), "Tgid:\t", "017"),
        18,
        17,
        &policy
    )
    .is_err());
    let after = asserted_task_status(
        &replace_status(&status(), "CapBnd:\t", "0000000000000000"),
        18,
        17,
        &policy,
    )
    .unwrap();
    assert_ne!(before, after);
}

#[test]
fn task_set_order_first_failure_and_seeded_refusal_are_exact() {
    let mut gate = Gate::default();
    let mut order = vec![];
    gate.observe(|step| {
        order.push(step);
        Ok(())
    })
    .unwrap();
    assert_eq!(order, ORDER);
    let mut rng = 0x7612026_u64;
    eprintln!("task-set modeled seed={rng:#x}; schedules=256");
    for schedule in 0..256 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let failed = if schedule < ORDER.len() {
            schedule
        } else {
            rng as usize % ORDER.len()
        };
        let mut gate = Gate::default();
        let mut calls = 0;
        assert!(matches!(
            gate.observe(|_| {
                let here = calls;
                calls += 1;
                if here == failed {
                    Err(TaskError::Io(std::io::Error::from_raw_os_error(libc::EIO)))
                } else {
                    Ok(())
                }
            }),
            Err(TaskError::Io(_))
        ));
        assert_eq!(calls, failed + 1);
        assert!(gate.refused);
        assert!(matches!(
            gate.observe(|_| panic!("no IO after refusal")),
            Err(TaskError::Refused)
        ));
    }
}

#[test]
fn task_set_is_neither_send_nor_sync() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <WorkerTasksRead<'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <WorkerTasksRead<'static> as AmbiguousSync<_>>::marker;
}

#[test]
fn task_set_real_single_and_multiple_tasks_remain_original_on_healthy_drop() {
    for threads in [0, 2] {
        let owned = Owned::new(threads);
        let process = pin_owned_test_helper(owned.child.id());
        let mut read = pin_asserted_worker_tasks(&process, policy()).unwrap();
        assert_eq!(read.probe.tasks.len(), threads + 1);
        read.revalidate().unwrap();
        assert!(!read.is_refused());
        drop(read);
        process.revalidate().unwrap();
    }
}

#[test]
fn task_set_real_add_remove_and_restoration_never_refresh_inventory() {
    for add in [true, false] {
        let mut owned = Owned::new(1);
        let process = pin_owned_test_helper(owned.child.id());
        let mut read = pin_asserted_worker_tasks(&process, policy()).unwrap();
        owned.command(if add { "add" } else { "remove" });
        let result = read.revalidate();
        eprintln!("task membership add={add}: {result:?}");
        assert!(matches!(result, Err(TaskError::Drift)), "{result:?}");
        owned.command(if add { "remove" } else { "add" });
        assert!(matches!(read.revalidate(), Err(TaskError::Refused)));
        drop(read);
        process.revalidate().unwrap();
    }
}

#[test]
fn task_set_original_exit_and_dead_construction_never_adopt_replacement() {
    let mut owned = Owned::new(1);
    let process = pin_owned_test_helper(owned.child.id());
    let mut read = pin_asserted_worker_tasks(&process, policy()).unwrap();
    owned.child.kill().unwrap();
    owned.child.wait().unwrap();
    let replacement = Owned::new(1);
    assert!(matches!(
        read.revalidate(),
        Err(TaskError::Process(ProcessError::NotLive))
    ));
    assert!(matches!(read.revalidate(), Err(TaskError::Refused)));
    assert!(pin_asserted_worker_tasks(&process, policy()).is_err());
    assert!(replacement.child.id() != owned.child.id());
}

#[test]
fn task_set_retained_substitution_namespace_type_and_thread_drift_refuse() {
    let owned = Owned::new(1);
    let process = pin_owned_test_helper(owned.child.id());
    for case in 0..8 {
        let mut read = pin_asserted_worker_tasks(&process, policy()).unwrap();
        match case {
            0 => read.probe.tasks[1].generation.start_ticks += 1,
            1 => read.probe.tasks[1].bounding ^= 1,
            2 => read.probe.tasks[1].id.1 ^= 1,
            3 => read.probe.tasks[1].ns_id.1 ^= 1,
            4 => read.probe.observer_tid += 1,
            5 => read.probe.tasks[1].user.file = tempfile::tempfile().unwrap(),
            6 => read.probe.tasks[1].mount.file = tempfile::tempfile().unwrap(),
            7 => read.probe.tasks[1].status.file = tempfile::tempfile().unwrap(),
            _ => unreachable!(),
        }
        assert!(read.revalidate().is_err(), "{case}");
        assert!(read.is_refused());
        assert!(matches!(read.revalidate(), Err(TaskError::Refused)));
    }
    let ns = reader_namespaces(&process.probe.files.root).unwrap();
    let user = Namespace::open(&ns, c"user", libc::CLONE_NEWUSER).unwrap();
    assert!(matches!(
        namespace_id(&user.file, libc::CLONE_NEWNS),
        Err(TaskError::Invalid)
    ));
    let mut read = pin_asserted_worker_tasks(&process, policy()).unwrap();
    read.probe.tasks[1].mount.id.1 ^= 1;
    assert!(matches!(read.revalidate(), Err(TaskError::Drift)));
    read.probe.tasks[1].mount.id.1 ^= 1;
    assert!(matches!(read.revalidate(), Err(TaskError::Refused)));
}

#[test]
fn task_set_concurrent_independent_readers_share_no_latch() {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let joins = (0..2)
        .map(|fail| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let owned = Owned::new(1);
                let process = pin_owned_test_helper(owned.child.id());
                let mut read = pin_asserted_worker_tasks(&process, policy()).unwrap();
                barrier.wait();
                if fail == 0 {
                    read.probe.tasks[1].bounding ^= 1;
                }
                assert_eq!(read.revalidate().is_err(), fail == 0);
                barrier.wait();
                assert_eq!(read.is_refused(), fail == 0);
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    barrier.wait();
    for join in joins {
        join.join().unwrap();
    }
}
