# Original validation frame: incomplete local candidate

The operator-approved refactoring is implemented in the working copy, not yet
accepted or committed. Its live-fault gate was interrupted by backwards WSL
realtime. Preserve that failure; do not push the runtime candidate or proceed to
Starting publication as if the gate passed. The preregistration is commit
fc374ea72624a5757456970aa56f2b86d185c18f, after baseline 47372f9f.

## What changed

The consuming `into_validation_frame` adapter retains the complete original
consent owner and common fence. Each validation has an explicit working round
and an unhooked closing round. It reuses the unchanged lower live predicates;
it does not cache signatures, observations or authorization. Construction and
validation failures refuse the complete chain and retain the fence until drop.
No new file, process, output, service or durable acknowledgement is created.

The original budget fixture remains byte-exact. The candidate copy changes only
the consuming adapter, test name and diagnostic labels. The complete frame's
34 IPC authentications describe its composition, not product goodput or an
exclusive cost model. The old refusal and its 264-authentication observation
remain in the separate historical packet.

## All preregistered budget chains

Each cell contains three fresh chains in one test process, requesting nineteen
publication-free checks. They are not independently launched product pairs.

| Profile | Rust 1.94 completed | Rust 1.88 completed |
| --- | --- | --- |
| Dev, optimized Ed25519/curve dependencies only | 6, 8, 11 | 10, 10, 11 |
| Normal release | 19, 19, 5 | 19, 19, 19 |

Every dev chain expires. The short release chain refuses `clock-order`, not a
changed budget: realtime moves backwards about four seconds while boottime
advances. Its failure remains. The observation test's exit zero means refusal
was correctly retained. Its `admitted=true` field means nineteen checks finished,
never runtime or server admission. Actual writes, sealing and fsyncs have not
been measured. Neither profile alone proves an installed publication budget.

## Fault gate and clock blocker

The Rust 1.94 release matrix reports five completed cases, all at cut zero:
worker trust drift, OIDC trust drift, same-byte controller trust rewrite,
unexpected authority-root context and removed Reserved. All refuse and retain
the Busy common fence. The next frame fails construction on a backwards
realtime step. The remaining cuts, five old-reference cases, healthy order,
construction-failure and concurrent controls were **not reached**. The Rust 1.88
manual fault matrix was not run. The source contains these cases, but source
coverage is not execution evidence. The authority-context fault is not proof of
a real signed epoch advance.

Seventy-five read-only clock samples show realtime steps relative to uptime.
The later context observes enabled/synchronized WSL timesyncd, stopped Windows
W32Time/vmictimesync, and clocksource tsc. This supports an environmental clock
problem, not a unique causal attribution. No host clock or time service was
changed. A separately labelled environmental control needs approval and must
retain these original failures, change no runtime predicate or admission origin,
and complete a fresh full matrix without selecting successful cases.

## Completed checks and retained failures

Both compilers pass two fast tests and six compile-fail API tests; the two manual
tests remain ignored in the fast lane. Both pass the unchanged 55 IPC/lease/
original-fence/host integrations. Windows/Linux affected-package check and strict
Clippy pass, including xtask Clippy. All 183 static contracts pass. Full workspace
formatting, docs, 176 discovered registered gates and the local non-promotable
performance contract pass. Ship admission is expected-red; full release verify
was not run and no milestone, merge or release is claimed.

The missing-API red result precedes implementation. An initial static run
retains a double-normalization bug in the new historical adapter; only that
adapter was corrected before the green repeat. One intervening Windows link
failed because the older static executable was still running. Its diagnostic
was truncated by the tool; the capture says so and is not complete stderr.
The first combined static/fmt capture also retains Windows cargo-fmt error 206;
the final WSL formatting check passes. Do not relabel these failures as green.

## Packet verification and provenance

Run `powershell -NoProfile -File ./verify.ps1` from any directory. The read-only
verifier checks nine selected checkout-byte hashes, 23 unchanged lower sources,
their Git content, forty captured command observations, every budget summary and
successful check index, exact partial fault coverage and closed scope flags.
The selected candidate is explicitly an **uncommitted working-copy snapshot**;
new Git object ids are computed content names, not claims that objects or a
candidate commit were published. Later changes should cause working-hash refusal.
`candidate.patch` preserves the nine source-file changes as unaccepted data;
its SHA256 is cfeb99884063d51dfdaea98f23c3a0529a25bc19acae54c529f1ac7c99f4377f.
The verifier also checks that hash. A non-mutating reverse `git apply --check`
passes against the current working sources; this is not a code acceptance gate.

Captures contain tool-returned text segments, not a byte-exact process stream.
Release executable hashes are unsigned local observations after the runs, not
build custody or installation receipts. No server, qualification inputs, clocks,
deadline, product workload or current 0.73 artifact was modified. Durable Starting,
new SIGKILL/lost-ack tests, authenticated start/all-writer cessation, later cells
and signed bounded host rehearsal remain open.

The verifier passes from the parent directory. Two separate manifests outside
Git deliberately change `server_ready` or the failed release sample's count;
both refuse. `verification-results.json` retains those tool observations. The
canaries neither edit the retained manifest nor rerun a runtime experiment.
