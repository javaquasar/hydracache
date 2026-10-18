//! Private original descriptor inventory, not guardian enrollment or start.
use super::*;
use std::os::fd::RawFd;

#[derive(Debug, PartialEq, Eq)]
struct Roles {
    // Initial guardian inheritance only: actual guardian must shed sink aliases
    // before its collector barrier, as the unchanged kernel engine already does.
    guardian: [RawFd; 6],
    collector: [RawFd; 4],
    writers: [RawFd; 4],
}
fn roles(fds: [RawFd; 10]) -> Result<Roles, OriginalCustodyError> {
    if fds.iter().any(|fd| *fd < 0) || fds.iter().enumerate().any(|(i, fd)| fds[..i].contains(fd)) {
        return Err(OriginalCustodyError::Binding);
    }
    let mut result = Roles {
        guardian: [fds[0], fds[1], fds[2], fds[3], fds[4], fds[5]],
        collector: [fds[2], fds[3], fds[4], fds[5]],
        writers: [fds[6], fds[7], fds[8], fds[9]],
    };
    result.guardian.sort_unstable();
    result.collector.sort_unstable();
    result.writers.sort_unstable();
    Ok(result)
}
#[derive(PartialEq, Eq)]
struct Shape {
    stat: [u64; 11],
    descriptor: i32,
    status: i32,
    position: Option<i64>,
}
fn io_error() -> OriginalCustodyError {
    OriginalGroupedPrepError::Io(std::io::Error::last_os_error()).into()
}
fn shape(fd: RawFd, sink: bool) -> Result<Shape, OriginalCustodyError> {
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd, &mut stat) } != 0 {
        return Err(io_error());
    }
    let descriptor = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    let status = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if descriptor < 0 || status < 0 {
        return Err(io_error());
    }
    let position = if sink {
        let position = unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) };
        if position < 0 {
            return Err(io_error());
        }
        if position != 0 {
            return Err(OriginalCustodyError::Binding);
        }
        Some(position)
    } else {
        None
    };
    Ok(Shape {
        stat: [
            stat.st_dev,
            stat.st_ino,
            u64::from(stat.st_uid),
            u64::from(stat.st_gid),
            u64::from(stat.st_mode),
            stat.st_nlink,
            stat.st_size as u64,
            stat.st_mtime as u64,
            stat.st_mtime_nsec as u64,
            stat.st_ctime as u64,
            stat.st_ctime_nsec as u64,
        ],
        descriptor,
        status,
        position,
    })
}
#[derive(PartialEq, Eq)]
struct Inventory {
    fds: [RawFd; 10],
    shapes: [Shape; 10],
    roles: Roles,
}
fn capture(
    original: &OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) -> Result<Inventory, OriginalCustodyError> {
    let outputs = &original.original.transport.material.outputs;
    let custody = original
        .custody
        .as_ref()
        .ok_or(OriginalCustodyError::Binding)?;
    let fence = custody.original_descriptor(outputs.reservation())?;
    let alarm = original
        .original
        .alarm
        .as_ref()
        .ok_or(OriginalCustodyError::Binding)?;
    let [Some(stdout), Some(stderr)] = &outputs.pipes.pipes else {
        return Err(OriginalCustodyError::Binding);
    };
    let [Some(out_sink), Some(err_sink)] = &outputs.sinks else {
        return Err(OriginalCustodyError::Binding);
    };
    let message = original
        .original
        .transport
        .message
        .as_ref()
        .ok_or(OriginalCustodyError::Binding)?;
    let aliases = message.data();
    let aliases = aliases.fds();
    if aliases.len() != 2 {
        return Err(OriginalCustodyError::Binding);
    }
    let fds = [
        fence.as_raw_fd(),
        alarm.file.as_raw_fd(),
        stdout.reader.as_raw_fd(),
        stderr.reader.as_raw_fd(),
        out_sink.file.as_raw_fd(),
        err_sink.file.as_raw_fd(),
        stdout.writer.as_raw_fd(),
        stderr.writer.as_raw_fd(),
        aliases[0].as_raw_fd(),
        aliases[1].as_raw_fd(),
    ];
    let roles = roles(fds)?;
    let mut shapes = Vec::with_capacity(10);
    for (i, fd) in fds.iter().enumerate() {
        shapes.push(shape(*fd, i == 4 || i == 5)?);
    }
    Ok(Inventory {
        fds,
        shapes: shapes
            .try_into()
            .map_err(|_| OriginalCustodyError::Binding)?,
        roles,
    })
}

