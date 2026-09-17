# AX42 0.71 D4 memory evidence index

The D4 chain used the same immutable source and workflow commit,
`da8d6de409a657e0260e7fbfb4ab31d8d6ad5ca8`, on the admitted AX42 host. The M3, M8, M9,
and M10 campaign receipts all report `success` and `ship_evidence_eligible=true`; M10's B1 and C
cells each ran for 24 hours. This index is small by design. Do **not** merge the evidence branch
into `main`.

- Evidence branch: `evidence/0.71/ax42/d4`
- Accepted evidence commit: `123bbd135e1db899ea218875aec8c5e6869531e9`
- Branch manifest: [`manifest.json`](https://github.com/javaquasar/hydracache/blob/123bbd135e1db899ea218875aec8c5e6869531e9/manifest.json)
- Redaction receipt: [`redaction-receipt.json`](https://github.com/javaquasar/hydracache/blob/123bbd135e1db899ea218875aec8c5e6869531e9/redaction-receipt.json)
- Verification: fresh Git LFS checkout followed by `python verify.py` passed for all five archives,
  their member contents, identity receipts, and hygiene checks.

| Stage | Immutable campaign ID | GitHub run | Artifact ID | Sanitized archive bytes | SHA-256 |
| --- | --- | ---: | ---: | ---: | --- |
| M3 | `hc071-d4-ax42-20260914-m3-01` | `34841416735` | `10349451463` | 268,995 | `bee06218e78c12e71febbc55189dcc9d776701f08f559cac13be5e4cee0cacf3` |
| M8 | `hc071-d4-ax42-20260914-m8-01` | `34909454654` | `10375653483` | 285,310 | `799f8542a0d96d0161f22d199cd7bc20452dfa4ff50e9348b214851a80de48fd` |
| M9 | `hc071-d4-ax42-20260915-m9-01` | `34915002362` | `10398284742` | 250,021 | `9593142e3f3a6b0126d3f813c8a4c89d2c1df8f141f0fa7b2bbd6d58daa15368` |
| M10 | `hc071-d4-ax42-20260915-m10-01` | `34975284094` | `10500899819` | 298,164 | `75c8d72abdd6088c85da4d5f08887f686507a4e9949231edff251bc336ae944b` |

The sanitized frozen-host-state archive is 222,845 bytes with SHA-256
`ca09cf09694153e08a803b50b6f43368b198bea07cd4d9729998cb9de124df9b`.
The original campaign, protected-mirror, and host-state archives remain unchanged in an external
copy at `Documents/HydraCache-Evidence/0.71/AX42/D4/raw` on the archive workstation, outside the
repository and outside AX42. All ten raw SHA-256 values are in the redaction receipt; every raw
copy matched the server or staging hash and passed tar readability checks. The 49 extracted
GitHub artifact files were also copied outside the repository and checked file-by-file.

The public archives omit test-generated private keys, compiled binaries, and a few
identity-bearing host files. They are not byte-for-byte substitutes for the raw copies. Claims
remain limited to the exact host, source/workflow SHA, and measured D4 memory scenarios; they do
not establish a general Redis/Hazelcast advantage or cross-host result. Release claims must be
generated only after D4 dispositions and ship gates accept this chain.
