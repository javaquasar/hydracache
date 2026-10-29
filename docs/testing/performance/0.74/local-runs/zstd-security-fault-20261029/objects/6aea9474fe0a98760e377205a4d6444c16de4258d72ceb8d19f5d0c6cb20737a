//! Only new, exactly marked fixture units on the real manager cgroup tree.
use super::*;
use std::time::{Duration, Instant};

pub(super) struct Scope {
    slice: String,
    service: String,
    marker: String,
    owned: bool,
}
pub(super) struct Group {
    pub(super) name: CString,
    pub(super) parent: File,
    path: PathBuf,
    pub(super) guardians: Option<[File; 2]>,
    service: String,
    slice: String,
}
fn show(unit: &str, property: &str) -> String {
    let output = Command::new("/usr/bin/systemctl")
        .args([
            "--system",
            "--no-pager",
            "show",
            "--value",
            "--property",
            property,
            unit,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success() || output.status.code() == Some(4),
        "{output:?}"
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
fn absent(unit: &str) -> bool {
    // systemctl show may synthesize/load a slice definition. GetUnit does not.
    let output = Command::new("/usr/bin/busctl")
        .env("LC_ALL", "C")
        .args([
            "--system",
            "--timeout=5",
            "call",
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
            "GetUnit",
            "s",
            unit,
        ])
        .output()
        .unwrap();
    lookup_absent(unit, output.status.code(), &output.stdout, &output.stderr).unwrap()
}
fn lookup_absent(
    unit: &str,
    code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> std::io::Result<bool> {
    if code == Some(0) {
        return Ok(false);
    } // Loaded/unknown positive result never admits creation.
    if code == Some(1)
        && stdout.is_empty()
        && stderr == format!("Call failed: Unit {unit} not loaded.\n").as_bytes()
    {
        return Ok(true);
    }
    Err(std::io::Error::other(
        "manager lookup is uncertain; refuse without load or start",
    ))
}
fn action(verb: &str, unit: &str) {
    let output = Command::new("/usr/bin/systemctl")
        .args(["--system", "--no-pager", verb, unit])
        .output()
        .unwrap();
    assert!(output.status.success(), "{verb} {unit}: {output:?}");
}
fn retire(service: &str, slice: &str) {
    if absent(service) {
        return;
    }
    assert_eq!(show(service, "Transient"), "yes");
    assert_eq!(
        show(service, "Description"),
        "HydraCache held diagnostic projection; no transport"
    );
    assert_eq!(show(service, "Slice"), slice);
    action("stop", service);
    if !absent(service) && show(service, "ActiveState") == "failed" {
        action("reset-failed", service);
    }
    let until = Instant::now() + Duration::from_secs(5);
    while !absent(service) {
        assert_eq!(show(service, "MainPID"), "0");
        assert!(Instant::now() < until, "owned service not unloaded");
        std::thread::sleep(Duration::from_millis(10));
    }
}
impl Scope {
    pub(super) fn create(attempt: [u8; 32], lease: &str) -> Self {
        assert_eq!(lease.len(), 64);
        assert!(lease
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        let mut scope = Self {
            slice: format!("system-hc074{}.slice", crate::hex(&attempt)),
            service: format!("hydracache-diagnostic-074-{lease}-1.service"),
            marker: format!("HC074 isolated original service {}", crate::hex(&attempt)),
            owned: false,
        };
        assert!(
            absent(&scope.slice) && absent(&scope.service),
            "never reuse an existing unit"
        );
        assert!(!Path::new("/sys/fs/cgroup/system.slice")
            .join(&scope.slice)
            .exists());
        scope.owned = true; // Only after all absence guards; uncertain start is cleanup-only.
        println!(
            "HC074-ORIGINAL-SCOPE-INTENT slice={} service={} marker={}",
            scope.slice, scope.service, scope.marker
        );
        let output = Command::new("/usr/bin/busctl")
            .args([
                "--system",
                "--timeout=5",
                "call",
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
                "StartTransientUnit",
                "ssa(sv)a(sa(sv))",
                &scope.slice,
                "fail",
                "2",
                "Description",
                "s",
                &scope.marker,
                "StopWhenUnneeded",
                "b",
                "false",
                "0",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let until = Instant::now() + Duration::from_secs(5);
        while show(&scope.slice, "ActiveState") != "active" {
            assert!(Instant::now() < until, "owned slice did not become active");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(show(&scope.slice, "Transient"), "yes");
        assert_eq!(show(&scope.slice, "Description"), scope.marker);
        assert_eq!(
            show(&scope.slice, "ControlGroup"),
            format!("/system.slice/{}", scope.slice)
        );
        println!(
            "HC074-ORIGINAL-SCOPE slice={} service={} temporary=true",
            scope.slice, scope.service
        );
        scope
    }
    pub(super) fn cgroup(&self) -> Group {
        let path = Path::new("/sys/fs/cgroup/system.slice").join(&self.slice);
        assert_eq!(
            fs::read(path.join("cgroup.events")).unwrap(),
            b"populated 0\nfrozen 0\n"
        );
        fs::create_dir(path.join("nested")).unwrap();
        Group {
            name: CString::new(self.slice.clone()).unwrap(),
            parent: OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open("/sys/fs/cgroup/system.slice")
                .unwrap(),
            path,
            guardians: None,
            service: self.service.clone(),
            slice: self.slice.clone(),
        }
    }
    pub(super) fn retire_service(&self) {
        retire(&self.service, &self.slice);
    }
}
impl Group {
    pub(super) fn empty(&self) {
        assert_eq!(
            fs::read(self.path.join("cgroup.events")).unwrap(),
            b"populated 0\nfrozen 0\n"
        );
    }
}
impl Drop for Group {
    fn drop(&mut self) {
        // Original owner and borrowed RootSource drop first, including refusal.
        // Cleanup is not evidence of autonomous writer cessation.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            retire(&self.service, &self.slice);
            if let Some(pins) = self.guardians.take() {
                for pin in pins {
                    cleanup_guardian(&pin);
                }
            }
            self.empty();
            fs::remove_dir(self.path.join("nested")).unwrap();
        }));
        println!("HC074-ORIGINAL-GROUP cleanup_ok={}", result.is_ok());
        if !std::thread::panicking() {
            result.unwrap();
        }
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        if !self.owned {
            return;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.retire_service();
            if !absent(&self.slice) {
                assert_eq!(show(&self.slice, "Transient"), "yes");
                assert_eq!(show(&self.slice, "Description"), self.marker);
                action("stop", &self.slice);
            }
            assert!(absent(&self.service) && absent(&self.slice));
        }));
        println!("HC074-ORIGINAL-SCOPE cleanup_ok={}", result.is_ok());
        if !std::thread::panicking() {
            result.unwrap();
        }
    }
}

#[test]
fn original_service_nonowner_drop_never_contacts_manager() {
    drop(Scope {
        slice: "preexisting.slice".into(),
        service: "preexisting.service".into(),
        marker: "not-owned".into(),
        owned: false,
    });
}

#[test]
fn original_service_lookup_requires_exact_missing_unit_and_refuses_uncertainty() {
    let unit = "system-hc074-test.slice";
    let missing = format!("Call failed: Unit {unit} not loaded.\n");
    assert!(lookup_absent(unit, Some(1), b"", missing.as_bytes()).unwrap());
    assert!(!lookup_absent(
        unit,
        Some(0),
        b"o \"/org/freedesktop/systemd1/unit/test\"\n",
        b""
    )
    .unwrap());
    for (code, stdout, stderr) in [
        (None, b"".as_slice(), missing.as_bytes()),
        (Some(2), b"", missing.as_bytes()),
        (Some(1), b"unexpected", missing.as_bytes()),
        (Some(1), b"", b"Call failed: Access denied.\n"),
        (Some(1), b"", b"Call failed: Unit other.slice not loaded.\n"),
        (Some(1), b"", b"Call failed: Connection refused.\n"),
    ] {
        assert!(lookup_absent(unit, code, stdout, stderr).is_err());
    }
}
