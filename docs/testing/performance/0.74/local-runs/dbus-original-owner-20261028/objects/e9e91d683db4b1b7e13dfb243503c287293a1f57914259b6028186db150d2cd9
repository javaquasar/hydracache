//! Frozen complete preceding projection parser.
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
    let field = |prefix: &str| -> Result<Vec<u32>, KernelError> {
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
    if field("Pid:")? != [credentials.pid as u32] || field("Tgid:")? != [credentials.pid as u32] {
        return Err(kernel_invalid!("projection-process-id"));
    }
    let uid: [u32; 4] = field("Uid:")?
        .try_into()
        .map_err(|_| kernel_invalid!("projection-uid-width"))?;
    let gid: [u32; 4] = field("Gid:")?
        .try_into()
        .map_err(|_| kernel_invalid!("projection-gid-width"))?;
    let groups = field("Groups:")?;
    if uid != [credentials.uid; 4]
        || gid != [credentials.gid; 4]
        || groups.windows(2).any(|v| v[0] >= v[1])
    {
        return Err(kernel_invalid!("projection-credentials-or-groups"));
    }
    Ok(Projection { uid, gid, groups })
}
