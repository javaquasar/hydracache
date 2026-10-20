# Privileged native Linux holder refusal

The authorized isolated control did not close the native holder gate. GNU 1.94
and MSRV 1.88 each refuse the first healthy case at the original ten-second
window while binding the signed placement. Neither reaches FixedPlacement or
the consuming held-role join. The remaining four cases are not executed; no
failed invocation is retried.

| Compiler | Matrix exit | Original refusal | Observed age |
| --- | --- | --- | --- |
| GNU 1.94 | 101 | diagnostic_collector_causal.rs line 109 | 10,120,568,145 ns |
| MSRV 1.88 | 101 | diagnostic_original_custody_join.rs line 121 | 10,014,035,477 ns |

These are admission-clock observations from one invocation per compiler, not
benchmark estimates. Both kernel refusal reports are null. This cohort records
expiry rather than the prior traced WSL clock-order refusal; it does not prove
clock stability over the entire run or identify a sole performance cause. The
first observed custody ages are already 9,126,504,822 and 9,165,257,066 ns. That
locates the next attribution before and through placement, not in a held-role
hashing operation which was never reached.

## Inputs and protected execution

The [preregistered plan](../../held-consuming-privileged-server-plan-20261020.md)
and runner were committed at 09d1c582; protected staging at 258a43be. Both exact
release test images come from 1eac3bb3. Their hashes and all nine historical
material-bundle files are pinned in inputs.sha256. No source or image was rebuilt
for the server. The bundle worker was not executed; held role images are synthetic.

Upload is /tmp/hc074-held-upload-20261020.PShZT2, owned 1000:1000 with mode 0700.
Execution is /tmp/hc074-held-root-20261020.Scmc35, owned 0:0 mode 0700, with test
images and reviewed runner 0500 and bundle files 0400. Root copies and checks
hashes before execution, uses a clean environment and a one-shot latch. Each
matrix runs once with the full exact selector, no case/cost override and one
test thread. Timeout remains 1200 seconds; original 300/60/10 bounds are intact.

The first Windows SCP attempt could not resolve its UNC input; no test started.
A local binary copy resolved transport without changing hashes. The staging
SSH shell later returned 127 after printing WRAPPER_PID because an extra trailing
CRLF was interpreted as a command. The protected wrapper had already started
once. Inspection of that same directory found both matrix exits 101, wrapper
exit 1 and postflight exit 0; the staging command was not replayed. These are
separate transport and test outcomes, retained in collection.json and logs.

The raw remote directory contains 17 logs/exit files plus remote-files.sha256.
Their hashes match the root export transcript. Other captured SSH/PowerShell
logs are text transcripts, not byte-exact remote streams. Staging and export
recipes are retained; this packet contains no private credentials or binaries.

## Safety and remaining gate

All nine campaign/lifecycle markers are absent around both selections and in
independent postflight. Supervisor remains active/running, PID 8839, NRestarts 0;
boot, installed executable and provisioning receipt remain unchanged. Host mount
tables before/after are byte-identical. No hc074-owned-root temporary directories
or cgroups remain after fixture cleanup. The protected input/log directory and
admin transfer/export directories are retained, not broadly removed. These are
point-in-time observations, not a continuous exclusive host reservation.

No release was installed, service stopped/restarted, clock changed, original
manager message sent, production trust enrolled or product workload run. No
completion marker was reached, so terminal custodian/guardian or all-writer
cessation is not claimed for this cohort. The old image's server=false field
is fixture vocabulary; the outer receipt correctly records server=true.

Next attribute preparation, enrollment and signed-placement validation using
observation-only local instrumentation. Preserve every live check, original
origin and deadline. Do not blame four-frame role hashing or extend admission
to make this candidate pass. Joined SIGKILL/lost-ACK remains gated on a healthy
complete composition; production delivery, lifecycle, reconnect and trust remain
independent gates. Earlier sealed WSL failures remain unchanged and unaccepted.

Run verify.ps1 to validate raw transfers, source/qualification identity and the
derived negative receipt, including in-memory missing/wrong/extra metadata
mutations. Evidence integrity passing does not mean the native matrix passed.

The recipe files are historical evidence, not authorization to replay the root
control. The all-workspace Windows format command fails with OS error 206
(command-line length); targeted formatting of xtask and the supervisor passes.
No Rust source, lockfile, compatibility entry, registry or qualification
manifest changes are part of this control.

Local checks pass: 321 performance-contract tests, 20 historical-source guards,
strict xtask lint, targeted formatting, doc-check, all 186 registered gates and
the local non-promotable performance-contract check. The earlier sealed WSL
packet re-verifies unchanged. The new verifier rejects 47 in-memory summary
metadata mutations. UnsealedReview deliberately cannot report verified evidence
and refuses to bypass an existing inventory; normal verification requires the
exact complete file inventory as well as the raw-transfer and outcome checks.
