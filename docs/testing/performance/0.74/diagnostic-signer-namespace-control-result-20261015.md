# Nested namespace regain control is not reproduced inside the selected chroot

The original preregistration predicted successful nested CLONE_NEWUSER and regained
capabilities under the old 34-instruction filter. The exact owned native probe
instead returns EPERM. The thirteen-case run passes twelve cases, including all
fifteen candidate denials and actual alternate-ABI SIGSYS; only that prediction
fails. A focused run records the exact native error exit. Preserve both red logs.

The old BPF model returns ALLOW for this call. Linux documents EPERM for
CLONE_NEWUSER when the caller's root differs from its mount namespace root in the
[unshare manual](https://man7.org/linux/man-pages/man2/unshare.2.html).
The selected child chroot matches that condition. This explains the observation;
it is not a demonstrated escape and not standalone attribution to the new filter.

Disposition of the regain counterexample: not reproduced in this fixed profile.
Do not change helper bytes/flags, remove the root or substitute unshare(0). Record
the old-policy EPERM with unchanged original namespace/maps/zero capabilities and
other context, then refuse and kill/reap that exact control before continuing its
failed-prediction helper branch. Never publish readiness from it or call it a
successful capability-regain probe. The other three old-policy controls retain
their original zero/ENOTDIR/EBADF expectations and exact zero terminal exits.

The candidate's fifteen EPERM requirements, original install/context guards,
hardening, ABI kills, bounds, thresholds and admission remain unchanged. Use the
independent exact BPF extension plus old chdir/fchdir/setns controls for filter
attribution. Keep capability-regain reproduced=false in the new contract and
packet; dummy handoff, actual tracer loss and production custody stay separate.
