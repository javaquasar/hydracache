# Fixed intent storage ancestry inspection

This local prerequisite adds a read-only original-object guard for a separate
fixed root-owned intent leaf. It does not publish intent, lock the host, inspect
journal records, authenticate authority or enroll a host. Baseline is
`794d164d26f9c0cbe8bd050eb68588f5b3caffae`.

Existing tmpfiles provisioning assigns `/var/lib/hydracache-performance/campaigns`
0750 to `hydracache-perf:hydracache-perf`, and its parent 0750 to
`root:hydracache-perf`. Do not change either to satisfy an intent predicate or
trust worker-writable storage as root authority. The prospective intent path is
`/var/lib/hydracache-performance/diagnostic-intents-074`; this step creates nothing.

The public constructor has no path, ownership, descriptor or environment selector.
Effective UID/GID must be zero before any open. Walk `/`, `var`, `lib`,
`hydracache-performance` and the fixed leaf with no-follow directory descriptors.
Require root UID for every component, root GID for `/var/lib`, and permit the
performance parent to retain its provisioned group only when it has no group or
other write and no special bits. Require root:root and exact 0700 for the leaf.
Root ancestry also has no group/other write or special bits. Cross-device
ancestors are permitted; namespace/mount admission is a separate prerequisite.

Capture device, inode, UID/GID and exact mode of every original directory. Each
later inspection compares original and current named objects, including a newly
opened `/`. Directory timestamps/link counts may change with unrelated entries;
they are not identity authority. Any error latches the original guard; subsequent
calls do no I/O, even if permissions or names are restored. The guard has no
Clone, Send/Sync, file-descriptor getter, fixture conversion or start capability.

Positive local tests exercise the same private walker using owned nonroot
temporary trees and an explicit private test-only ownership policy. Metadata
predicates additionally test modeled root-owned stamps. These positives do not
certify initial-host namespaces, real root storage, issuer enrollment or durable
consumption. Test missing/unsafe/symlink components, all named/descriptor object
replacements, sticky refusal, benign directory growth and API non-promotion.
Preserve baseline and missing-API red, then run repeat tests, lint, MSRV, docs and
local contracts. Frozen plan/manifest inputs and admission remain unchanged.

Next, a separately preregistered publisher must compose this original storage
with the original authority and supervisor-owned fence/state. It may not promote
the fixture journal or use the worker-owned campaign lock as trusted root
authority. Lease-specific bounded retention, original signed worker borrowing,
live peer/clock and Starting/watchdog/cleanup composition remain open.
