use super::*;
struct Prefix {
    bytes: Vec<u8>,
    remaining: usize,
    interrupted: bool,
    zero: bool,
}
impl Write for Prefix {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if self.interrupted {
            self.interrupted = false;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        if self.zero {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::Error::other("fixture full storage"));
        }
        let n = input.len().min(self.remaining).min(2);
        self.bytes.extend_from_slice(&input[..n]);
        self.remaining -= n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn prefix(remaining: usize) -> Prefix {
    Prefix {
        bytes: vec![],
        remaining,
        interrupted: false,
        zero: false,
    }
}
#[test]
fn interrupted_and_partial_log_writes_preserve_exact_retained_prefix_hashes() {
    let mut out = prefix(3);
    out.interrupted = true;
    let mut err = prefix(10);
    let mut logs = LogCapture::new(&mut out, &mut err);
    let failure = logs.append(b"abcdef", b"stderr", 100).unwrap_err();
    assert!(failure.journal && !failure.aggregate);
    assert_eq!((logs.out.bytes, logs.err.bytes), (3, 0));
    assert_eq!(logs.hashes(), (hash(b"abc"), hash(b"")));
    assert_eq!(out.bytes, b"abc");
    assert!(err.bytes.is_empty());
    let mut out = prefix(10);
    let mut err = prefix(1);
    let mut logs = LogCapture::new(&mut out, &mut err);
    assert!(logs.append(b"out", b"err", 100).is_err());
    assert_eq!((logs.out.bytes, logs.err.bytes), (3, 1));
    assert_eq!(logs.hashes(), (hash(b"out"), hash(b"e")));
    assert_eq!(out.bytes, b"out");
    assert_eq!(err.bytes, b"e");
}
#[test]
fn empty_zero_and_aggregate_exhaustion_never_fabricate_retained_bytes() {
    let mut out = prefix(10);
    out.zero = true;
    let mut err = prefix(10);
    let mut logs = LogCapture::new(&mut out, &mut err);
    logs.append(b"", b"", 0).unwrap();
    let failure = logs.append(b"x", b"", 1).unwrap_err();
    assert!(failure.journal);
    assert_eq!(logs.hashes(), (hash(b""), hash(b"")));
    let mut logs = LogCapture::new(Vec::new(), Vec::new());
    let failure = logs.append(b"123", b"456", 5).unwrap_err();
    assert!(failure.aggregate && !failure.journal);
    assert_eq!((logs.out.bytes, logs.err.bytes), (0, 0));
    logs.append(b"123", b"456", 6).unwrap();
    assert_eq!(logs.hashes(), (hash(b"123"), hash(b"456")));
}
#[test]
fn cleanup_failure_has_priority_over_journal_cancellation_and_output_failures() {
    assert_eq!(
        failure_outcome(FailureKind::Cleanup, true, true, true),
        BuildOutcome::CleanupFailed
    );
    assert_eq!(
        failure_outcome(FailureKind::Io, true, true, true),
        BuildOutcome::JournalFailed
    );
    assert_eq!(
        failure_outcome(FailureKind::Timeout, false, true, true),
        BuildOutcome::Cancelled
    );
    assert_eq!(
        failure_outcome(FailureKind::Timeout, false, false, true),
        BuildOutcome::OutputLimit
    );
    for (kind, expected) in [
        (FailureKind::Timeout, BuildOutcome::TimedOut),
        (FailureKind::StdoutLimit, BuildOutcome::OutputLimit),
        (FailureKind::StderrLimit, BuildOutcome::OutputLimit),
        (FailureKind::Io, BuildOutcome::Failed),
    ] {
        assert_eq!(failure_outcome(kind, false, false, false), expected);
    }
    assert_eq!(
        remaining_budget_from(Ok("1100".into()), 100).unwrap(),
        Duration::from_secs(1)
    );
    assert_eq!(
        remaining_budget_from(Err(std::env::VarError::NotPresent), 0).unwrap(),
        Duration::from_secs(2400)
    );
    assert!(remaining_budget_from(
        Err(std::env::VarError::NotUnicode(
            "invalid deadline fixture".into()
        )),
        0
    )
    .is_err());
}
