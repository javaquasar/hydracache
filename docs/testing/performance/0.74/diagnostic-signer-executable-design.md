# Signer process executable object observation

An original asserted bundle can now borrow an existing ProcessRead and compare
its retained binary with the actual kernel `/proc/exe` object. Different-inode
exec refuses even when process generation still matches. This is not original
invocation or runtime integrity: an actual same-inode re-exec counterexample
still matches, so bootstrap/start/signing admission remains closed.

## Original objects and checks

`bind_asserted_process_executable` borrows the original bundle mutably and the
original ProcessRead immutably. A crate-private executable-only observer owns
two additional descriptors: a read-only clone of the expected binary and the
original procfs magic target opened O_PATH/CLOEXEC. Procfs/generation selection
uses the existing process reader; no public arbitrary PID, path or replacement
capture is added. Its current production scope remains diagnostic worker
cgroups, not controller-job enrollment. Tests use only the existing owned-helper
cfg(test) capture seam.

The shared legacy IO metadata comparison and fixed magic-link/read-only clone
helpers keep their predicates. Acquisition and rechecks bracket the original
pidfd/boot/PID/start ticks/group/cgroup. Expected, retained target and fresh
targets must agree in dev/inode, owner/mode/link count, length and mtime/ctime.
Bundle names/stamps/hashes are checked before and after that observation. The
wrapper takes no replacement arguments and exports no process, model, path,
file or FD; success returns only the existing borrowed receipt digest.

Every observed error latches wrapper and bundle refusal, including failed
initial binding and refusal retained after wrapper drop. Restoring public
files cannot repair them. Healthy drop releases additional observation handles
without signals or changes to the original process or healthy bundle. This is
runtime refusal, not a durable nonce or a persisted failed attempt.

## What the kernel object match cannot establish

The same-inode re-exec test executes the same copied shell a second time. Its
acknowledgement proves the controlled transition, while retained generation and
executable object comparison still succeed. Those observations therefore cannot
prove the original exec invocation or no intervening exec. Unobserved changes
between checks also remain possible; these are bracketed observations, not an
atomic future-execution guarantee.

`/proc/exe` identifies a kernel executable object, not the process's complete
instruction/runtime state. No executable pages, loader, shared libraries,
interpreter, JIT, preloads or bootstrap environment are authenticated here.
The signed source/profile/context and issuer/root selection remain assertions.
There is no authenticated build/job, trusted ancestry, production issuer/pins
or consumer that grants key/token/signing authority from this guard.

## Local tests and remaining integration

The [preregistration](diagnostic-signer-executable-plan-20261014.md) fixes owned
nonroot helpers and synthetic signatures. Tests copy `/bin/sh` into the fixture,
clear its environment and use a fixed stdin protocol to acknowledge, exec
`/bin/cat`, re-exec the same copy or exit on EOF. Only that child is owned; no
product/service/cgroup mutation or signal is performed. Five-second handshake
budgets bound a broken fixture, not a performance/correctness threshold.

Eight wrapper cases cover original positive/healthy drop, equal-byte foreign
inode/wrong process, different exec, same-inode counterexample, exit, bundle
drift/sticky refusal, CLOEXEC/descriptor release and seed `0x07403b24` with 16
fresh observations/four independent originals. One internal case covers invalid
expected descriptors. Four compile-fail cases keep clone/deserialization/FD
export/replacement recheck closed; the existing Linux API target adds the
borrowed method signature. Legacy IO and bundle regressions stay separate.

The next pre-exposure boundary needs independently reviewed runtime/bootstrap
custody and original invocation evidence, not a broadened meaning of this match.
Credential coordination, original receiver/replay/common fence/start, all-writer
cessation and later fixed cells remain before any separately admitted server,
pilot or qualification. The local counterexample must not be erased by promoting
one observed inode match to authorization.
