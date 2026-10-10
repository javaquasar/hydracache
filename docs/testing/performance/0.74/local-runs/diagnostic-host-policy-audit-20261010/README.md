# Local worker authority audit evidence

This packet records existing receipt rejection behavior and a proposed authority
choice. It does not enroll a worker or implement trusted host/NSS policy.
The [audit](../../diagnostic-host-policy-audit.md) describes the choice requiring
approval and the local implementation sequence that would follow it.

The Windows baseline passes seven existing host-receipt tests. Adding two
regressions yields nine Windows and ten Linux passes; Linux additionally executes
its existing mountinfo parser test. Each new test rejects eight added authority
fields. These synthetic schema mutations establish refusal, not signatures or
real host/namespace enrollment. The production prefix of host_receipt.rs remains
exactly unchanged from the audit baseline. There is no API-red capture because
this slice adds tests of existing strict decoders, not a new API.

Source is the test commit recorded in manifest.json; documentation and registry
captures are working-tree observations, not clean-source release qualification.
The root suite began before packet registration completed. A separate final
registry check repeats after the packet exists; neither check authenticates the
packet as host evidence. Raw captures retain exact bytes under -text attributes.
Require-ship is an intentional refusal, not a green release gate.

Run `python verify-packet.py` in this directory to verify the exact capture
inventory, hashes, declared test rows, closed proof boundaries and frozen
qualification digest. The verifier performs no writes or host operations and
cannot authenticate arbitrary replacements of both logs and their manifest.
No performance measurement, server inspection/mutation, workload, qualification
or full workspace milestone verification belongs to this packet.
