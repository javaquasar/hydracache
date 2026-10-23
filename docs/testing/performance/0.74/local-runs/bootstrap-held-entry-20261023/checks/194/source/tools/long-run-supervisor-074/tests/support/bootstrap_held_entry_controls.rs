//! Owned unprivileged readiness fixtures, never an original manager attempt.
use super::*;
use hydracache_long_run_supervisor_074::{diagnostic_kernel_admission::KernelError, protocol::MAX_PACKET_BYTES};
use std::os::fd::OwnedFd;
#[path = "../../src/diagnostic_kernel_ancillary.rs"]
mod ancillary;
const HELD_ARG: &str = "--systemd-bootstrap-held-readiness-v1";
const NAMES: &str = "release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry:installed-image:installed-entry:readiness";
const INVOCATION: &str = "0102030405060708090a0b0c0d0e0f10";
struct Running(std::process::Child);
impl Drop for Running {
    fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
}
fn pair() -> [OwnedFd;2] {
    let mut fds = [-1;2];
    assert_eq!(unsafe { libc::socketpair(libc::AF_UNIX,libc::SOCK_SEQPACKET|libc::SOCK_CLOEXEC|libc::SOCK_NONBLOCK,0,fds.as_mut_ptr()) },0);
    let pair = fds.map(|fd|unsafe { OwnedFd::from_raw_fd(fd) });
    for fd in &pair { ancillary::configure(fd).unwrap(); }
    pair
}
fn spawn(f:&Installed, readiness:&OwnedFd, fault:&str) -> Running {
    let mut sources = f.old.fds().to_vec();
    sources.extend([f.image.as_raw_fd(),f.record.as_raw_fd(),readiness.as_raw_fd()]);
    if fault == "swap" { sources.swap(3,17); }
    let scratch:Vec<File> = sources.into_iter().map(|fd| {
        let n = unsafe { libc::fcntl(fd,libc::F_DUPFD_CLOEXEC,64) }; assert!(n>=64); unsafe { File::from_raw_fd(n) }
    }).collect();
    let numbers:Vec<_> = scratch.iter().map(AsRawFd::as_raw_fd).collect();
    let extra = fault == "extra";
    let missing = fault == "missing";
    let offset = fault == "offset";
    let script = if fault == "pid" { "unset PWD; LISTEN_PID=1; SYSTEMD_EXEC_PID=$$; export LISTEN_PID SYSTEMD_EXEC_PID; exec \"$@\"" }
        else { "unset PWD; LISTEN_PID=$$; SYSTEMD_EXEC_PID=$$; export LISTEN_PID SYSTEMD_EXEC_PID; exec \"$@\"" };
    let mut c = Command::new("/bin/sh");
    c.env_clear().args(["-c",script,"held-entry-control"]).arg(if fault=="inode" { std::path::PathBuf::from(BINARY) } else { f.path() })
        .arg(if fault=="arg" { "--systemd-bootstrap-held-readiness-v0" } else { HELD_ARG })
        .env("LISTEN_FDS",if fault=="count" {"14"} else {"15"})
        .env("LISTEN_FDNAMES",if fault=="names" {"readiness"} else {NAMES})
        .env("INVOCATION_ID",if fault=="invocation-env" {"00000000000000000000000000000000"} else {INVOCATION})
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if fault=="foreign-env" { c.env("UNEXPECTED","1"); }
    if fault=="missing-env" { c.env_remove("INVOCATION_ID"); }
    unsafe { c.pre_exec(move || {
        for (i,fd) in numbers.iter().enumerate() {
            if libc::dup2(*fd,i as i32)<0 { return Err(std::io::Error::last_os_error()); }
        }
        if extra && libc::dup2(numbers[4],20)<0 { return Err(std::io::Error::last_os_error()); }
        if libc::syscall(libc::SYS_close_range,18u32,u32::MAX,libc::CLOSE_RANGE_CLOEXEC)!=0 { return Err(std::io::Error::last_os_error()); }
        if extra && libc::fcntl(20,libc::F_SETFD,0)!=0 { return Err(std::io::Error::last_os_error()); }
        if missing && libc::close(17)!=0 { return Err(std::io::Error::last_os_error()); }
        if offset && libc::lseek(16,1,libc::SEEK_SET)!=1 { return Err(std::io::Error::last_os_error()); }
        Ok(())
    }); }
    Running(c.spawn().unwrap())
}
fn drain(f:&File) -> Vec<u8> {
    let mut file = f;
    let mut raw=Vec::new(); let mut b=[0;1024];
    loop { match file.read(&mut b) {
        Ok(0)=>break, Ok(n)=>{raw.extend_from_slice(&b[..n]); assert!(raw.len()<=8192);},
        Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>break, Err(e)=>panic!("{e}"),
    } }
    raw
}
fn finish(child:&mut Running,f:&Installed,code:Option<i32>) {
    let until=Instant::now()+Duration::from_secs(5);
    while child.0.try_wait().unwrap().is_none() && Instant::now()<until { std::thread::sleep(Duration::from_millis(5)); }
    let status=child.0.try_wait().unwrap();
    let stderr=String::from_utf8(drain(&f.old.retained[2])).unwrap();
    println!("held-entry child stderr: {stderr}");
    assert!(drain(&f.old.retained[1]).is_empty(),"no worker output");
    assert!(status.is_some(),"bounded child timeout; first result: {stderr}");
    assert_eq!(status.unwrap().code(),code,"{stderr}");
}
// Independent byte oracle: not the runtime Challenge encoder or ACK method.
fn challenge(f:&Installed,pid:u32) -> [u8;256] {
    let mut raw=[0;256]; raw[..8].copy_from_slice(b"HCHR0741");
    raw[8..16].copy_from_slice(&f.old.entry.bounds[3].to_le_bytes());
    for (i,v) in [f.old.entry.owner[0] as u32,f.old.entry.owner[4] as u32,f.old.entry.owner[5] as u32,pid].into_iter().enumerate() {
        raw[16+4*i..20+4*i].copy_from_slice(&v.to_le_bytes());
    }
    raw[32..64].fill(0x74);
    let record=std::fs::read(format!("/proc/thread-self/fd/{}",f.record.as_raw_fd())).unwrap();
    raw[64..96].copy_from_slice(&Sha256::digest(record));
    raw[96..128].copy_from_slice(&f.old.entry.original_context);
    raw[128..160].copy_from_slice(&f.old.entry.names[0]); raw[160..192].copy_from_slice(&f.old.entry.names[1]);
    for i in 0..16 { raw[192+i]=(i+1) as u8; }
    raw
}
fn send(fd:&OwnedFd,raw:&[u8]) {
    assert_eq!(unsafe { libc::send(fd.as_raw_fd(),raw.as_ptr().cast(),raw.len(),libc::MSG_DONTWAIT|libc::MSG_NOSIGNAL) },raw.len() as isize);
}
fn await_ack(fd:&OwnedFd,child:&mut Running,f:&Installed,raw:&[u8]) {
    let mut poll=libc::pollfd {fd:fd.as_raw_fd(),events:libc::POLLIN,revents:0};
    let result=unsafe {libc::poll(&mut poll,1,5000)};
    // Capture diagnostics before assertions; RAII still reaps on any failure.
    let stderr=String::from_utf8(drain(&f.old.retained[2])).unwrap();
    println!("held-entry before ACK stderr: {stderr}");
    assert_eq!(result,1,"{stderr}");
    let (ack,cred,pidfd)=ancillary::receive(fd).unwrap().expect("one kernel-authenticated ACK");
    assert_eq!([cred.pid as u32,cred.uid,cred.gid],[child.0.id(),unsafe{libc::geteuid()},unsafe{libc::getegid()}]);
    let mut p=libc::pollfd{fd:pidfd.as_raw_fd(),events:libc::POLLIN,revents:0};
    assert_eq!(unsafe{libc::poll(&mut p,1,0)},0);
    let mut expected=[0;80]; expected[..8].copy_from_slice(b"HCHA0741"); expected[8..16].copy_from_slice(&80u64.to_le_bytes());
    let mut hash=Sha256::new(); hash.update(b"hydracache-held-readiness-074-v1\0"); hash.update(raw);
    expected[16..48].copy_from_slice(&hash.finalize()); expected[48..].copy_from_slice(&raw[32..64]);
    assert_eq!(ack,expected); assert!(child.0.try_wait().unwrap().is_none());
    assert!(drain(&f.old.retained[1]).is_empty());
}
#[test]
fn held_entry_valid_readiness_then_duplicate_refuses_without_worker() {
    let f=Installed::new(); let [parent,endpoint]=pair(); let mut child=spawn(&f,&endpoint,"");
    let raw=challenge(&f,child.0.id()); send(&parent,&raw); await_ack(&parent,&mut child,&f,&raw);
    send(&parent,&raw); finish(&mut child,&f,Some(2));
    assert!(ancillary::receive(&parent).is_err() || ancillary::receive(&parent).unwrap().is_none());
}
#[test]
fn held_entry_activation_and_census_faults_refuse_without_ack() {
    for fault in ["foreign-env","missing-env","pid","count","names","invocation-env","arg","extra","missing","swap","offset","inode"] {
        println!("held-entry first fault: {fault}");
        let f=Installed::new(); let [parent,endpoint]=pair(); let mut child=spawn(&f,&endpoint,fault);
        finish(&mut child,&f,Some(2));
        assert!(ancillary::receive(&parent).unwrap().is_none());
    }
}
#[test]
fn held_entry_context_entry_invocation_deadline_reserved_and_pid_faults_refuse() {
    for at in [8,28,64,96,128,160,192,208] {
        let f=Installed::new(); let [parent,endpoint]=pair(); let mut child=spawn(&f,&endpoint,"");
        let mut raw=challenge(&f,child.0.id()); raw[at]^=1; send(&parent,&raw);
        finish(&mut child,&f,Some(2)); assert!(ancillary::receive(&parent).unwrap().is_none());
    }
}
#[test]
fn held_entry_guardian_loss_and_peer_disconnect_stop_nonexecuting_hold() {
    for guardian in [true,false] {
        let mut f=Installed::new(); let [parent,endpoint]=pair(); let mut child=spawn(&f,&endpoint,"");
        let raw=challenge(&f,child.0.id()); send(&parent,&raw); await_ack(&parent,&mut child,&f,&raw);
        if guardian { f.old.children[0].kill().unwrap(); f.old.children[0].wait().unwrap(); }
        else { assert_eq!(unsafe{libc::shutdown(parent.as_raw_fd(),libc::SHUT_RDWR)},0); }
        finish(&mut child,&f,Some(2));
    }
}
#[test]
fn held_entry_sigkill_before_and_after_unconsumed_ack_is_not_reexecution() {
    for sent in [false,true] {
        let f=Installed::new(); let [parent,endpoint]=pair(); let mut child=spawn(&f,&endpoint,"");
        if sent {
            send(&parent,&challenge(&f,child.0.id()));
            let mut p=libc::pollfd{fd:parent.as_raw_fd(),events:libc::POLLIN,revents:0};
            assert_eq!(unsafe{libc::poll(&mut p,1,5000)},1,"first lost-ACK observation");
        }
        child.0.kill().unwrap(); finish(&mut child,&f,None);
        // Never start another child, resend or reinterpret a queued ACK as completion.
        assert!(drain(&f.old.retained[1]).is_empty());
    }
}
#[test]
fn held_entry_original_alarm_expires_without_rearming() {
    let f=Installed::new(); let [parent,endpoint]=pair(); let mut child=spawn(&f,&endpoint,"");
    let raw=challenge(&f,child.0.id()); send(&parent,&raw); await_ack(&parent,&mut child,&f,&raw);
    let until=Instant::now()+Duration::from_secs(12);
    while child.0.try_wait().unwrap().is_none() && Instant::now()<until { std::thread::sleep(Duration::from_millis(10)); }
    finish(&mut child,&f,Some(2));
}
