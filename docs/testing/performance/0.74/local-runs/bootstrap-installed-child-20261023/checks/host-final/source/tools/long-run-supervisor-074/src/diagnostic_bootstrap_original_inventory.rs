//! Opaque value contribution from the old authenticated inventory, no FD getter.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::OriginalFdSet;

pub(crate) fn capture(inventory: &[Stamp]) -> io::Result<OriginalFdSet> {
    need(inventory.len() == 20)?;
    let mut values = [-1; 20];
    for (at, entry) in inventory.iter().enumerate() {
        need(
            entry.descriptor == libc::FD_CLOEXEC
                && entry.status >= 0
                && entry.status & libc::O_PATH == 0,
        )?;
        values[at] = entry.fd;
    }
    OriginalFdSet::from_material(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inventory() -> Vec<Stamp> {
        (0..20)
            .map(|fd| Stamp {
                fd: fd + 74,
                stat: [0; 11],
                descriptor: libc::FD_CLOEXEC,
                status: libc::O_RDONLY,
                position: None,
            })
            .collect()
    }
    #[test]
    fn bootstrap_image_original_inventory_requires_all_exact_flags_and_twenty_entries() {
        assert!(capture(&inventory()).is_ok());
        for length in [0, 19, 21] {
            let mut values = inventory();
            values.resize_with(length, || Stamp {
                fd: 99,
                stat: [0; 11],
                descriptor: 0,
                status: 0,
                position: None,
            });
            assert!(capture(&values).is_err());
        }
        for at in 0..20 {
            let mut values = inventory();
            values[at].descriptor = 0;
            assert!(capture(&values).is_err());
            let mut values = inventory();
            values[at].status = libc::O_PATH;
            assert!(capture(&values).is_err());
            let mut values = inventory();
            values[at].fd = -1;
            assert!(capture(&values).is_err());
        }
        for i in 0..20 {
            for j in 0..i {
                let mut values = inventory();
                values[i].fd = values[j].fd;
                assert!(capture(&values).is_err());
            }
        }
    }
}
