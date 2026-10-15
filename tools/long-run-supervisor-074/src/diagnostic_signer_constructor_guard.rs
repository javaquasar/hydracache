//! Whole original-parent loss before the ptrace EXITKILL handover, fixture only.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum Cut {
    BeforeArm,
    Armed,
    Checked,
    Traceme,
    Stopped,
}
pub(super) const CUTS: [Cut; 5] = [
    Cut::BeforeArm,
    Cut::Armed,
    Cut::Checked,
    Cut::Traceme,
    Cut::Stopped,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum Fault {
    None,
    NoArm,
    NoParentCheck,
    NoCreatorCheck,
}
pub(super) struct Hook {
    pub(super) cut: Cut,
    pub(super) fault: Fault,
    pub(super) parent: File,
    pub(super) child: File,
    pub(super) observe: fn(&RootedExec, &Hook) -> io::Result<()>,
}
pub(super) fn checkpoint(hook: Option<&Hook>, cut: Cut, raw: i32) {
    let Some(hook) = hook.filter(|h| h.cut == cut) else {
        return;
    };
    let token = cut as u8;
    let mut received = [255u8; 2];
    let mut poll = libc::pollfd {
        fd: raw,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: prepared original private descriptor, initialized bounded stack data, scalar libc only.
    unsafe {
        if libc::send(
            raw,
            (&token as *const u8).cast(),
            1,
            libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
        ) != 1
            || libc::poll(&mut poll, 1, 5000) != 1
            || poll.revents & (libc::POLLIN | libc::POLLHUP) == 0
            || poll.revents & (libc::POLLERR | libc::POLLNVAL) != 0
        {
            libc::_exit(85);
        }
        let n = libc::recv(
            raw,
            received.as_mut_ptr().cast(),
            received.len(),
            libc::MSG_DONTWAIT | libc::MSG_TRUNC,
        );
        if n == 0 {
            // Closed negative controls terminate here or after the missed parent check, never exec.
            if hook.fault == Fault::NoArm {
                libc::_exit(83);
            }
            return;
        }
        if n != 1 || received[0] != 1 {
            libc::_exit(85);
        }
    }
}
pub(super) fn arm(
    original_parent: libc::pid_t,
    hook: Option<&Hook>,
    creator: &super::creator_task::Creator,
) {
    if let Some(hook) = hook {
        // SAFETY: raw child must not retain the opposite gate endpoint and hide parent EOF.
        if unsafe { libc::close(hook.parent.as_raw_fd()) } != 0 {
            unsafe {
                libc::_exit(85);
            }
        }
    }
    let raw = hook.map_or(-1, |h| h.child.as_raw_fd());
    checkpoint(hook, Cut::BeforeArm, raw);
    let fault = hook.map_or(Fault::None, |h| h.fault);
    // SAFETY: original fork child only; no allocations, destructors, locks or caller-selected PID.
    unsafe {
        if fault != Fault::NoArm {
            let mut installed = -1i32;
            if libc::prctl(
                libc::PR_SET_PDEATHSIG,
                libc::SIGKILL as usize,
                0usize,
                0usize,
                0usize,
            ) != 0
                || libc::prctl(
                    libc::PR_GET_PDEATHSIG,
                    &mut installed,
                    0usize,
                    0usize,
                    0usize,
                ) != 0
                || installed != libc::SIGKILL
            {
                libc::_exit(82);
            }
        }
        checkpoint(hook, Cut::Armed, raw);
        if fault == Fault::NoParentCheck {
            libc::_exit(84);
        }
        if libc::getppid() != original_parent {
            libc::_exit(81);
        }
        if fault == Fault::NoCreatorCheck {
            // Counterfactual terminates only after the old TGID check passed, before exec.
            libc::_exit(86);
        }
        if !super::creator_task::live(creator.task.as_raw_fd(), creator.tgid, creator.tid) {
            libc::_exit(81);
        }
        checkpoint(hook, Cut::Checked, raw);
    }
}
