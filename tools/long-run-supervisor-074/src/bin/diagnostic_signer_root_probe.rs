//! Fixed local root diagnostics only: no signing or authority consumer.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "../diagnostic_signer_owned_root.rs"]
mod root;

fn main() {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let mut args = std::env::args_os().skip(1);
        let mode = args.next();
        if args.next().is_none() {
            if mode.as_deref() == Some(std::ffi::OsStr::new("--root-child")) {
                root::rooted_child();
            }
            if mode.as_deref() == Some(std::ffi::OsStr::new("prepare")) && root::prepare().is_ok() {
                println!("fixed root preparation probe passed; admission=false");
                return;
            }
            if mode.as_deref() == Some(std::ffi::OsStr::new("root-exec"))
                && root::rooted_probe().is_ok()
            {
                println!("selected rooted self-exec observed; admission=false");
                return;
            }
        }
    }
    eprintln!("Fixed root preparation refused.");
    std::process::exit(2);
}
