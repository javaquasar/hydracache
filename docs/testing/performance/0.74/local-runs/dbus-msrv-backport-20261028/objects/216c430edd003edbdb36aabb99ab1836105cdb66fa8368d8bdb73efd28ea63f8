//! Tests of the pool of threads that blocking work runs on, each on a pool of its own, small enough
//! to fill, with an idle timeout chosen for the test.
//!
//! They hand the pool plain jobs that report back over channels: how the pool shares its threads
//! out among jobs is the same whatever the job, and the hand-over of an outcome that `unblock`
//! builds on it is tested in the module above. They come in the order of what they pin down: that a
//! thread that is done with a job takes up the next rather than the pool starting one for it, that
//! jobs that can only finish together each get a thread, that past its cap the pool has a job wait
//! for a thread to be done with the one it runs, that a thread goes once it has waited for a job
//! for the idle timeout, that a job that panics leaves its thread to the jobs after it, even where
//! the payload of its panic panics in turn as it drops, and that the threads of a pool have the
//! name the pool was given.
//!
//! The pool settles whether to start a thread for a job before handing the job over returns, so a
//! test can count the threads right after. Where a test waits for a thread to go back to waiting
//! for a job, or to go, it looks at the count until it says so.
//!
//! Every test ends by waiting for the threads of its pool to go, which each does once it has waited
//! for a job for the idle timeout: Miri ends a program with an error if a thread other than the
//! main one is still running as the main one returns. So the idle timeout of a test is also what
//! the test waits out at its end, and it is short wherever no thread has to wait for a job in the
//! course of the test.

use std::{
    collections::HashSet,
    num::NonZeroUsize,
    panic,
    sync::{Arc, Barrier, mpsc},
    thread::{self, ThreadId},
    time::Duration,
};

use ntest::timeout;

use crate::unblock::pool::Pool;

/// A thread that is done with a job and waits for the next takes it up, and the pool starts no
/// thread for it.
#[test]
#[timeout(15000)]
fn an_idle_thread_takes_up_the_next_job() {
    let pool = pool(4, LONG_IDLE_TIMEOUT);

    let first = run(&pool, || thread::current().id());
    // The job reports from inside itself, before its thread goes back to waiting for a job: a job
    // handed over before then would find no thread waiting, and get one started for it.
    wait_until(|| pool.idle_threads() == 1);
    let second = run(&pool, || thread::current().id());

    assert_eq!(first, second);
    assert_eq!(pool.threads(), 1);
    wind_down(&pool);
}

/// Jobs that can only finish together each get a thread, as long as the cap leaves room for them,
/// and the pool starts no more threads than there are jobs.
#[test]
#[timeout(15000)]
fn jobs_that_wait_on_each_other_each_get_a_thread() {
    const JOBS: usize = 3;

    let pool = pool(JOBS + 1, SHORT_IDLE_TIMEOUT);
    // The test waits on the barrier as well, so that no job is done, and so no thread can go,
    // before the test has counted them.
    let barrier = Arc::new(Barrier::new(JOBS + 1));
    let (report, reported) = mpsc::channel();

    for _ in 0..JOBS {
        let barrier = barrier.clone();
        let report = report.clone();
        Pool::submit(
            pool.clone(),
            Box::new(move || {
                barrier.wait();
                let id = thread::current().id();
                report.send(id).expect("the test waits for the job");
            }),
        );
    }
    assert_eq!(pool.threads(), JOBS);
    barrier.wait();

    let threads: HashSet<ThreadId> = reported.iter().take(JOBS).collect();
    assert_eq!(threads.len(), JOBS);
    wind_down(&pool);
}

/// A pool at its cap starts no thread for a job, which waits for one of the threads to be done
/// with the job it runs, and then runs on that thread.
#[test]
#[timeout(15000)]
fn past_its_cap_a_job_waits_for_a_thread_to_be_done() {
    const CAP: usize = 2;

    let pool = pool(CAP, SHORT_IDLE_TIMEOUT);
    let (started, starts) = mpsc::channel();
    let releases: Vec<mpsc::Sender<()>> = (0..CAP)
        .map(|job| {
            let (release, released) = mpsc::channel();
            let started = started.clone();
            Pool::submit(
                pool.clone(),
                Box::new(move || {
                    let id = thread::current().id();
                    started.send((job, id)).expect("the test waits for the job");
                    released.recv().expect("the test lets the job end");
                }),
            );

            release
        })
        .collect();
    let mut busy: Vec<(usize, ThreadId)> = starts.iter().take(CAP).collect();
    busy.sort_by_key(|&(job, _)| job);

    let (ran, runs) = mpsc::channel();
    Pool::submit(
        pool.clone(),
        Box::new(move || {
            let id = thread::current().id();
            ran.send(id).expect("the test waits for the job");
        }),
    );
    assert_eq!(pool.threads(), CAP);

    // The other job still holds its thread, so the one left waiting can only run on this one's.
    releases[0].send(()).expect("the job waits for the test");
    assert_eq!(runs.recv().expect("the job left waiting runs"), busy[0].1);

    for release in &releases[1..] {
        release.send(()).expect("the job waits for the test");
    }
    wind_down(&pool);
}

