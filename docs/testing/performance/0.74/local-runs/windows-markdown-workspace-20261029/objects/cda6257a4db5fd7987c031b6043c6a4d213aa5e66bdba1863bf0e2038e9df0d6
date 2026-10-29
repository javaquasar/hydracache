# Server preparation profile, one non-admitting attempt

Uses the exact already built GNU 1.94 release libtest, SHA256
`d25675c71977c4dc48d4cbdf2973081f5d37189c4339e7de5465f571e2ad94c7`.
Checks all 1,891 frozen build inputs, protected roots and its original build
receipt. No rebuild, dependency update or unchanged original-manager retry.

The exact ignored `privileged_consuming_held_live_matrix` test receives only
`healthy`, with the existing cost observer. An empty environment excludes both
original-service and optional materialization modes. This fixture stops after
consuming join/drop/cleanup; it cannot send original service-start, release a
worker or publish completion. Its private mount namespace binds a uniquely owned
cgroup subtree as system.slice; host system.slice and existing services remain
unchanged. It uses explicitly synthetic issuers/roles and a private fixture fence,
never the supervisor's common host lock. No enrollment, account, /opt or /etc install.

This is a different fixture composition from the first local original-manager
failure. It attributes shared preparation, not real manager delivery. systemd
255 is not promoted to an admitted extra-FD delivery environment. The paid host
is not exclusive; supervisor activity and observer overhead limit numeric claims.
Preserve all live checks, their count/order, original 300/60/10 windows and first
refusals. No retry-to-pass, workload or product qualification.

`run.py prepare` creates an exclusive local candidate directory. Run offline
controls, review source/effects and commit the caller before `run.py dispatch`.
Dispatch performs a fresh read-only host/campaign preflight, transfers a hashed
archive into a root-only /tmp directory and starts one detached autonomous CI
owner. The original tested `_owned` function is extracted without rewriting;
its 1,200-second envelope, 4 GiB floor, five-second capacity polling and
five-second retirement are preserved. The Linux subreaper signals retained
descendant pidfds and requires ECHILD. This is CI containment, not evidence of
production all-writer retirement. SSH loss never dispatches a replacement.

The owner writes started/preflight/execution/postflight/result with exclusive,
fsynced creates; stdout/stderr exist before preflight. Inspect/retrieve the same
root after disconnection, never repeat stage or dispatch. A failure retains its
original result; no directory removal occurs automatically. Postflight must
prove unchanged supervisor PID/invocation/restart count, boot, protected receipt/
image, mount tree, namespace and clock-service states, with no surviving fixture
cgroups. A failed postflight is not successful cleanup. No host settings, clocks,
AppArmor or existing services may be changed to make this profile pass.
