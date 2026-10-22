# First unit refusal and source representation repair

The first test_view.py setup materialized the Git archive, then the isolated
worker refused with ValueError: Qualification drift in reconcile.py. The unit
runner reported Ran 0 tests and FAILED (errors=1), after 25.687 seconds. This is
an observed test-run report, not a performance measurement. No formal first
replay receipt or old facts were replaced. TemporaryDirectory retired the
generated view after the setup failure.

Initial source and inputs are retained under initial-source/. The cause was
source-file newline representation: the archive emitted 3068 CRLF bytes while
the original qualification blob/worktree contains 2979 LF bytes. The initial
explanation reversed that direction and was corrected after raw comparison.
The separately preregistered text-image adapter preserves canonical source
content and reconstructs only the explicitly pinned UTF-8 LF source images.
Packet/evidence bytes and the qualification file in the worktree
are unchanged. The first refusal remains a negative development result.

This note records the tool-reported exception and unit summary; no raw first
stdout/stderr file was captured by the initial invocation. Subsequent source-
identified checks have their own logs, not a replacement for that first output.
