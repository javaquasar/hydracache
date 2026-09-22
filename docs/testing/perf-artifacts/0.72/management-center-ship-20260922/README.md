# HydraCache 0.72 Management Center release evidence archive

This branch preserves the accepted release evidence for exact candidate
`24927c28c279c6c34ad90111ee6470b4065e0815`. It is an evidence archive, not a
different product candidate. The `v0.72.0` runtime tag must continue to resolve to the exact
qualified candidate above.

## Accepted campaigns

| Purpose | GitHub run | Result |
| --- | ---: | --- |
| Full exact-SHA campaign | [35537203094](https://github.com/javaquasar/hydracache/actions/runs/35537203094) | 50 jobs completed, 0 failures; six-hour candidate soak and real 0.71 to 0.72 compatibility passed |
| 24-hour ship confirmation | [35556487047](https://github.com/javaquasar/hydracache/actions/runs/35556487047) | ship soak passed without weakening duration, rate, TTL, fault schedule or resource ceilings |
| External baseline capture | [35723493297](https://github.com/javaquasar/hydracache/actions/runs/35723493297) | pre-feature and published `v0.71.0` baseline receipts passed and were bound to the candidate |

The final aggregate in `accepted/release-evidence-0.72.json` contains 15 `ship-ready` work
items and no lower-stage item or rejection reason. `accepted/test-evidence` retains the candidate,
ship and mixed-version measurement records. `accepted/management-center` retains the validated
per-work-item and baseline evidence. `accepted/receipts` and `accepted/canaries` retain the exact
inputs consumed by release admission.

## Original GitHub artifacts

The files under `original-artifacts` are the byte-for-byte ZIP responses downloaded from GitHub
Actions. They are named for readability; `artifact-manifest.tsv` binds each file to its immutable
artifact ID, run and original name. `SHA256SUMS` covers every retained file in this archive except
itself.

The 424 MB `daemon-process-066` artifact is deliberately not duplicated in Git because it contains
compiled binaries and broad diagnostic logs rather than an additional 0.72 measurement. The small
`daemon-process-evidence` artifact, the exact receipt and the immutable Actions artifact retain its
admission result. This archive contains no credentials and does not replace the original Actions
retention policy.

## Verification

From this directory:

```powershell
Get-Content SHA256SUMS | ForEach-Object {
    $hash, $path = $_ -split '  ', 2
    if ((Get-FileHash -Algorithm SHA256 $path).Hash.ToLowerInvariant() -ne $hash) {
        throw "checksum mismatch: $path"
    }
}
```

To repeat admission, copy the `accepted` evidence into the equivalent `target` paths of a clean
checkout at the candidate SHA, then run:

```text
cargo xtask management-center-check --release 0.72 --require-evidence
cargo xtask release-evidence --release 0.72 --receipts-dir target/release-evidence/receipts --require-ship
```

