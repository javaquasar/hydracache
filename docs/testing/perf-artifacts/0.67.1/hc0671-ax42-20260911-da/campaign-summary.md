# HydraCache 0.67.1 reference campaign

- Campaign: `hc0671-ax42-20260911-da`
- Source: `7bd31af9a5092466d7a7284995f388d33ed3110f`
- State: `closed`
- Runner fingerprint: `25baeec1c9ec75da90b5dd0f29c101897450b96651d7e47570e18684be486ebb`
- Host archive SHA-256: `d862b5f9e9bcf3f9243efd6a85690b071b763a74bbc733d24103521c4c5d22e9`
- IRQ burn-in SHA-256: `34dba473aa5c49f74e74c44dfe52b0ae499fc26fbc35d070d77de2978e52fea0`
- Sample-set SHA-256: `unavailable`

| Stage | Status | GitHub run | Receipt SHA-256 |
|---|---|---:|---|
| qualification | not-run |  |  |
| full-dress-1 | not-run |  |  |
| full-dress-2 | not-run |  |  |
| bootstrap-1 | not-run |  |  |
| bootstrap-2 | not-run |  |  |
| bootstrap-3 | not-run |  |  |
| bootstrap-4 | not-run |  |  |
| bootstrap-5 | not-run |  |  |
| frozen-candidate | completed | 34580969962 | 0e673a0660e68330e66bebfb664542eb35f2f2e4c34b83ca23b5fd668c326de4 |

Original GitHub artifact ZIP files are retained under `runs/*/original-artifacts/`;
their byte sizes and SHA-256 digests are recorded beside each run.
The exact W5 inputs are materialized without changing those ZIPs under
`reference-inputs/sample-{1..5}` and sealed by `reference-inputs/reference-inputs.json`.

Bootstrap campaigns remain non-ship inputs; only the separate post-activation
frozen-candidate campaign may produce ship-eligible evidence.
