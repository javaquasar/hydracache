# HydraCache 0.73 six-hour qualification archive

This directory preserves the original GitHub Actions artifact for run
[`36532416869`](https://github.com/javaquasar/hydracache/actions/runs/36532416869). The workflow
qualified final registry candidate `16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b` against frozen
baseline `e757556d3a31d565f52a9561d6d4e555bb1cc373` using tooling
`6135b9d4470234eebf00ecc07200da285e2aaa20`.

`github-artifact.zip` is the byte-exact downloaded artifact. Its SHA-256 matches both GitHub's
artifact digest and the independently downloaded value recorded in the qualification receipt. The
archive contains minute checkpoint series, compact receipts, calibration records and process logs;
it contains no payloads or credentials. Infrastructure paths and the pseudonymous host fingerprint
are retained because they participate in the attempt identity.

Verify the archive with:

```powershell
./verify.ps1
```

The successful qualification opens a separately authorized 24-hour-per-role confirmation. It is
not itself confirmation and does not authorize final release or portable numerical claims.
