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
            if let Some(name) = mode.as_deref().and_then(std::ffi::OsStr::to_str) {
                if name.starts_with("root-loss-") && root::creator_loss(name).is_ok() {
                    println!("selected rooted creator-loss observation passed; admission=false");
                    return;
                }
            }
            for (mode_name, kill) in [
                ("root-kill-installed", true),
                ("root-drop-installed", false),
            ] {
                if mode.as_deref() == Some(std::ffi::OsStr::new(mode_name))
                    && root::installed_cleanup(kill).is_ok()
                {
                    println!("selected rooted installed cleanup observed; admission=false");
                    return;
                }
            }
            use hydracache_long_run_supervisor_074::diagnostic_signer_isolation::FixedProbe;
            for (public, child, probe) in [
                ("root-native", "--root-native-child", FixedProbe::Native),
                ("root-x32", "--root-x32-child", FixedProbe::X32),
                ("root-i386", "--root-i386-child", FixedProbe::I386),
            ] {
                if mode.as_deref() == Some(std::ffi::OsStr::new(child)) {
                    root::isolated_child(probe);
                }
                if mode.as_deref() == Some(std::ffi::OsStr::new(public))
                    && root::isolated_probe(probe).is_ok()
                {
                    println!("selected rooted isolation observed; admission=false");
                    return;
                }
            }
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
