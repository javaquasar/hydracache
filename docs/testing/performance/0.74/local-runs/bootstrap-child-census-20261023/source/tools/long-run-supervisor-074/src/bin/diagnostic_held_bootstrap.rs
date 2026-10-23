//! Nonexecuting bootstrap reader. Original context authentication is still closed.
#[path = "../diagnostic_bootstrap_child_ingress.rs"]
mod ingress;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "../diagnostic_bootstrap_child_reader.rs"]
mod reader;

#[path = "../diagnostic_bootstrap_census_entry.rs"]
mod census_entry;

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
#[path = "../../tests/support/bootstrap_child_material.rs"]
mod bootstrap_child_material;

fn main() {
    if census_entry::requested() {
        if let Err(error) = census_entry::run() {
            eprintln!("Bootstrap refused: {error}; no start, send or executable handoff.");
        }
        std::process::exit(2);
    }
    let result = (|| -> std::io::Result<()> {
        ingress::inspect(
            &std::env::args_os().skip(1).take(2).collect::<Vec<_>>(),
            &std::env::vars_os().take(4).collect::<Vec<_>>(),
            std::process::id(),
        )?;
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            let observed = reader::read(14)?;
            reader::revalidate(14, &observed)?;
            // A forged sealed assertion can reach here. Nothing here establishes
            // credentials, manager, guardians, root, role census or build trust.
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "original bootstrap authentication is closed",
            ))
        }
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Linux x86-64 required",
        ))
    })();
    if let Err(error) = result {
        eprintln!("Bootstrap refused: {error}; no start, send or executable handoff.");
    }
    std::process::exit(2);
}
