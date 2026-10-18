param([Parameter(Mandatory)][ValidateSet('194','188')][string]$Lane,[string]$Round='lint-final')
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$out=Join-Path $repo 'target/negotiated-dispatch-20261018'
$suffix=if ($Lane -ceq '194') {'b26b156d3dcd0098'} else {'d2aba7f58d773234'}
$image="/home/artur/.cache/hydracache-074-$Lane-check/release/deps/hydracache_long_run_supervisor_074-$suffix"
$test='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::validation_frame::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::barred_custody::barred_collector::enrollment_frame::dispatch_exchange::tests::negotiated_tests::dispatch_negotiated_private_bus_fault_matrix'
foreach ($name in @("$Round-units-$Lane","matrix-$Lane-$Round","image-$Lane-$Round-before","image-$Lane-$Round-after")) {
    if (Test-Path -LiteralPath (Join-Path $out "$name.log")) {throw "retain existing receipt $name"}
}
$before=& wsl -d Ubuntu -- sha256sum $image
if ($LASTEXITCODE -ne 0) {throw 'pre-execution image hash'}
$before | Tee-Object -FilePath (Join-Path $out "image-$Lane-$Round-before.log")
& wsl -d Ubuntu -- $image negotiated_atomic_receiver_ --nocapture --test-threads=1 2>&1 | Tee-Object -FilePath (Join-Path $out "$Round-units-$Lane.log")
if ($LASTEXITCODE -ne 0) {throw 'atomic receive units stopped'}
& wsl -d Ubuntu -- $image $test --ignored --exact --nocapture --test-threads=1 2>&1 | Tee-Object -FilePath (Join-Path $out "matrix-$Lane-$Round.log")
if ($LASTEXITCODE -ne 0) {throw 'private bus matrix stopped'}
$after=& wsl -d Ubuntu -- sha256sum $image
if ($LASTEXITCODE -ne 0 -or $before -cne $after) {throw 'post-execution image drift'}
$after | Tee-Object -FilePath (Join-Path $out "image-$Lane-$Round-after.log")
