//! Pinned diagnostic PKI, not production trust or host enrollment.
use super::{MtlsFixture, TransportReceipt};
use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    RootCertStore,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::Arc,
};

const MAX_BYTES: u64 = 65536;

// Deliberately no Debug. This is the only serialization containing private keys.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Material {
    schema_version: u32,
    ca: String,
    server_cert: String,
    server_key: String,
    client_cert: String,
    client_key: String,
}

/// Public-only receipt. Equality is a necessary coordinator check, not admission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MaterialSeal {
    pub material_sha256: String,
    pub transport: TransportReceipt,
    pub production_enrollment: bool,
}

fn private_regular(metadata: &std::fs::Metadata) -> Result<(), String> {
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("material must be a regular nonsymlink file".to_owned());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o077 != 0 || metadata.nlink() != 1 {
            return Err("material requires private permissions and one link".to_owned());
        }
    }
    Ok(())
}

fn same_file(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(not(unix))]
    {
        // Raw-byte pinning remains mandatory; no Unix permission claim on Windows.
        a.len() == b.len() && a.modified().ok() == b.modified().ok()
    }
}

impl MtlsFixture {
    /// Create once in the coordinator's private directory. Never log these bytes.
    /// On Unix both file and directory publication are synced before returning.
    pub fn save_new(&self, path: &Path) -> Result<MaterialSeal, String> {
        self.validate_material()?;
        let material = Material {
            schema_version: 1,
            ca: self.ca.clone(),
            server_cert: self.server_cert.clone(),
            server_key: self.server_key.clone(),
            client_cert: self.client_cert.clone(),
            client_key: self.client_key.clone(),
        };
        let raw =
            serde_json::to_vec(&material).map_err(|_| "material encoding failed".to_owned())?;
        if raw.len() as u64 > MAX_BYTES {
            return Err("material exceeds 64 KiB".to_owned());
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path).map_err(|e| e.to_string())?;
        file.write_all(&raw).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        #[cfg(unix)]
        File::open(
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )
        .and_then(|parent| parent.sync_all())
        .map_err(|e| e.to_string())?;
        Ok(MaterialSeal {
            material_sha256: format!("{:x}", Sha256::digest(&raw)),
            transport: self.receipt(),
            production_enrollment: false,
        })
    }

