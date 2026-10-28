# Native D-Bus transport: paired compatibility and private faults

The isolated candidate removes the remaining zbus 5 test-backend requirement
and executes the native zbus 6 registered Unix backend. GNU Rust 1.94 and MSRV
1.88 both pass 65 selected cases, including the real private dbus-daemon matrix,
and strict library/test-cfg/all-feature Clippy. Root source/lock, deny policy,
qualification identity, live observation cadence and 300/60/10 windows remain
unchanged. This is compatibility/security preparation, not product adoption or
an improvement claim.

The [plan](dbus-native-transport-plan-20261028.md) and
[producer](../../../../tools/dbus-native-transport-074/README.md) define the scope.
The [held packet](local-runs/dbus-native-transport-20261028/index.json), SHA-256
`5b52a78c032b622f401079ce2b0d895e47ba93504c4cffcf71bcdf024c87f701`,
retains 472 logical members / 165 unique objects. Actual images are identified
by source/path/size/hash receipts; executable bytes are excluded.

## Graph and first results

| Context | Observed result | Disposition |
| --- | --- | --- |
| First offline lock resolution | Cargo succeeds; strict audit refuses Tokio→tracing removal | First failed native result retained; Cargo not repeated |
| Separate offline source/effect audit | 156→135 identities, 21 removed; no new packages or retained checksum changes | Exact old root→zbus 5 and source-proven optional Tokio→tracing effects accepted |
| c compiler envelope | Exit 125 at original physical 4 GiB floor; retired=true | No compiler/native verdict invented; consumed context retained |
| r repaired resource context | GNU/MSRV library check, strict Clippy, complete images, 64/64 each | Only vendor paths changed; 1,529 filtered per compiler |
| r first private-bus matrix | Exit 101: 0 passed, 1 failed, 1,592 filtered | Bare-receiver expects old FD flag 0; native receiver returns CLOEXEC=1; later cases not reached |
| f inert projection | Exact cohort driver render refuses before execution | Frozen preparation and first driver source retained; no Cargo/test dispatch |
| f2 test-only fault candidate | Both complete images, 65/65 each, strict test-cfg/all-feature Clippy | 1,593 complete identities; 1,528 explicitly filtered per compiler |

The first lock review loader also refuses a Python module-name collision;
its raw first source/refusal is retained. The corrected unique loader reviews
the same first lock using exact primary manifests. No original failure becomes
a passing receipt. Old archives in provenance are not selected dependencies.

## What changed and what was exercised

The manual fixture delegates native reads/writes and kernel peer credentials
to zbus 6's actual registered Unix backend, selected through one explicitly
nondefault fixture feature. Default production features and receiver/authority/
kernel predicates stay exact. FD identity, body/serial/destination, bounded partial
writes, one-time ancillary delivery, reply authentication and no-replay assertions
remain. The DTO control preserves optional fields and FD alias/close behavior
within the single selected backend.

The old bare-receiver characterization cannot generate a missing-CLOEXEC fault
on this backend. f2 therefore first asserts the native flag, explicitly clears
only that owned received FD, executes the original negative writer-shape/flag
checks, then restores the flag before returning or resuming a panic. The guard
itself is unchanged. Positive scenarios use the actual native receiver; a tenth
atomic-adapter scenario retains the former adapter's coverage. Both compilers
record exactly this sequence:

1. bare-receiver: injected flag loss refused and restored;
2. healthy;
3. atomic-adapter;
4. fragmented (bounded partial writes);
5. lost-ack;
6. foreign-sender;
7. wrong-serial;
8. disconnect;
9. manager-error;
10. no-capability (no send).

The original nine scenarios remain in their original relative order. The matrix
keeps its 60-second total, three-second infrastructure/two-second method bounds;
the CI owner keeps 1200/1100 origins, 4 GiB floors and five-second polling/retirement.
Every actual final phase retires; source/vendor/boot and outer postflight pass.
The f2 source differs from r only in cfg-test fault generation and two vendor
paths; r's normal-library check applies to identical production source. Fresh
f2 image builds are not described as fresh library-check runs.

Seventeen inert producer controls passed before execution. Three additional
retention controls verify every byte and reject missing fault markers or wrong/
duplicate identities even when a success total is retained. No system bus,
existing service, original owner, server run or product workload is involved.

## Remaining gates

The subsequent [revision-2 consumer gate](dbus-native-source-2-result-20261028.md)
closes reconstruction/formatting for this candidate with paired 65/65 and strict
Clippy. The following paragraph records the boundary at this transport packet;
root/policy/full product and original execution gates remain open.

A separate read-only rustfmt check reports formatting differences in the
generated transport/credential test files. That is an open formatting gate;
the consumed f2 source is not edited in place. Canonical source-package revision
2 reconstruction, formatted consumer proof, final source/effect/policy review,
root adoption and complete package/workspace gates still follow. No registry
publication, signed receipt or production enrollment follows from these passes.

Real original-owner delivery, autonomous cessation of all writers, durable
completion/cells 2–4, full SIGKILL/lost-ACK and same-attempt reconnect remain
separate open execution gates. The dedicated server's systemd 255 limitation is
unchanged; these private-bus tests do not establish ExtraFileDescriptors support.
Chitchat preview remains declined, MSRV stays 1.88 and no deny exception is added.
