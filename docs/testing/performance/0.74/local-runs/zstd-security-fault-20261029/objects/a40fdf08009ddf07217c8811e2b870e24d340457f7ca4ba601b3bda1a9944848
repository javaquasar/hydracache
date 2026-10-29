//! Frozen complete bounded scalar projection parser.
use crate::diagnostic_kernel_admission::*;
pub(in crate::diagnostic_kernel_admission) fn projection(
    bytes: &[u8],
    credentials: &libc::ucred,
) -> Result<Projection, KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Projection,
    );
    let text = std::str::from_utf8(bytes).map_err(|_| kernel_invalid!("projection-utf8"))?;
    let groups_field = |prefix: &str| -> Result<Vec<u32>, KernelError> {
        let mut lines = text.lines().filter_map(|line| line.strip_prefix(prefix));
        let body = kernel_required!(lines.next(), "projection-missing-field")?;
        if lines.next().is_some() {
            return Err(kernel_invalid!("projection-duplicate-field"));
        }
        let values = body
            .split_ascii_whitespace()
            .take(33)
            .map(|v| v.parse::<u32>().map_err(|_| kernel_invalid!("proc-u32")))
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() > 32 {
            return Err(kernel_invalid!("projection-group-budget"));
        }
        Ok(values)
    };
    let scalar = |prefix: &str| -> Result<([u32; 33], usize), KernelError> {
        let mut lines = text.lines().filter_map(|line| line.strip_prefix(prefix));
        let body = kernel_required!(lines.next(), "projection-missing-field")?;
        if lines.next().is_some() {
            return Err(kernel_invalid!("projection-duplicate-field"));
        }
        let mut values = [0; 33];
        let mut len = 0;
        for value in body.split_ascii_whitespace().take(33) {
            values[len] = value
                .parse::<u32>()
                .map_err(|_| kernel_invalid!("proc-u32"))?;
            len += 1;
        }
        if len > 32 {
            return Err(kernel_invalid!("projection-group-budget"));
        }
        Ok((values, len))
    };
    let (values, len) = scalar("Pid:")?;
    if values[..len] != [credentials.pid as u32] {
        return Err(kernel_invalid!("projection-process-id"));
    }
    let (values, len) = scalar("Tgid:")?;
    if values[..len] != [credentials.pid as u32] {
        return Err(kernel_invalid!("projection-process-id"));
    }
    let (values, len) = scalar("Uid:")?;
    let uid: [u32; 4] = values[..len]
        .try_into()
        .map_err(|_| kernel_invalid!("projection-uid-width"))?;
    let (values, len) = scalar("Gid:")?;
    let gid: [u32; 4] = values[..len]
        .try_into()
        .map_err(|_| kernel_invalid!("projection-gid-width"))?;
    let groups = groups_field("Groups:")?;
    if uid != [credentials.uid; 4]
        || gid != [credentials.gid; 4]
        || groups.windows(2).any(|v| v[0] >= v[1])
    {
        return Err(kernel_invalid!("projection-credentials-or-groups"));
    }
    Ok(Projection { uid, gid, groups })
}
