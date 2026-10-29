# Bind refusal retention and bootstrap selector checks

Do not rerun probe.py or regular_control.py. Their sole first outcomes are retained:
sealed memfd open_tree EINVAL, and an ordinary-file partial SIGKILL/mount observation
whose whole control failed in cleanup. The latter source is unsealed/ineligible.
cleanup.py fixes the ismount assumption only in offline regression tests; no mount
control is repeated to turn either first failure green. No temporary fixture folder
was found in the subsequent read-only prefix audit.

```powershell
python -B tools/held-bind-alias-074/check.py host
python -B tools/held-bind-alias-074/check.py 194
python -B tools/held-bind-alias-074/check.py 188
python -B tools/held-bind-alias-074/check.py metadata
```

The fixed runner reuses the unchanged exclusive component logger, selecting only
bootstrap protected-source tests and three old configuration/key regressions.
It does not call unshare, either mount control, ignored/original fixtures, product,
services or servers. Existing lanes refuse before execution. Root/kernel negative
observations and source hashes remain separate from passing source/bootstrap checks.
An initial Windows offline import failure was fixed by mocking CDLL at test import;
that test correction is not another Linux control. Its captured-tool summary is
documented separately; it is not a retained raw process receipt.

retain.py verifies the two red integration baselines, unchanged failed diagnostic
outcomes/source hashes, exact command/source/log/scope receipts and qualification,
then exclusively seals an inventory. It cannot promote either negative bind result
to bootstrap execution or original delivery. Compile/test elapsed time is metadata,
not a performance measurement.
