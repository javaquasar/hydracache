# Consuming join comparison on the isolated Linux host

Compare the unchanged consuming-join test images from source
f07dcca31e2e69796644d70fef695404b22d1519 with the retained WSL refusals. The
operator authorized this nonproduct host control. It does not authorize an
installation, supervisor operation, enrollment, product workload or qualification.
This plan inherits docs/RULES.md, especially R-3, R-7 and R-8.

Use strict known-host SSH as hydracache-admin, UID/GID 1000. Create one owned
0700 directory under /tmp/hc074-consuming-server-20261020.XXXXXX. Transfer only
the reviewed runner and four already built, SHA256-bound test images. No rebuild,
clock change, CPU affinity, sudo test execution or privileged fixture is included.
Read-only sudo checks inspect only existing marker absence and installed hashes.

Before transfer and around every selection, require the original boot, active
supervisor PID 8839 with zero restarts, installed binary/receipt hashes and absent
campaign/diagnostic/lifecycle contexts. These sequential checks are not an
exclusive host reservation. Refuse on a changed preflight; never stop another run.

Run GNU 1.94 and MSRV 1.88 serially, two rounds each. Each round selects exactly
17 consuming_held tests, 26 previous held-runtime tests excluding the new bridge,
five previous fixed-placement models and 38 held-build integration tests. Use
one test thread, unrestricted CPUs, nice 10 and a 900-second external safety stop
per selection. Keep every result including failures; no retry into a pass. These
are the same selections/fixtures and original 300/60/10 origins and live checks,
not an attempt to extend admission deadlines. Observe image dependencies before
execution and image hashes before and after the matrix.

Retain raw logs, per-selection exit codes, selected counts, host identity and
pre/postflight observations in a new repository packet. Preserve all preceding
WSL packets byte-exact. A server pass establishes only this selection in this
environment; it neither identifies the WSL clock actor nor exercises the actual
privileged FixedPlacement holder, original delivery, all-writer cessation or the
joined SIGKILL/lost-ACK matrix. The next gate remains that complete holder and
its four-frame/full-hash attribution, with separate scope for any privilege need.
