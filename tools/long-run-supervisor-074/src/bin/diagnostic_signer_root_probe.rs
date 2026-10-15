//! Fixed local preparation only: no child, signing or authority consumer.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "../diagnostic_signer_owned_root.rs"]
mod root;

fn main() {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let mut args = std::env::args_os().skip(1);
        if args.next().as_deref() == Some(std::ffi::OsStr::new("prepare"))
            && args.next().is_none()
            && root::prepare().is_ok()
        {
            println!("fixed root preparation probe passed; admission=false");
            return;
        }
    }
    eprintln!("Fixed root preparation refused.");
    std::process::exit(2);
}
