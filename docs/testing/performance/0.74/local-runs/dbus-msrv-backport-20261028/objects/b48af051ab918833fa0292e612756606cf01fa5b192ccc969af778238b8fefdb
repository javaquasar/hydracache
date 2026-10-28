//! The pool of threads that blocking work runs on.
//!
//! A thread of the pool runs one job after another, for as long as there are jobs to run, and
//! then waits for the next. A job handed to the pool goes to a thread that waits, where one does;
//! failing that, to a thread started for it, where the pool has room for one more; and failing
//! that, it waits its turn, behind the jobs handed over before it, for a thread to be done with the
//! job it runs. A thread that has waited for a job for the pool's idle timeout and found none goes,
//! so a pool with nothing to run holds no threads.

use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    ops::Deref,
    panic::{self, AssertUnwindSafe},
    sync::{Condvar, Mutex, MutexGuard, PoisonError},
    thread,
    time::Duration,
};

use super::dispose;

/// A pool of threads that run the jobs handed to it, one at a time on each thread.
///
/// A pool is handed jobs through a handle to it, of which every thread it starts keeps a copy for
/// as long as it runs: a `&'static Pool` for a pool that is a `static`, which needs no allocation
/// and can be built in a `const`, and an `Arc<Pool>` for one that is not.
///
/// Every thread of a pool has the name the pool was given, so that the threads of one pool can be
/// told from those of another in a list of threads, in a debugger or a profiler, say.
pub(crate) struct Pool {
    /// The jobs that wait for a thread, and the tally of the threads.
    queue: Mutex<Queue>,
    /// What a thread with no job to run waits on for one to come.
    job_queued: Condvar,
    /// The name of every thread of the pool.
    name: &'static str,
    /// The most threads the pool has at once.
    cap: NonZeroUsize,
    /// How long a thread waits for a job before it goes.
    idle_timeout: Duration,
}

/// A piece of work for a thread of a pool, which runs it once.
pub(crate) type Job = Box<dyn FnOnce() + Send>;

impl Pool {
    /// A pool with no thread yet, which starts up to `cap` of them as jobs come, each named `name`
    /// and each of which goes once it has waited for a job for `idle_timeout` and found none.
    pub(crate) const fn new(name: &'static str, cap: NonZeroUsize, idle_timeout: Duration) -> Self {
        Self {
            queue: Mutex::new(Queue {
                jobs: VecDeque::new(),
                threads: 0,
                idle: 0,
            }),
            job_queued: Condvar::new(),
            name,
            cap,
            idle_timeout,
        }
    }

    /// Hands `job` to the pool that `pool` refers to, for one of its threads to run.
    ///
    /// Where no thread of the pool waits for a job, one is started for it, as long as the pool has
    /// fewer threads than its cap. Otherwise the job waits its turn.
    ///
    /// # Panics
    ///
    /// Panics if the pool has no thread at all and cannot start one, as [`std::thread::spawn`]
    /// does when it cannot. A pool that has threads leaves the job to them instead.
    pub(crate) fn submit<P>(pool: P, job: Job)
    where
        P: Deref<Target = Self> + Clone + Send + 'static,
    {
        let mut queue = pool.lock();
        queue.jobs.push_back(job);
        pool.job_queued.notify_one();

        // Each idle thread takes up a job: one that waits, or one that has been started and is yet
        // to take its first. A job beyond those gets a thread started for it, while there is room.
        while queue.jobs.len() > queue.idle && queue.threads < pool.cap.get() {
            let handle = pool.clone();
            let spawned = thread::Builder::new()
                .name(pool.name.into())
                .spawn(move || serve(handle));
            match spawned {
                Ok(_) => {
                    // Counted idle until it takes a job, so that the next turn of this loop, or
                    // another call, leaves that job to it rather than start another thread for it.
                    queue.threads += 1;
                    queue.idle += 1;
                }
                // The threads that the pool has get to the job in their turn. None of them goes
                // while it is queued, since a thread only goes once it finds the queue empty.
                Err(_) if queue.threads > 0 => break,
                Err(error) => {
                    // With no thread to run it, the job is taken back, and dropped past the lock:
                    // what it holds may have a drop of its own that hands the pool a job.
                    let job = queue.jobs.pop_back();
                    drop(queue);
                    drop(job);
                    panic!("failed to spawn a thread for blocking work: {error}");
                }
            }
        }
    }

    /// How many threads the pool has, whether running a job or waiting for one.
    #[cfg(test)]
    pub(crate) fn threads(&self) -> usize {
        self.lock().threads
    }

    /// How many threads of the pool are idle: waiting for a job, or started and yet to take their
    /// first.
    #[cfg(test)]
    pub(crate) fn idle_threads(&self) -> usize {
        self.lock().idle
    }

    /// The queue behind its lock, taken whether or not a panic poisoned it.
    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Runs the jobs of the pool that `pool` refers to, one after another, until the thread has waited
/// for the idle timeout and found no job: what a thread of the pool does from its start to its end.
fn serve<P>(pool: P)
where
    P: Deref<Target = Pool>,
{
    let mut queue = pool.lock();
    // Counted idle from its start until now, on its way to the queue, so that no other thread was
    // started for the job it was started for.
    queue.idle -= 1;
    loop {
        if let Some(job) = queue.jobs.pop_front() {
            drop(queue);
            // A panic in the work inside a job is caught there, to be handed to whoever awaits its
            // outcome. This catches whatever else in the job panics, such as the waker that the job
            // wakes as it ends, so that it never takes the thread down with it, and disposes of its
            // payload so that a `Drop` of the payload's that panics in turn does not either: a
            // thread that went would still be counted, and hold its place under the cap for good.
            // Running the job consumes it, so that it holds nothing once it has run, while the
            // thread waits for the next.
            if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(job)) {
                dispose(payload);
            }
            queue = pool.lock();
            continue;
        }

        queue.idle += 1;
        let (guard, wait) = pool
            .job_queued
            .wait_timeout(queue, pool.idle_timeout)
            .unwrap_or_else(PoisonError::into_inner);
        queue = guard;
        queue.idle -= 1;
        // A job handed over as the wait ran out finds this thread still counted idle, and so no
        // thread started for it: it is taken up before the thread may go.
        if wait.timed_out() && queue.jobs.is_empty() {
            queue.threads -= 1;
            return;
        }
    }
}

/// The jobs of a pool that wait for a thread, and the tally of its threads.
struct Queue {
    /// The jobs that no thread has taken up yet, the oldest first.
    jobs: VecDeque<Job>,
    /// How many threads the pool has.
    threads: usize,
    /// How many of those are idle: waiting for a job, or started and yet to take their first.
    ///
    /// A thread that waits takes itself out of this count as it stops waiting, whatever stopped
    /// it, so that a job handed over in between finds it counted, and no thread started for it.
    idle: usize,
}
