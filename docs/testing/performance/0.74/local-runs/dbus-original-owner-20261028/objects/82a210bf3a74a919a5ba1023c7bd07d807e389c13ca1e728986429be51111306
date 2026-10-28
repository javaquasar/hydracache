//! Public selfcheck inventory only. No issuer enrollment, signing or execution authority.
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;

const PROBE: &[u8] =
    b"hydracache-diagnostic-signer-build-key-selfcheck-074-v1\0not-a-build-receipt\n";
const OLD: [&str; 3] = [
    "c56f89595c45d0656225f1b8915babed046d278dc9ce1908c6bcb6e9c93e2fda",
    "39215088b0102dffb987b937efa9f49a6c5c7704192fa1f16b4e19ccad12c3ba",
    "e7bd73d40638095eb7fdb3d03eec3c36af688d185fd750b83fc3ee81683a2f59",
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Public {
    schema_version: String,
    algorithm: String,
    purpose: String,
    public_key_hex: String,
    public_key_sha256: String,
    selfcheck_message_hex: String,
    selfcheck_signature_hex: String,
    storage_uid: u32,
    directory_mode: String,
    private_key_mode: String,
    storage_format: String,
    signer_build_statement_sha256: Option<String>,
    public_pin_approved: bool,
    production_trust_enrolled: bool,
    signer_build_receipt_signed: bool,
    workflow_installed: bool,
    admission_allowed: bool,
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}
fn decode<const N: usize>(s: &str) -> Result<[u8; N], ()> {
    if s.len() != N * 2
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(());
    }
    let mut out = [0u8; N];
    for (i, pair) in s.as_bytes().chunks_exact(2).enumerate() {
        let nibble = |b| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
        out[i] = (nibble(pair[0]) << 4) | nibble(pair[1]);
    }
    Ok(out)
}
fn canonical(p: &Public) -> Result<Vec<u8>, ()> {
    let value = serde_json::to_value(p).map_err(|_| ())?;
    let mut raw = serde_json::to_vec(&value).map_err(|_| ())?;
    raw.push(b'\n');
    Ok(raw)
}
fn verify(raw: &[u8]) -> Result<String, ()> {
    if raw.is_empty() || raw.len() > 4096 {
        return Err(());
    }
    let p: Public = serde_json::from_slice(raw).map_err(|_| ())?;
    if canonical(&p)? != raw
        || p.schema_version != "diagnostic-signer-build-issuer-selection-074-v1"
        || p.algorithm != "Ed25519"
        || p.purpose != "diagnostic-signer-build-receipt-only"
        || p.selfcheck_message_hex != hex(PROBE)
        || p.storage_uid == 0
        || p.directory_mode != "0700"
        || p.private_key_mode != "0600"
        || p.storage_format != "unencrypted-pkcs8-pem"
        || p.signer_build_statement_sha256.is_some()
        || p.public_pin_approved
        || p.production_trust_enrolled
        || p.signer_build_receipt_signed
        || p.workflow_installed
        || p.admission_allowed
        || OLD.contains(&p.public_key_hex.as_str())
    {
        return Err(());
    }
    let raw_key = decode::<32>(&p.public_key_hex)?;
    if p.public_key_sha256 != hex(&Sha256::digest(raw_key)) {
        return Err(());
    }
    let key = VerifyingKey::from_bytes(&raw_key).map_err(|_| ())?;
    if key.is_weak() {
        return Err(());
    }
    let signature = Signature::from_bytes(&decode::<64>(&p.selfcheck_signature_hex)?);
    key.verify_strict(PROBE, &signature).map_err(|_| ())?;
    Ok(p.public_key_sha256)
}
fn run() -> Result<String, ()> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or(())?;
    if args.next().is_some() {
        return Err(());
    }
    let path = std::path::Path::new(&path);
    if !std::fs::symlink_metadata(path).map_err(|_| ())?.is_file() {
        return Err(());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    let file = options.open(path).map_err(|_| ())?;
    let metadata = file.metadata().map_err(|_| ())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 4096 {
        return Err(());
    }
    let mut raw = Vec::new();
    file.take(4097).read_to_end(&mut raw).map_err(|_| ())?;
    verify(&raw)
}
fn main() {
    match run() {
        Ok(fingerprint) => println!("public_signer_selfcheck=verified raw_public_sha256={fingerprint} pin_approved=false production_enrolled=false"),
        Err(()) => { eprintln!("public signer selfcheck refused; no enrollment"); std::process::exit(1); }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    fn fixture() -> Public {
        let key = SigningKey::from_bytes(&[75; 32]);
        Public {
            schema_version: "diagnostic-signer-build-issuer-selection-074-v1".into(),
            algorithm: "Ed25519".into(),
            purpose: "diagnostic-signer-build-receipt-only".into(),
            public_key_hex: hex(&key.verifying_key().to_bytes()),
            public_key_sha256: hex(&Sha256::digest(key.verifying_key().to_bytes())),
            selfcheck_message_hex: hex(PROBE),
            selfcheck_signature_hex: hex(&key.sign(PROBE).to_bytes()),
            storage_uid: 1000,
            directory_mode: "0700".into(),
            private_key_mode: "0600".into(),
            storage_format: "unencrypted-pkcs8-pem".into(),
            signer_build_statement_sha256: None,
            public_pin_approved: false,
            production_trust_enrolled: false,
            signer_build_receipt_signed: false,
            workflow_installed: false,
            admission_allowed: false,
        }
    }
    #[test]
    fn synthetic_public_proof_is_not_an_approved_pin() {
        let p = fixture();
        assert_eq!(
            verify(&canonical(&p).unwrap()).unwrap(),
            p.public_key_sha256
        );
        for old in OLD {
            let mut p = fixture();
            p.public_key_hex = old.into();
            assert!(verify(&canonical(&p).unwrap()).is_err());
        }
    }
    #[test]
    fn each_field_and_every_signature_byte_is_bound() {
        let p = fixture();
        let value = serde_json::to_value(&p).unwrap();
        for name in value.as_object().unwrap().keys() {
            let mut changed = value.clone();
            let field = &mut changed[name];
            *field = match field {
                serde_json::Value::Bool(_) => serde_json::json!(true),
                serde_json::Value::Number(_) => serde_json::json!(0),
                _ => serde_json::json!("changed"),
            };
            let mut raw = serde_json::to_vec(&changed).unwrap();
            raw.push(b'\n');
            assert!(verify(&raw).is_err(), "{name}");
        }
        for index in 0..64 {
            let mut p = fixture();
            let mut sig = decode::<64>(&p.selfcheck_signature_hex).unwrap();
            sig[index] ^= 1;
            p.selfcheck_signature_hex = hex(&sig);
            assert!(verify(&canonical(&p).unwrap()).is_err());
        }
    }
    #[test]
    fn unknown_duplicate_noncanonical_and_oversized_reports_refuse() {
        let raw = canonical(&fixture()).unwrap();
        for bad in [
            Vec::new(),
            vec![b' '; 4097],
            raw[..raw.len() - 1].to_vec(),
            [b" ".as_slice(), &raw].concat(),
            [b"{\"algorithm\":\"Ed25519\",".as_slice(), &raw[1..]].concat(),
            [b"{\"future\":false,".as_slice(), &raw[1..]].concat(),
        ] {
            assert!(verify(&bad).is_err());
        }
        for value in ["", "00", "GG", &"f".repeat(64)] {
            assert!(decode::<64>(value).is_err());
        }
    }
}
