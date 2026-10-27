# Exact package closure test inputs

This module composes six named test inputs with their independently identified
historical producers. It is not a runtime validator, trust source or execution
permit. It performs no IO, Git calls or historical-code execution, and caches
only immutable compiled rule data. Every selected current input is checked again
before inverse edits and the complete predecessor is checked afterwards.

The fixed source is `46648903c0eb7ce88d438008fa6465a96cc3c414`. The rule SHA256 is
`5e591676f92de283fd9e0d2dd7048c5ff3943406cd71a6fc1474090cbf6edff3`.
Six explicit consumer/path pairs use ten ordered inverse edits. CRLF-to-LF is
the original readers' text representation, not arbitrary whitespace folding.
Unknown consumers refuse; other paths pass through unchanged to their original
checks. No additional filesystem/Git observations or fallback are inserted into
those guards.

| Consumer | Paths | Named predecessor |
| --- | ---: | --- |
| cache | 2 | `25faa7bb762aa229631e6f6cfc25cae33e16ff43` |
| remaining | 2 | `49f9b0b22b45010d6b407faf92588e7032ba05f2` |
| census | 1 | `ae508a57220fe95b11887b81f16ff05263f1dee9` |
| leaf | 1 | `a329cac91784d79d9e5b925c22d882cb0d327283` |

The independent compatibility-document producer advances explicitly to
`d3256fc08b8da2df424d6da5bce99c0e168939b0`; its predecessor remains
`3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f`. Original test names and assertion
tokens stay exact. New controls bind every current and prior body to actual Git
objects, reject stale/truncated/appended and seeded-mutated inputs, and check
fresh successful calls after refusals.

The first nine-row candidate is rejected: PID admission already enters after a
verified runtime view, so applying the raw-input row there refuses. Editing the
PID test also trips the unchanged old whole-source guard. Only that candidate's
three rows and PID-test edits are removed; no PID repair is installed here.
Its exact source, compile refusal and failed controls remain in the
[repair evidence](../../docs/testing/performance/0.74/package-closure-repairs-result-20261027.md).
Keep PID's conditional read order and existing guards when designing a separate
composition. A green focused cohort does not complete GNU or full package gates.
