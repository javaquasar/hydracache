# Creator loss in the original send windows

Baseline is ff02c0ca3f95f34b06b358fa0458753faf51ebd7. This local W11 slice adds
two creator-side cuts to the existing ordinary-root public-dummy experiment:
after the sole attempt latch before any send dispatch, and after successful
send16 before resuming the original recv-entry stop. The original worker,
root/exec/policy path and original monitor objects remain the same.

## Fixed state and controls

Separate attempt, send-dispatch and successful-send facts. A sticky dispatch
flag must be set before the actual syscall and must remain set even on EPIPE;
neither retry nor a second attempt can be recovered. Private continuation
values retain only the original observed stack pointer. At both new cuts the
worker is still at the same original recv-entry stop, not after delivery.

Register cuts11/12 in the private version1 frame. Each has creator SIGKILL loss,
healthy H continuation and a no-EXITKILL control: six new modes. Old cuts and
14-mode classification stay unchanged. New no-EXITKILL modes use mode3 and do
not inject a group-stop or signal the worker. They must demonstrate the original
untraced child continuing: latched/no input exits74 without ACK; sent/input
available emits exactly ACK16, closes and exits0. These are explicit negative
counterexamples, not authenticated handoff, automatic death or production
completion. An unexpected status, record or cleanup signal cannot pass.

Positive losses signal only the original creator pidfd. Exact creator SIGKILL/
reap and adopted original worker SIGKILL/WNOWAIT/reap/ECHILD, unrelated original
child survival and original root cleanup retain the old ordering. Healthy paths
finish original ACK/close/zero exit with one attempt/dispatch/send. No restart,
permission recovery or caller-provided PID/path/FD/payload API is introduced.

## Terminal socket behavior

A preliminary nonblocking Linux seqpacket check observes that closing a peer
with an unread dummy produces one ECONNRESET, then actual zero reads with HUP;
without unread input it produces only zero/HUP. The saved check is transport
feasibility, not process-loss evidence. A reset is not EOF.

Keep the existing drain function and its 48-byte/five-read guard unchanged.
Only the new sent-loss cut must first consume exactly one ECONNRESET; only the
new sent no-EXITKILL control may consume exact ACK16. After any prefix, require
zero/HUP with no queued extra record and the unchanged bounded EOF drain before
root cleanup. Other errors or resets in any other disposition refuse. Prefix
reads are fixed at one, plus one zero/HUP peek and the old five-read drain;
there is no unbounded loop, generic reset tolerance or timeout extension.
Unknown terminal/EOF evidence preserves the original root.

## Verification and exclusions

Retain a complete 63-test baseline and an isolated new-mode red against unchanged
code. The red-only test becomes a helper after old25 and old14 cases in their
existing process test: do not add a competing fixture, serial flag or wider
outer guard. Add state/dispatch and exact terminal-prefix units, frame-bit and
closed-selector coverage, six process controls, extra-argument/environment
refusal and explicit six-mode MSRV extra-libm refusal. Repeat current targeted
groups, old121 regressions separately, current Linux/Windows strict lint,
current/MSRV checks and portable tests, static/document/registry/closed-admission
checks, historical source comparison and unsigned packet verification.

No raw worker, child protocol, policy/installer, four-role graph, 8MiB image,
65536-byte proc/128-line maps, startup256 pairs, protocol16 events, five-second
wait/outer guard or frozen qualification inputs change. Preserve the existing
parallel-fixture timeout and MSRV lint failures; do not fix unrelated macros
or label their gate green. Pre-arm/running/thread-only loss, fresh creator CLI
exec, in-syscall interruption, restart/recovery of permission, hostile same-UID
custody, trusted credentials/runtime/issuer/job custody, original worker start,
P2/P3 writer cessation, later cells, server/qualification and performance claims
remain outside this local experiment.
