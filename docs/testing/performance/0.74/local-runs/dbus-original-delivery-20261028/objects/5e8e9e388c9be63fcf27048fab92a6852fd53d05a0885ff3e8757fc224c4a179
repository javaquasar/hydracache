//! Kernel executable object observation only, not original exec or memory proof.
use super::io::{magic_target, readonly_clone, Executable};
use super::{ProcessError, ProcessRead};
use std::fs::File;
use std::os::unix::fs::MetadataExt;

pub(crate) struct ProcessExecutableRead<'a> {
    process: &'a ProcessRead,
    expected: File,
    target: File,
    identity: Executable,
    refused: bool,
}

impl ProcessRead {
    pub(crate) fn pin_executable(
        &self,
        expected: &File,
    ) -> Result<ProcessExecutableRead<'_>, ProcessError> {
        self.revalidate()?;
        let expected = readonly_clone(expected)?;
        let metadata = expected.metadata()?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.nlink() != 1 {
            return Err(ProcessError::Invalid);
        }
        let identity = Executable::of(&metadata);
        let target = magic_target(&self.probe.files.directory, "exe")?;
        let mut observed = ProcessExecutableRead {
            process: self,
            expected,
            target,
            identity,
            refused: false,
        };
        observed.revalidate()?;
        Ok(observed)
    }
}

impl ProcessExecutableRead<'_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), ProcessError> {
        if self.refused {
            return Err(ProcessError::Drift);
        }
        let result = self.inspect();
        if result.is_err() {
            self.refused = true;
        }
        result
    }

    fn inspect(&self) -> Result<(), ProcessError> {
        self.process.revalidate()?;
        let directory = &self.process.probe.files.directory;
        let fresh = magic_target(directory, "exe")?;
        for file in [&self.expected, &self.target, &fresh] {
            if Executable::of(&file.metadata()?) != self.identity {
                return Err(ProcessError::Drift);
            }
        }
        let after = magic_target(directory, "exe")?;
        if Executable::of(&after.metadata()?) != self.identity
            || Executable::of(&self.expected.metadata()?) != self.identity
        {
            return Err(ProcessError::Drift);
        }
        self.process.revalidate()
    }
}

#[cfg(test)]
impl ProcessExecutableRead<'_> {
    pub(crate) fn assert_owned_cloexec_and_drop(self) {
        use std::os::fd::AsRawFd;
        let originals = [&self.expected, &self.target].map(|file| {
            let fd = file.as_raw_fd();
            // SAFETY: still-owned descriptor, read-only query.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            assert!(flags >= 0);
            assert_ne!(flags & libc::FD_CLOEXEC, 0);
            let m = file.metadata().unwrap();
            (fd, m.dev(), m.ino())
        });
        drop(self);
        for (fd, dev, ino) in originals {
            let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
            // SAFETY: fstat initializes the destination on success. Reused FD
            // numbers in parallel tests must not point at this unique fixture.
            if unsafe { libc::fstat(fd, metadata.as_mut_ptr()) } == 0 {
                // SAFETY: successful fstat above initialized the struct.
                let m = unsafe { metadata.assume_init() };
                assert_ne!((m.st_dev, m.st_ino), (dev, ino));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn executable_reader_rejects_writable_o_path_and_nonmatching_expectations() {
        let process = crate::diagnostic_process::pin_owned_test_helper(std::process::id());
        let temporary = tempfile::tempdir().unwrap();
        let writable = File::create(temporary.path().join("public-fixture")).unwrap();
        assert!(process.pin_executable(&writable).is_err());
        assert!(process
            .pin_executable(&File::open(temporary.path().join("public-fixture")).unwrap())
            .is_err());
        assert!(process
            .pin_executable(&File::open(temporary.path()).unwrap())
            .is_err());
        std::fs::write(temporary.path().join("linked"), b"public").unwrap();
        std::fs::hard_link(
            temporary.path().join("linked"),
            temporary.path().join("alias"),
        )
        .unwrap();
        assert!(process
            .pin_executable(&File::open(temporary.path().join("linked")).unwrap())
            .is_err());
        let current = magic_target(&process.probe.files.directory, "exe").unwrap();
        assert!(process.pin_executable(&current).is_err());
        assert!(process
            .pin_executable(&File::open("/bin/true").unwrap())
            .is_err());
        let own = File::open(std::env::current_exe().unwrap()).unwrap();
        process.pin_executable(&own).unwrap().revalidate().unwrap();
    }
}
