# Owned inline boot experiment (not adopted)

This standalone, dependency-free test image compares a private owned inline
36-byte UTF-8 value with the frozen String sample. Longer inputs retain an owned
String fallback. It is not linked by any product or supervisor target. No host
capability, transport, real clock receiver or admission path is present.

The [preregistered decision](../../docs/testing/performance/0.74/sample-boot-inline-plan-20261022.md)
rejects adoption when the checked getter median is higher on both installed
Windows compilers. That condition occurred. Allocation savings are real within
this synthetic corpus, not a full-path speedup. Production representation stays
unchanged; the rejected prototype is retained only for reproducibility.

The initial reference-only source is commit `4ec3b04d`. In order, the first
receipts came from `check.ps1 -Phase reference -Lane 194`, `188`, `host`, then
`-Phase candidate` for the same lanes. Reference and paired timing are separate;
each retains all seven first intervals, with no retry/noise filtering. Do not
rerun measurements to replace this packet. The first candidate host canary
refused on an extra newline in its restoration assertion, not a frame change.
Only the host source gate was repaired and rerun under `-Cohort canary-repair`;
both original measurement lanes and the first source refusal are preserved.

Read-only review after packet publication:

```powershell
python -B tools/sample-boot-inline-074/review_inline.py --output docs/testing/performance/0.74/local-runs/sample-boot-inline-20261022/facts.json --verify
python -B -m unittest discover -s tools/sample-boot-inline-074 -p test_review_inline.py -v
```

The reader checks first receipts/raw hashes, fixed corpora, exact passing tests,
timer types/order/work, ownership, layout and allocation rows. It reconstructs
the earlier baseline with its unchanged reader and verifies fifteen preceding
seals/856 members. Packet verification also requires the retained reference
snapshot. A passing pure model does not satisfy material, real faults,
SIGKILL/lost-ACK, all-writer retirement, reconnect or server gates. Original
checks, windows and first refusals are untouched. No live/WSL/server workload is
included.
