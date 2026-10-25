# Original wrapper output binding

The local observer can now bind a reserved attempt to the unchanged wrapper's
original output objects and pass those held bytes to the independent target
reader. The first two-case Windows fixture session passes: normal output sealing
succeeds, while an outstanding write handle refuses sealing and cannot be retried
after that handle closes. Both commands are Python fixtures, not the Cargo target.

Implementation commit: `8a7dd6472da31adc60adaa1397b38966465fde02`.

## Reservation and live attachment

`tools/xtask-package-attribution-074/attempt_output_binding.py` checks that the
selected wrapper output slot is absent before spawn and holds the reservation
and wrapper-parent directories. The attempt label selects the single output directory; neither a
result document nor a transcript can select another path. The caller reserves an
independent `ExpectedSource` before launch rather than adopting the response's
HEAD/source identity.

The exclusive consumed marker belongs to `DispatchOnce`; the unchanged wrapper
still creates its own directory exclusively. The absence check is not a global
filesystem reservation against another controller using a different directory.

After original process/lineage attachment, `AttemptOutputs.attach` requires the
same live owner PID and creation time and holds the wrapper directory plus its
`intent.json`, `stdout.log` and `stderr.log`. Attachment is attempted once.
The Windows handles are non-inheritable, have read access only and omit delete
sharing. Reparse objects, wrong object types and hard-linked files refuse.
The workspace and wrapper remain trusted prerequisites; this is not hostile
filesystem ancestry, signed enrollment or remote producer authentication.

## Terminal sealing and the target verdict

After the existing custody object proves the held processes terminal,
`AttemptOutputs.seal` compares the original owner with its terminal witness and
holds `result.json`. It compares volume/file IDs for all original directories
and files. All four file locks must be acquired without write or delete sharing
before any content is read. An existing ordinary write handle causes refusal;
no replacement file is adopted.

The reader uses the sealed handles, verifies size/write metadata around reads
and records each file's ID, length and SHA-256. Fixed read ceilings are 16 MiB
for each JSON receipt and 64 MiB for each transcript. These ceilings reject an
observer read; they do not truncate output, shorten the wrapper's execution or
kill its owner. A read, identity or cleanup failure latches the first refusal.
There is no sealing retry, including after an offending writer closes.

`bound_target_observation.py` joins the same dispatch reservation, original owner,
custody object and held output reader. It takes no external log paths. Wrapper
exit status must agree with the original result before the unchanged source,
command, 321-case and semantic checks run. A model's complete target verdict,
failed/partial target and lost ACK remain separate from observation success.
Only this bound local join removes the pure reader's outstanding file-binding
requirement; it cannot grant execution, full-package or release admission.

Sealing excludes ordinary write handles to these four objects during reading.
It does not revoke writers, exclude writable mapped sections, establish every
descriptor alias or prove cessation across a production cgroup. Query cleanup
is still not a writer-fence release. The original live predicates, order,
frequency and 300/60/10 origins/windows remain unchanged, as do the 780-second
wrapper and the observer's 30/1200-second discovery/outer bounds.

## First controls

`output_binding_control.py` runs exactly `healthy`, then `writer-held`, stopping
on the first unexpected failure. Each case spawns the original wrapper once,
retains its PID/creation identity and observes the four held processes terminal.
Both wrappers finish zero without source/HEAD drift or early cleanup.

In `healthy`, sealing records all four original files. The result reader then
retains the expected `target command/window/retry drift` refusal: a Python node
command must not become a successful `performance_contract_074` result. In
`writer-held`, a deliberately retained append handle to stdout causes
`hold original output: Win32 error 32`. Closing that handle does not permit a
second join. Neither fixture supplies a selected-target or full-package pass.

The final attribution suite passes 188 Python cases, including 24 added cases
and two actual Windows filesystem-sharing tests. The separate historical-tool
suite passes 91 cases: 279 cases across the final suites, not the sum of repeated
runs. Compileall, tabnanny, the unchanged v3 source/effect review and the local
non-promotable performance contract pass. No product throughput or observer
overhead comparison is claimed.

The [first evidence packet](local-runs/original-output-binding-20261025/index.json)
has 91 members in 66 content-addressed objects. Its index SHA-256 is
`568aba8368782b54bb555173a7c3cee2319b2d4f962bc26588ff696bd343d0c7`.
It retains the first model/native results, final suites, exact source versions,
wrapper receipts and unchanged protected inputs.

The [independent reader packet](local-runs/original-output-binding-reader-20261025/index.json)
retains the first successful staged-byte verification and documentation check:
10 members, 10 objects, index SHA-256
`e9f58c2f3b565dd430b8978d968a80eb747612ff6301801baebece796c582076`.
The staged reader checks member lengths/hashes, original owner/terminal joins,
held-file digests, expected refusals and the unchanged 780-second receipts
without repeating either fixture producer.

## Next admission boundary

The [fixed observation protocol](contract-observation-protocol-20261025.toml)
and launch receipt are unchanged and still disable real-target dispatch. The
composition and file binding now have local controls, but the actual Cargo
delegation shape and output lifecycle remain unobserved. Next is review of the
finite real-target caller and its explicit launch authorization, followed only
then by that protocol's single diagnostic observation. Unexpected shape must
refuse without dropped edges, a fresh origin or a retry.

Durable production custody, original delivery/all-writer retirement, the complete
package gate, dependency-policy decisions and server admission remain open.
No clocks, services, dependencies, qualification inputs or server state change.