    /// Read one bounded file through a retained descriptor. Verify raw bytes
    /// before decoding; path replacement or a different certificate/key refuses.
    pub fn read_pinned(path: &Path, expected: &str) -> Result<(Self, MaterialSeal), String> {
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("invalid material SHA256 pin".to_owned());
        }
        let before = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        private_regular(&before)?;
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let opened = file.metadata().map_err(|e| e.to_string())?;
        private_regular(&opened)?;
        if !same_file(&before, &opened) {
            return Err("material path changed during open".to_owned());
        }
        let mut raw = Vec::new();
        (&mut file)
            .take(MAX_BYTES + 1)
            .read_to_end(&mut raw)
            .map_err(|e| e.to_string())?;
        if raw.len() as u64 > MAX_BYTES {
            return Err("material exceeds 64 KiB".to_owned());
        }
        if format!("{:x}", Sha256::digest(&raw)) != expected {
            return Err("material raw-byte seal mismatch".to_owned());
        }
        let after = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        private_regular(&after)?;
        if !same_file(&opened, &after) {
            return Err("material path changed during read".to_owned());
        }
        let material: Material = serde_json::from_slice(&raw)
            .map_err(|_| "invalid material schema or encoding".to_owned())?;
        if material.schema_version != 1 {
            return Err("unsupported material schema".to_owned());
        }
        let fixture = Self {
            ca: material.ca,
            server_cert: material.server_cert,
            server_key: material.server_key,
            client_cert: material.client_cert,
            client_key: material.client_key,
        };
        fixture.validate_material()?;
        let seal = MaterialSeal {
            material_sha256: expected.to_owned(),
            transport: fixture.receipt(),
            production_enrollment: false,
        };
        Ok((fixture, seal))
    }

    fn validate_material(&self) -> Result<(), String> {
        // with_single_cert and with_client_auth_cert check matching private keys.
        // The real fixture handshake still validates issuer/name/EKU and AUTH.
        // Metadata modes run before a server can install a process-wide provider.
        // Select ring locally; do not change the legacy transport's global choice.
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut roots = RootCertStore::empty();
        roots
            .add(
                CertificateDer::from_pem_slice(self.ca.as_bytes())
                    .map_err(|_| "invalid root certificate".to_owned())?,
            )
            .map_err(|_| "invalid root certificate".to_owned())?;
        let client_cert = CertificateDer::from_pem_slice(self.client_cert.as_bytes())
            .map_err(|_| "invalid client certificate".to_owned())?;
        let client_key = PrivateKeyDer::from_pem_slice(self.client_key.as_bytes())
            .map_err(|_| "invalid client key".to_owned())?;
        rustls::ClientConfig::builder_with_provider(Arc::clone(&provider))
            .with_safe_default_protocol_versions()
            .map_err(|_| "invalid validation protocols".to_owned())?
            .with_root_certificates(roots)
            .with_client_auth_cert(vec![client_cert], client_key)
            .map_err(|_| "invalid client certificate/key pair".to_owned())?;
        let cert = CertificateDer::from_pem_slice(self.server_cert.as_bytes())
            .map_err(|_| "invalid server certificate".to_owned())?;
        let key = PrivateKeyDer::from_pem_slice(self.server_key.as_bytes())
            .map_err(|_| "invalid server key".to_owned())?;
        rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| "invalid validation protocols".to_owned())?
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .map_err(|_| "invalid server certificate/key pair".to_owned())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, raw: &[u8]) -> String {
        std::fs::write(path, raw).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        format!("{:x}", Sha256::digest(raw))
    }

    #[test]
    fn two_reads_roundtrip_exact_material_without_secret_receipts_or_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("material.json");
        let original = MtlsFixture::new().unwrap();
        let seal = original.save_new(&path).unwrap();
        assert!(original.save_new(&path).is_err());
        for _ in 0..2 {
            let (loaded, receipt) = MtlsFixture::read_pinned(&path, &seal.material_sha256).unwrap();
            assert_eq!(receipt, seal);
            assert_eq!(loaded.receipt(), original.receipt());
            let json = serde_json::to_string(&receipt).unwrap();
            for secret in [&original.server_key, &original.client_key] {
                assert!(!json.contains(secret));
            }
            assert!(!json.contains("PRIVATE KEY"));
            assert!(!receipt.production_enrollment);
        }
        assert!(MtlsFixture::read_pinned(&path, &"0".repeat(64)).is_err());
        assert!(MtlsFixture::read_pinned(&path, &"A".repeat(64)).is_err());
        assert!(MtlsFixture::read_pinned(dir.path(), &seal.material_sha256).is_err());
    }

    #[test]
    fn malformed_unknown_oversized_and_mismatched_keys_refuse_without_panics() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("material.json");
        MtlsFixture::new().unwrap().save_new(&path).unwrap();
        let original: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let foreign = MtlsFixture::new().unwrap();
        for (field, value) in [
            ("schema_version", serde_json::json!(2)),
            ("unknown", serde_json::json!(true)),
            ("ca", serde_json::json!("not a certificate")),
            ("server_cert", serde_json::json!("invalid")),
            ("server_key", serde_json::json!(foreign.server_key)),
            ("client_key", serde_json::json!(foreign.client_key)),
        ] {
            let mut changed = original.clone();
            changed[field] = value;
            let pin = write(&path, &serde_json::to_vec(&changed).unwrap());
            assert!(MtlsFixture::read_pinned(&path, &pin).is_err(), "{field}");
        }
        for raw in [b"not JSON".to_vec(), vec![b' '; MAX_BYTES as usize + 1]] {
            let pin = write(&path, &raw);
            assert!(MtlsFixture::read_pinned(&path, &pin).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn public_permissions_symlinks_and_hardlinks_are_not_private_material() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("material.json");
        let seal = MtlsFixture::new().unwrap().save_new(&path).unwrap();
        let alias = dir.path().join("alias");
        symlink(&path, &alias).unwrap();
        assert!(MtlsFixture::read_pinned(&alias, &seal.material_sha256).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(MtlsFixture::read_pinned(&path, &seal.material_sha256).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::hard_link(&path, dir.path().join("hardlink")).unwrap();
        assert!(MtlsFixture::read_pinned(&path, &seal.material_sha256).is_err());
    }
}
