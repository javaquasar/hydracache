//! Diagnostic mirror, NOT installed transport. Erasing observer lines must restore
//! the exact source function; private helpers/constants are pinned separately.
use crate::census_runtime::{
    decoder,
    transport::{CommandFailure, Output, Refusal},
};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
const CLEANUP: Duration = Duration::from_secs(5);
const INPUT: usize = 256 * 1024;
const STDERR: usize = 64 * 1024;

pub enum CaptureEvent<'a> {
    Enter(&'static str),
    Leave(&'static str),
    Spawned(&'a Child),
    Reaped(&'a Child),
}

struct Owned {
    child: Child,
    cleanup_attempted: bool,
}
impl Owned {
    fn stop(&mut self) -> std::io::Result<()> {
        self.cleanup_attempted = true;
        if self.child.try_wait()?.is_none() {
            if let Err(error) = self.child.kill() {
                if self.child.try_wait()?.is_none() {
                    return Err(error);
                }
            }
            let until = Instant::now() + CLEANUP;
            while self.child.try_wait()?.is_none() {
                if Instant::now() >= until {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "owned child cleanup",
                    ));
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        Ok(())
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.cleanup_attempted {
            let _ = self.stop();
        }
    }
}
fn timely(deadline: Instant) -> Result<(), Refusal> {
    if Instant::now() >= deadline {
        Err(Refusal::Deadline)
    } else {
        Ok(())
    }
}
fn bytes(file: &mut File, limit: usize) -> Result<Vec<u8>, CommandFailure> {
    file.seek(SeekFrom::Start(0))?;
    let mut raw = Vec::new();
    file.take((limit + 1) as u64).read_to_end(&mut raw)?;
    if raw.len() > limit {
        return Err(Refusal::Budget.into());
    }
    Ok(raw)
}

pub fn run_profiled(
    command: &mut Command,
    input: &[u8],
    deadline: Instant,
    observe: &mut impl FnMut(CaptureEvent<'_>),
) -> Result<Output, CommandFailure> {
    timely(deadline)?;
    if input.len() > INPUT {
        return Err(Refusal::Input.into());
    }
    observe(CaptureEvent::Enter("source-tempfile"));
    let mut source = tempfile::tempfile()?;
    observe(CaptureEvent::Leave("source-tempfile"));
    observe(CaptureEvent::Enter("input-write-seek"));
    source.write_all(input)?;
    source.seek(SeekFrom::Start(0))?;
    observe(CaptureEvent::Leave("input-write-seek"));
    observe(CaptureEvent::Enter("output-tempfiles"));
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    observe(CaptureEvent::Leave("output-tempfiles"));
    timely(deadline)?;
    observe(CaptureEvent::Enter("setup-and-spawn"));
    let child = command
        .stdin(Stdio::from(source))
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .spawn()?;
    let mut owned = Owned {
        child,
        cleanup_attempted: false,
    };
    observe(CaptureEvent::Leave("setup-and-spawn"));
    observe(CaptureEvent::Spawned(&owned.child));
    let result = (|| loop {
        timely(deadline)?;
        observe(CaptureEvent::Enter("metadata"));
        if stdout.metadata()?.len() > decoder::MAX_STREAM as u64
            || stderr.metadata()?.len() > STDERR as u64
        {
            observe(CaptureEvent::Leave("metadata"));
            return Err(Refusal::Budget.into());
        }
        observe(CaptureEvent::Leave("metadata"));
        observe(CaptureEvent::Enter("try-wait"));
        if let Some(status) = owned.child.try_wait()? {
            observe(CaptureEvent::Leave("try-wait"));
            observe(CaptureEvent::Enter("capture-read"));
            let result = Output {
                stdout: bytes(&mut stdout, decoder::MAX_STREAM)?,
                stderr: bytes(&mut stderr, STDERR)?,
                code: status.code(),
            };
            observe(CaptureEvent::Leave("capture-read"));
            timely(deadline)?;
            return Ok(result);
        }
        observe(CaptureEvent::Leave("try-wait"));
        observe(CaptureEvent::Enter("sleep"));
        std::thread::sleep(Duration::from_millis(5));
        observe(CaptureEvent::Leave("sleep"));
    })();
    observe(CaptureEvent::Enter("stop"));
    if let Err(error) = owned.stop() {
        return Err(CommandFailure {
            refusal: Refusal::Cleanup {
                primary: result
                    .err()
                    .map(|failure: CommandFailure| Box::new(failure.refusal)),
                reason: error.to_string(),
            },
            original_io: Some(error),
        });
    }
    observe(CaptureEvent::Leave("stop"));
    observe(CaptureEvent::Reaped(&owned.child));
    result
}
