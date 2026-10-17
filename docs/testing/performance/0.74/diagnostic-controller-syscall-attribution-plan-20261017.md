# Original controller syscall attribution

Pin source b05bb9a2065627f560958517320dfca126d1b35f and reuse the two
existing normal GNU release component-control binaries from the component
packet. Do not rebuild or change a validator, fixture, observation, deadline,
registry, kernel setting or permission. This is a diagnostic of the existing
test family before reservation, not a product candidate.

Run each binary once without tracing and once under nonroot `strace -f -c -w`.
Use order 194 plain, 194 traced, 188 traced, 188 plain. Each invocation already
contains three fresh samples and ten separate controls per sample. Require all
thirty rows, original unchanged/no-refusal control markers and successful exit.
Retain failures without a protocol retry or refreshed timestamp. Trace only the
exact test process and descendants, not the supervisor. Use summary output only:
no syscall arguments, paths, payloads or credentials. Bound each invocation to
120 seconds and isolate its TMPDIR in a unique owned server directory.

Retain raw summary counts, errors and strace wall-clock syscall time by name,
plus every existing numeric component row and binary hash. Compare counts across
compilers and traced spans to contemporary plain spans only as a tracing
perturbation check. The aggregate summary includes fixture setup, full opening
and closing OIDC validation, all ten controls and teardown. It cannot assign a
syscall to one component, establish exclusive CPU cost or predict a speedup.
Overlapping thread syscall wall times are not end-to-end wall time. Failed
syscalls can be expected probes; their presence alone is not a protocol failure.

Use the existing exact boot, supervisor PID/start/restarts, installed binary,
receipt and absent campaign/context guards before and after every invocation.
Independent final postflight checks executable objects, including deleted image
names, and absence of the diagnostic installation. Only nonroot writes in the
new isolated temporary directory are authorized. No installation, supervisor
stop, root mutation, clock change, product workload or expensive Actions.

Retain an inventory-hashed evidence packet and a local verifier that checks raw
rows, summary membership/totals, source and binary provenance, host guards and
the unchanged qualification manifest. Run documentation, formatting and local
non-promotable performance-contract checks. Existing production test results
remain attributed to their earlier source; do not report them as new runs.

Use the result to choose the next narrow owner-level experiment while retaining
all live checks. Grouped preparation still needs separate agreement; original
start, independent guardian, writer cessation, completion and later SIGKILL
boundaries remain closed.
