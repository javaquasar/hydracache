# HydraCache 0.73 interrupted confirmation archive

This directory preserves the complete-or-incomplete GitHub Actions artifact and job log for run
[`36622527013`](https://github.com/javaquasar/hydracache/actions/runs/36622527013). The run used the
frozen baseline `e757556d3a31d565f52a9561d6d4e555bb1cc373`, candidate
`16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b`, and tooling
`b9f621a5b4f50c5277600fff9a8278fbd2154e28`.

The I73 role completed all 1,036,800,000 operations and produced its final checkpoint. The C73
role was still healthy after 352 checkpoints and 252,728,137 completed operations when GitHub
Actions delivered an external cancellation at `2026-10-01T01:58:42Z`. The candidate process had
reported no errors, timeouts, rejections or major faults, and the immediate post-C73 calibration
passed on the same host and lease. The artifact therefore diagnoses an orchestration interruption,
not a product regression, but it is incomplete and cannot admit C73, satisfy confirmation or
support a comparative numerical claim.

`github-artifact.zip` is the byte-exact Actions artifact. Its SHA-256 matches GitHub artifact ID
`11136098992`. `job-log.txt` is the corresponding job log and contains the cancellation marker,
successful post-cancellation calibration and successful artifact upload. Infrastructure paths and
the pseudonymous host fingerprint are retained because they participate in the attempt identity;
the archive contains no payloads or credentials.

Verify the preserved files with:

```powershell
./verify.ps1
```

No automatic retry was performed. A future confirmation must retain the frozen product,
workload, duration, estimator, seed and thresholds and requires explicit authorization.