/// A thread that has waited for a job for the idle timeout goes, and the pool starts another for
/// the next job that comes.
#[test]
#[timeout(15000)]
fn an_idle_thread_goes_after_the_idle_timeout() {
    let pool = pool(4, SHORT_IDLE_TIMEOUT);
    let (release, released) = mpsc::channel();

    Pool::submit(
        pool.clone(),
        Box::new(move || released.recv().expect("the test lets the job end")),
    );
    // Busy with the job, the thread cannot go yet.
    assert_eq!(pool.threads(), 1);
    release.send(()).expect("the job waits for the test");

    wind_down(&pool);
    assert_eq!(run(&pool, || 42), 42);
    wind_down(&pool);
}

/// A job that panics leaves its thread to the pool, which runs the jobs after it there.
#[test]
#[timeout(15000)]
fn a_job_that_panics_leaves_its_thread_to_the_jobs_after_it() {
    let pool = pool(4, LONG_IDLE_TIMEOUT);
    let (report, reported) = mpsc::channel();

    Pool::submit(
        pool.clone(),
        Box::new(move || {
            report
                .send(thread::current().id())
                .expect("the test waits for the job");
            panic!("the job panicked");
        }),
    );
    let panicked = reported.recv().expect("the job runs");
    // As in `an_idle_thread_takes_up_the_next_job`, the next job waits for the thread to go back
    // to waiting for one.
    wait_until(|| pool.idle_threads() == 1);
    let next = run(&pool, || thread::current().id());

    assert_eq!(next, panicked);
    assert_eq!(pool.threads(), 1);
    wind_down(&pool);
}

/// A job whose panic carries a payload that panics in turn as it is dropped leaves its thread to
/// the jobs after it all the same. The pool has room for one thread alone, so a thread that went
/// with the second panic, still counted, would leave the next job waiting for good.
#[test]
#[timeout(15000)]
fn a_panic_whose_payload_panics_as_it_drops_leaves_the_thread_to_the_jobs_after_it() {
    let pool = pool(1, LONG_IDLE_TIMEOUT);

    Pool::submit(pool.clone(), Box::new(|| panic::panic_any(PanicsOnDrop)));
    let ran = run(&pool, || "the next job ran");

    assert_eq!(ran, "the next job ran");
    assert_eq!(pool.threads(), 1);
    wind_down(&pool);
}

/// A thread of a pool has the name the pool was given, so that the threads of one pool can be told
/// from those of another.
#[test]
#[timeout(15000)]
fn the_threads_of_a_pool_have_its_name() {
    let pool = pool(1, SHORT_IDLE_TIMEOUT);

    let name = run(&pool, || thread::current().name().map(ToOwned::to_owned));

    assert_eq!(name.as_deref(), Some(POOL_NAME));
    wind_down(&pool);
}

/// A pool of up to `threads` threads, each named `POOL_NAME` and each of which goes once it has
/// waited for a job for `idle_timeout`.
fn pool(threads: usize, idle_timeout: Duration) -> Arc<Pool> {
    let cap = NonZeroUsize::new(threads).expect("a pool has room for a thread");

    Arc::new(Pool::new(POOL_NAME, cap, idle_timeout))
}

/// Runs `job` on `pool`, and waits for what it returns.
fn run<F, T>(pool: &Arc<Pool>, job: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let (report, reported) = mpsc::channel();
    Pool::submit(
        pool.clone(),
        Box::new(move || report.send(job()).expect("the test waits for the job")),
    );

    reported.recv().expect("the job runs")
}

/// Waits for every thread of `pool` to go, which each does once it has waited for a job for the
/// idle timeout.
fn wind_down(pool: &Pool) {
    wait_until(|| pool.threads() == 0);
}

/// Waits for `condition` to hold, looking again every millisecond.
fn wait_until<F>(condition: F)
where
    F: Fn() -> bool,
{
    while !condition() {
        thread::sleep(Duration::from_millis(1));
    }
}

/// An idle timeout for a pool whose threads wait for a job in the course of the test, long enough
/// that a thread is still waiting when the test hands it one, however slowly the test runs.
const LONG_IDLE_TIMEOUT: Duration = Duration::from_secs(2);

/// An idle timeout for a pool whose threads only wait for a job once the test is done with them,
/// short so that the test does not wait long for them to go.
const SHORT_IDLE_TIMEOUT: Duration = Duration::from_millis(50);

/// The name of every thread of the pools that the tests build.
const POOL_NAME: &str = "zruntime test pool";

/// A panic payload whose `Drop` panics in turn.
struct PanicsOnDrop;

impl Drop for PanicsOnDrop {
    fn drop(&mut self) {
        panic!("the payload of the panic panicked as it dropped");
    }
}