/// Private borrowed plan; neither a raw-FD API nor an execution capability.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::OriginalCustodiedPreparation;
/// fn denied(mut p: OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.plan_original_descriptors(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::OriginalDescriptorPlan;
/// fn denied<T: serde::de::DeserializeOwned>() {} denied::<OriginalDescriptorPlan>();
/// ```
pub(crate) struct OriginalDescriptorPlan<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original:
        &'held mut OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    inventory: Inventory,
    allow_fixture: bool,
}
impl OriginalDescriptorPlan<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(super) fn revalidate(&mut self) -> Result<(), OriginalCustodyError> {
        let result = (|| {
            self.original.validate(self.allow_fixture)?;
            if capture(self.original)? != self.inventory {
                return Err(OriginalCustodyError::Binding);
            }
            self.original.inspect_deadline()
        })();
        if result.is_err() {
            self.original.refuse();
        }
        result
    }
    #[cfg(test)]
    pub(crate) fn layout_for_test(&self) -> ([RawFd; 6], [RawFd; 4], [RawFd; 4], [u64; 6]) {
        (
            self.inventory.roles.guardian,
            self.inventory.roles.collector,
            self.inventory.roles.writers,
            self.original.origins_for_test(),
        )
    }
    #[cfg(test)]
    pub(crate) fn inherited_for_test(&self) -> ([RawFd; 6], [RawFd; 10]) {
        (self.inventory.roles.guardian, self.inventory.fds)
    }
    #[cfg(test)]
    pub(crate) fn hidden_writer_for_test(&self) -> File {
        self.original
            .original
            .transport
            .material
            .outputs
            .pipes
            .pipes[0]
            .as_ref()
            .unwrap()
            .writer
            .try_clone()
            .unwrap()
    }
    #[cfg(test)]
    pub(crate) fn revalidate_for_test(&mut self) -> Result<(), OriginalCustodyError> {
        self.revalidate()
    }
    #[cfg(test)]
    pub(crate) fn tamper_for_test(&mut self, kind: &str) {
        self.original.tamper_for_test(kind);
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(super) fn plan_original_descriptors(
        &mut self,
    ) -> Result<
        OriginalDescriptorPlan<'_, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalCustodyError,
    > {
        self.plan_descriptors_with(
            false,
            #[cfg(test)]
            |_| {},
        )
    }
    #[cfg(test)]
    pub(crate) fn plan_descriptors_fixture_for_test(
        &mut self,
        checkpoint: impl FnOnce(&mut Self),
    ) -> Result<
        OriginalDescriptorPlan<'_, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalCustodyError,
    > {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        self.plan_descriptors_with(true, checkpoint)
    }
    #[cfg(test)]
    pub(crate) fn shift_original_sink_for_test(&mut self) {
        let file = &self.original.transport.material.outputs.sinks[0]
            .as_ref()
            .unwrap()
            .file;
        assert_eq!(
            unsafe { libc::lseek(file.as_raw_fd(), 17, libc::SEEK_SET) },
            17
        );
    }
    fn plan_descriptors_with(
        &mut self,
        allow_fixture: bool,
        #[cfg(test)] checkpoint: impl FnOnce(&mut Self),
    ) -> Result<
        OriginalDescriptorPlan<'_, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalCustodyError,
    > {
        let result = (|| {
            if !self.ready || self.is_refused() {
                return Err(OriginalCustodyError::Refused);
            }
            let inventory = capture(self)?;
            #[cfg(test)]
            checkpoint(self);
            self.validate(allow_fixture)?;
            if capture(self)? != inventory {
                return Err(OriginalCustodyError::Binding);
            }
            self.inspect_deadline()?;
            Ok(inventory)
        })();
        match result {
            Ok(inventory) => Ok(OriginalDescriptorPlan {
                original: self,
                inventory,
                allow_fixture,
            }),
            Err(error) => {
                self.refuse();
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_descriptor_roles_exclude_all_four_writer_aliases() {
        let fds = [17, 3, 15, 8, 20, 12, 10, 6, 19, 5];
        let split = roles(fds).unwrap();
        assert_eq!(split.guardian, [3, 8, 12, 15, 17, 20]);
        assert_eq!(split.collector, [8, 12, 15, 20]);
        assert_eq!(split.writers, [5, 6, 10, 19]);
        assert!(split
            .writers
            .iter()
            .all(|fd| !split.guardian.contains(fd) && !split.collector.contains(fd)));
    }
    #[test]
    fn original_descriptor_roles_refuse_any_duplicate_or_negative_slot() {
        for i in 0..10 {
            let mut fds = std::array::from_fn(|n| n as RawFd + 3);
            fds[i] = -1;
            assert!(matches!(roles(fds), Err(OriginalCustodyError::Binding)));
            for j in 0..10 {
                if i != j {
                    let mut fds = std::array::from_fn(|n| n as RawFd + 3);
                    fds[i] = fds[j];
                    assert!(matches!(roles(fds), Err(OriginalCustodyError::Binding)));
                }
            }
        }
    }
    #[test]
    fn original_descriptor_capture_refuses_a_closed_fd_without_replacement() {
        assert!(shape(-1, false).is_err());
        assert!(shape(-1, true).is_err());
    }
    #[test]
    fn original_descriptor_capture_refuses_nonzero_sink_position() {
        let file = tempfile::tempfile().unwrap();
        assert_eq!(
            unsafe { libc::lseek(file.as_raw_fd(), 17, libc::SEEK_SET) },
            17
        );
        assert!(matches!(
            shape(file.as_raw_fd(), true),
            Err(OriginalCustodyError::Binding)
        ));
    }
}
