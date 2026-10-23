//! Uses the old sealed constructor. No descriptor extraction or new time query.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::{
    entry::EntryBindings, BootstrapImageBindings,
};
pub(in super::super) struct LegacyContribution {
    pub(in super::super) config: [u64; 3],
    pub(in super::super) legacy: [u64; 3],
    pub(in super::super) stdin: [u64; 2],
    pub(in super::super) channel: [u64; 2],
    pub(in super::super) hash: [u8; 32],
}
impl Resources {
    pub(in super::super) fn entry_contribution(
        &self,
        b: &Bindings,
    ) -> io::Result<LegacyContribution> {
        let before = stamp(self.bootstrap.as_raw_fd())?;
        readonly(&self.bootstrap)?;
        let config = id(&self.config)?;
        let stdin = pair_id(&self.stdin)?;
        let channel = pair_id(&self.release[1])?;
        let expected = bootstrap(b, &self.token, stdin, config, channel)?;
        need(
            read(&self.bootstrap, BOOT_BYTES)? == expected
                && stamp(self.bootstrap.as_raw_fd())? == before,
        )?;
        use sha2::Digest;
        Ok(LegacyContribution {
            config,
            legacy: id(&self.bootstrap)?,
            stdin,
            channel,
            hash: sha2::Sha256::digest(expected).into(),
        })
    }
}
pub(in super::super) struct EntryResource {
    file: File,
    retained: Stamp,
}

#[path = "diagnostic_bootstrap_installed_entry_resource.rs"]
pub(in super::super) mod installed_material;
impl EntryResource {
    pub(in super::super) fn create(raw: &[u8; 1024]) -> io::Result<Self> {
        let file = sealed(raw)?;
        let retained = stamp(file.as_raw_fd())?;
        need(retained.position == Some(0))?;
        Ok(Self { file, retained })
    }
    pub(in super::super) fn inspect(
        &self,
        raw: &[u8; 1024],
        base: &BootstrapImageBindings,
    ) -> io::Result<EntryBindings> {
        need(stamp(self.file.as_raw_fd())? == self.retained)?;
        readonly(&self.file)?;
        need(read(&self.file, 1024)? == raw)?;
        let entry = base.append_entry(&self.file)?;
        need(stamp(self.file.as_raw_fd())? == self.retained)?;
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bootstrap_entry_legacy_contribution_uses_actual_sealed_config_and_rejects_token_drift() {
        // Deliberately no live owner/creator proof: only this private value leaf.
        let config = sealed(&[7; 64]).unwrap();
        let stdin = stdin().unwrap();
        let release = release().unwrap();
        let b = Bindings {
            owner: [74, 75, 1000, 1000, 1000, 1000],
            deadline: 10,
            guardians: [(76, [1, 2]), (77, [1, 3])],
            worker: ([1, 4, 64], "a".repeat(64)),
            config: ([99, 98, 64], crate::sha256_hex(&[7; 64])),
            streams: [[1, 6], [1, 7]],
        };
        let token = [8; 32];
        let raw = bootstrap(
            &b,
            &token,
            pair_id(&stdin).unwrap(),
            id(&config).unwrap(),
            pair_id(&release[1]).unwrap(),
        )
        .unwrap();
        let mut resource = Resources {
            stdin,
            release,
            config,
            bootstrap: sealed(&raw).unwrap(),
            creator: tempfile::tempfile().unwrap(),
            token,
            retained: vec![],
        };
        let contribution = resource.entry_contribution(&b).unwrap();
        assert_eq!(contribution.config, id(&resource.config).unwrap());
        assert_ne!(contribution.config, b.config.0);
        assert_eq!(contribution.legacy[2], 256);
        assert_eq!(contribution.stdin, pair_id(&resource.stdin).unwrap());
        assert_eq!(contribution.channel, pair_id(&resource.release[1]).unwrap());
        use sha2::Digest;
        assert_eq!(
            contribution.hash,
            <[u8; 32]>::from(sha2::Sha256::digest(raw))
        );
        resource.token[0] ^= 1;
        assert!(resource.entry_contribution(&b).is_err());
    }
    #[test]
    fn bootstrap_entry_resource_is_readonly_exact_sealed_and_position_retained() {
        let raw = [74; 1024];
        let r = EntryResource::create(&raw).unwrap();
        readonly(&r.file).unwrap();
        assert_eq!(read(&r.file, 1024).unwrap(), raw);
        assert_eq!(r.retained.position, Some(0));
        assert_eq!(
            unsafe { libc::pwrite(r.file.as_raw_fd(), raw.as_ptr().cast(), 1, 0) },
            -1
        );
        assert_eq!(unsafe { libc::ftruncate(r.file.as_raw_fd(), 1) }, -1);
        assert_eq!(stamp(r.file.as_raw_fd()).unwrap(), r.retained);
    }
    #[test]
    fn bootstrap_entry_resource_every_byte_mismatch_same_bytes_replacement_and_offset_refuse() {
        use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::{
            collect, tests::Fixture,
        };
        let f = Fixture::new();
        let base = collect(
            &f.prior(),
            f.original(),
            f.images.each_ref(),
            f.identities(),
        )
        .unwrap();
        let raw = [74; 1024];
        let mut r = EntryResource::create(&raw).unwrap();
        assert!(r.inspect(&raw, &base).is_ok());
        for at in 0..1024 {
            let mut bad = raw;
            bad[at] ^= 1;
            assert!(r.inspect(&bad, &base).is_err());
        }
        let other = sealed(&raw).unwrap();
        r.file = other;
        assert!(r.inspect(&raw, &base).is_err());
        let r = EntryResource::create(&raw).unwrap();
        assert_eq!(
            unsafe { libc::lseek(r.file.as_raw_fd(), 1, libc::SEEK_SET) },
            1
        );
        assert!(r.inspect(&raw, &base).is_err());
    }
}
