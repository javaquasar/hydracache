# D1 predecessor artifact intake

## Verified input

On 25 November 2026, the published
[0.74.0 release](https://github.com/javaquasar/hydracache/releases/tag/v0.74.0) supplied
`I74-stable-signed.zip`. Downloaded files were checked against the release API's SHA-256
digests. The archive was restricted to its three expected regular files before use.

| Identity | Value |
| --- | --- |
| Product source | `bcba511f0c1f4b3e965c9e17f81b248deb0ea331` |
| Product tree | `93d3f20b1c358af093ad068989185ded83896894` |
| Cargo.lock SHA-256 | `7cd4d3f6688c33b9e680d0af7390d8909ef0e0c4c053f66584c1f9b3128427c3` |
| ELF SHA-256 | `60a37932c554e58a54303f1196c347948fbe0dde162d5a3e32cef5e00f0dd10a` |
| ELF size | `22425336` bytes |
| Protected build | `45092245564`, attempt `1` |
| Signing workflow revision | `ac5c3d9ce45b9466817547bf9ff6c6693142bdfe` |
| Compiler/profile/target | Rust `1.94.0`, `release`, `x86_64-unknown-linux-gnu` |

The release's unmodified `verify-release.py` passed. It pins ELF, receipt and Sigstore-bundle
bytes, requires the exact protected GitHub-hosted workflow, denies self-hosted signing runners,
and compares the authenticated predicate with the full expected receipt. Product source and
signing workflow are separate identities and were verified as such.

The release's unmodified `smoke-stable.py` then passed on Linux as an ordinary user. It used
its own temporary storage/TLS material, a loopback port and its own process group. The observed
checks were client-certificate refusal, AUTH refusal, RESP3 binary SET/GET and absence, fresh
connection authentication, and clean SIGINT shutdown with exit code zero.

The maintenance evidence archive SHA-256
`6d54ff1abae6e3dc9484ecb519e4a80880b303a9190c9cfd9e3c19fce8a85b30` was also checked.
Its 17 members were extracted into the original run-name directory. The unmodified published
Linux verifier passed the complete 108,000,000-byte timing stream: 21,600 measured seconds,
5,400,000 verified SET/GET pairs and 64 reconciled values. Recorded controller reattachment,
process generations, calendar and raw-versus-reported measurements were checked by that reader.
The copied attestation bundle matches the signature-verified stable build's digest.

## Retention

Local files are under `target/imap-075/d1-b74-20261125/`:

- `release-inventory.json` records asset names, byte counts, API digests and download URLs;
- `assets/` retains the signed archive and published verification/smoke scripts;
- `server/` retains the exact ELF, build receipt and Sigstore bundle;
- `signed-binary-verification.json` and `smoke-verification.json` record the passing checks.
- `six-hour-verification.json` records the complete offline maintenance replay.

The smoke executable copy is under the owned Linux preparation directory
`/home/artur/hydracache-075-dbus.F2FOn6/b74-server/`. No existing service was changed.

## Remaining D1 gate

This intake authenticates the network-control binary and its bounded functional smoke.
Embedded and direct-native B74 controls still need builds from the pinned source with their
frozen build/configuration identities. Upgrade/rollback compatibility evidence still needs its
own retained replay. The authenticated binary, smoke and maintenance replay are distinct inputs.

D1 remains open until the production local-backend extraction and asynchronous dispatcher pass
unchanged protocol/conformance suites and matched B74/I75 measurements for local, direct-native
and RESP GET/SET. The dynamic and static dispatch candidates retain the plan's non-regression
bounds; this intake contains no throughput or latency comparison result.
