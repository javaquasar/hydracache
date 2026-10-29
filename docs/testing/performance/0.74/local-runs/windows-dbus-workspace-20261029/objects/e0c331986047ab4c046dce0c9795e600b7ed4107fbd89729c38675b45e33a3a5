//! Bounded GNU ELF declarations and byte digests, never runtime or signing authority.
use serde::Serialize;
use std::collections::BTreeSet;
use std::io;

#[path = "diagnostic_signer_runtime_elf.rs"]
pub(crate) mod elf;
pub(crate) const NAMES: [&str; 4] = [
    "signer",
    "ld-linux-x86-64.so.2",
    "libc.so.6",
    "libgcc_s.so.1",
];
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "GNU signer declaration catalog refused",
    )
}

#[derive(Debug, Serialize)]
struct Image {
    role: &'static str,
    sha256: String,
    bytes: usize,
    interpreter: Option<String>,
    needed: Vec<String>,
    soname: Option<String>,
    elf_kind: u16,
    program_headers: usize,
}

/// Serialize-only observation data. No reader, trust loader, signing or start consumer.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_gnu::GnuDeclarationCatalog;
/// fn admit(catalog: GnuDeclarationCatalog) { catalog.into_bootstrap(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_gnu::GnuDeclarationCatalog;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<GnuDeclarationCatalog>();
/// ```
#[derive(Debug, Serialize)]
pub struct GnuDeclarationCatalog {
    schema_version: &'static str,
    images: Vec<Image>,
    complete_elf_validation: bool,
    loaded_closure_proved: bool,
    runtime_custody_authenticated: bool,
    original_start_authenticated: bool,
    signing_allowed: bool,
    admission_allowed: bool,
}

pub(crate) fn graph(info: &[elf::Info]) -> io::Result<()> {
    if info.len() != 4
        || info[0].interpreter.as_deref() != Some(elf::LOADER)
        || info[0].soname.is_some()
        || info[1].interpreter.is_some()
        || !info[1].needed.is_empty()
    {
        return Err(invalid());
    }
    for index in 1..4 {
        if info[index].soname.as_deref() != Some(NAMES[index]) {
            return Err(invalid());
        }
    }
    let mut reached = BTreeSet::from([0usize, 1]);
    for _ in 0..4 {
        for (index, item) in info.iter().enumerate() {
            for name in &item.needed {
                let next = NAMES
                    .iter()
                    .position(|candidate| candidate == name)
                    .ok_or_else(invalid)?;
                if next == 0 || next == index {
                    return Err(invalid());
                }
                if reached.contains(&index) {
                    reached.insert(next);
                }
            }
        }
    }
    if reached.len() != 4 {
        return Err(invalid());
    }
    Ok(())
}

/// Four fixed roles: signer, GNU loader, libc and libgcc. Inputs remain assertions.
/// No filesystem, dynamic loader, relocation, symbol/version or mapped-memory check.
pub fn inspect_gnu_declarations(images: [&[u8]; 4]) -> io::Result<GnuDeclarationCatalog> {
    if images
        .iter()
        .any(|b| b.len() < 64 || b.len() > elf::MAX_IMAGE)
    {
        return Err(invalid());
    }
    let info = images
        .iter()
        .enumerate()
        .map(|(i, bytes)| elf::parse(bytes, i == 0))
        .collect::<io::Result<Vec<_>>>()?;
    graph(&info)?;
    let images = info
        .into_iter()
        .enumerate()
        .map(|(i, info)| Image {
            role: NAMES[i],
            sha256: crate::sha256_hex(images[i]),
            bytes: images[i].len(),
            interpreter: info.interpreter,
            needed: info.needed,
            soname: info.soname,
            elf_kind: info.kind,
            program_headers: info.program_headers,
        })
        .collect();
    Ok(GnuDeclarationCatalog {
        schema_version: "diagnostic-signer-gnu-declarations-074-v1",
        images,
        complete_elf_validation: false,
        loaded_closure_proved: false,
        runtime_custody_authenticated: false,
        original_start_authenticated: false,
        signing_allowed: false,
        admission_allowed: false,
    })
}
