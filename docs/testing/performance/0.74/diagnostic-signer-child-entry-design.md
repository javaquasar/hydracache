# Fixed ordinary child restrictions, not original launch admission

The normal library now contains the existing fixed x86_64 seccomp policy, a
private Linux installer, and a non-returning public fixed-probe entry. The normal
`diagnostic_signer_isolation_probe` binary selects only `native`, `x32` or `i386`.
It accepts no executable, PID, descriptor, key, filter, root or command override.
It is a secret-free nonproduct diagnostic, not a signer or production launcher.

The 64 instructions/512 bytes must equal the independent old namespace fixture's
generator. Neither that generator nor its standalone helper source is replaced.
Eight exec/task/ptrace/seccomp calls and 15 namespace/root/mount calls return
EPERM. Non-x86_64 audit architecture and x32 syscall numbers kill the process.
The old prctl tail permits only PR_SET_DUMPABLE(0), checking both full-width
arguments; the four unused prctl arguments are not constrained. Other syscalls
are allowed: this denial policy is deliberately **not a complete sandbox**.

The installer refuses nonempty environment, numerical root UID/GID, unequal
credential slots, capabilities, multiple threads, non-main TID or FDs beyond
stdio. It reads bounded current-thread procfs status and a live FD inventory,
checking procfs filesystem type. It does not prove initial-host namespace,
independent job custody, account enrollment, loader provenance, instruction
integrity, absence of shared mappings or prior runtime execution. Root and
multi-thread negatives are parser models; no privileged root process is launched.

In an owned fresh child, it sets both core limits to zero, sets/verifies
no_new_privs, requires exact zero from fixed-filter TSYNC, then requires ND0.
It rechecks credentials, single-thread/TID, FDs, NNP1, Seccomp2 and exactly one
additional filter. At most 64 pre-existing filters are supported; the bounded
decoder accepts 65 only for the post-install slot. Any failure terminates this
child, including partial installation. No reusable installed-guard object or
success return to caller exists. A five-second owned-child timeout is a test
harness bound, not a production watchdog.

Native probes check exact EPERM on 20 fixed safe calls. They never issue
fork/vfork/clone/clone3; those policy numbers remain model/old-fixture coverage.
Invalid pointers/FDs cannot exec or mount if the policy is broken. The only
positive control is chdir('/') in a separate owned raw-fork child that uses
only libc chdir/_exit after fork. The filtered child requires EPERM for that
same call. x32/i386 probes must actually terminate SIGSYS with empty output and
no core; native success prints one fixed line with `admission=false`. Tests
also check inherited-FD/environment/argument refusal and unchanged parent
seccomp/core/cwd state across four independent owners.

A repeat initially failed on preflight descriptors: temporary diagnosis observed
FDs {0,1,2,3,7}, with 3 the probe's own proc directory. The source of inherited
FD7 was not authenticated. The refusal was correct; rerunning without fixing
the harness could produce green results. The owned test launcher now marks
all child-only FDs above stdio CLOEXEC before exec, requiring close_range support
without fallback. The extra-FD negative clears that flag on exactly its private
child copy *after* the sweep, so the normal probe must still refuse it. Parent
flags are untouched. Temporary diagnostic output is removed from ordinary code;
the failed captures remain evidence, not a waived guard.

There is no selected executable/root, original-owner handshake, authenticated
first exec, FD3 handoff, credential read, controller start or all-writer fence in
this entry. In particular, stdio-only refusal makes it unsuitable for the old
fixture's FD3 handoff as-is. Next work must compose an owned original launcher
with explicit root/runtime/descriptor lifecycle, parent loss and sticky refusal;
it cannot treat this probe's stdout as an admission receipt. Server installation,
signing, completion, later cells, qualification and ship remain closed. Product
code, old helper images, frozen manifest/lock inputs and thresholds are unchanged.

See the [preregistration](diagnostic-signer-child-entry-plan-20261015.md) and
[contract](diagnostic-signer-child-entry-contract.toml). Local kernel results do
not establish dedicated-server compatibility or cache performance.
