$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$rows=Get-Content (Join-Path $root 'target/manager-channel-attribution-build.log') | Where-Object {$_.StartsWith('{')} | ForEach-Object {$_ | ConvertFrom-Json}
$artifacts=@($rows | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
if ($artifacts.Count -ne 1) {throw 'attribution image ambiguous'}
$image=[string]$artifacts[0].executable
if ($image -cnotmatch '^/home/artur/\.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-[a-f0-9]{16}$') {throw 'attribution image path'}
& wsl -d Ubuntu -- sha256sum -- $image
if ($LASTEXITCODE -ne 0) {throw 'image hash'}
$test='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::validation_frame::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::barred_custody::barred_collector::enrollment_frame::manager_channel::tests::manager_channel_root_fixed_context'
& wsl -d Ubuntu -u root -- env HC074_LOCAL_MANAGER_READONLY=1 HC074_PRIVATE_MANAGER_REPLY_TRACE=1 $image $test --ignored --exact --nocapture --test-threads=1
$status=$LASTEXITCODE
if ($status -ne 101) {throw "attribution changed expected refusal: $status"}
"HC074-MANAGER-ATTRIBUTION expected_exit=$status replacement_gate=false start=false"
