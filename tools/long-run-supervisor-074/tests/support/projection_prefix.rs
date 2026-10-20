//! Frozen b41a7fb6 projection and differential inputs; no live-state substitute.
use crate::diagnostic_kernel_admission::*;

pub(in crate::diagnostic_kernel_admission) fn reference(
    bytes: &[u8],
    credentials: &libc::ucred,
) -> Result<Projection, KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Projection,
    );
    let text = std::str::from_utf8(bytes).map_err(|_| kernel_invalid!("projection-utf8"))?;
    let field = |name: &str| -> Result<Vec<u32>, KernelError> {
        let prefix = format!("{name}:");
        let mut lines = text.lines().filter_map(|line| line.strip_prefix(&prefix));
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
    if field("Pid")? != [credentials.pid as u32] || field("Tgid")? != [credentials.pid as u32] {
        return Err(kernel_invalid!("projection-process-id"));
    }
    let uid: [u32; 4] = field("Uid")?
        .try_into()
        .map_err(|_| kernel_invalid!("projection-uid-width"))?;
    let gid: [u32; 4] = field("Gid")?
        .try_into()
        .map_err(|_| kernel_invalid!("projection-gid-width"))?;
    let groups = field("Groups")?;
    if uid != [credentials.uid; 4]
        || gid != [credentials.gid; 4]
        || groups.windows(2).any(|v| v[0] >= v[1])
    {
        return Err(kernel_invalid!("projection-credentials-or-groups"));
    }
    Ok(Projection { uid, gid, groups })
}

#[test]
fn projection_prefix_differential_seeded_bytes_and_first_refusal() {
    let credentials = libc::ucred {
        pid: 23,
        uid: 24,
        gid: 25,
    };
    let good = b"Pid: 23\nTgid: 23\nUid: 24 24 24 24\nGid: 25 25 25 25\nGroups: 26 27\n";
    fn compare(raw: &[u8], credentials: &libc::ucred) {
        let (old, old_site) = refusal_trace::capture(|| reference(raw, credentials));
        let (new, new_site) = refusal_trace::capture(|| projection(raw, credentials));
        assert_eq!(
            old.map_err(|e| e.to_string()),
            new.map_err(|e| e.to_string())
        );
        assert_eq!(
            old_site.map(|e| (e.site, e.clock)),
            new_site.map(|e| (e.site, e.clock))
        );
    }
    compare(good, &credentials);
    let mut seed = 0x0740_1d5e_u64;
    for _ in 0..1024 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut raw = good.to_vec();
        let index = seed as usize % raw.len();
        match (seed >> 32) % 3 {
            0 => {
                raw[index] = (seed >> 48) as u8;
            }
            1 => {
                raw.remove(index);
            }
            _ => {
                raw.insert(index, (seed >> 48) as u8);
            }
        }
        compare(&raw, &credentials);
    }
    for field in ["Pid", "Tgid", "Uid", "Gid", "Groups"] {
        for body in [
            "",
            "0",
            "-1",
            "+23",
            "4294967296",
            "DO_NOT_RETAIN",
            "24 24 24 24",
        ] {
            compare(
                format!("{}\n{field}: {body}\n", String::from_utf8_lossy(good)).as_bytes(),
                &credentials,
            );
        }
    }
    for count in [0, 1, 2, 31, 32, 33, 34] {
        let groups = (0..count)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let raw =
            String::from_utf8_lossy(good).replace("Groups: 26 27", &format!("Groups: {groups}"));
        compare(raw.as_bytes(), &credentials);
    }
    println!("HC074-PROJECTION-DIFFERENTIAL seed=0x07401d5e mutations=1024 original_refusals=true product=false");
}
