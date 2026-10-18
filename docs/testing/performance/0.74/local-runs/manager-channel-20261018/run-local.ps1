$ErrorActionPreference='Stop'
$images=Get-Content (Join-Path $PSScriptRoot 'images.json') -Raw | ConvertFrom-Json
$prefix='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::validation_frame::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::barred_custody::barred_collector::enrollment_frame::manager_channel::tests::'
foreach ($entry in $images) {
    $image=[string]$entry.image
    if ($image -cnotmatch '^/home/artur/\.cache/hydracache-074-(194|188)-check/release/deps/hydracache_long_run_supervisor_074-[a-f0-9]{16}$') {throw 'unsafe image'}
    $receipt=(& wsl -d Ubuntu -- sha256sum -- $image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $receipt -cne "$($entry.sha256)  $image") {throw 'initial image differs'}
    $receipt
    "HC074-MANAGER-COHORT compiler=$($entry.compiler) begin=true"
    & wsl -d Ubuntu -- $image manager_channel:: --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) {throw 'ordinary units failed'}
    & wsl -d Ubuntu -- env HC074_LOCAL_MANAGER_READONLY=1 $image ($prefix+'manager_channel_nonroot_fixed_context_refusal') --ignored --exact --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) {throw 'nonroot refusal failed'}
    # Only the exact read-only test runs as local WSL-root, never all libtests.
    & wsl -d Ubuntu -u root -- env HC074_LOCAL_MANAGER_READONLY=1 $image ($prefix+'manager_channel_root_fixed_context') --ignored --exact --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) {throw 'root read-only context failed'}
    & wsl -d Ubuntu -- $image dispatch_exchange:: --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) {throw 'exchange regression'}
    & wsl -d Ubuntu -- $image descriptor_plan:: --skip dispatch_exchange:: --skip manager_channel:: --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) {throw 'descriptor regression'}
    & wsl -d Ubuntu -- $image diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence:: --skip original_fence_custody_matrix --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) {throw 'storage regression'}
    $receipt=(& wsl -d Ubuntu -- sha256sum -- $image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $receipt -cne "$($entry.sha256)  $image") {throw 'final image differs'}
    $receipt
    "HC074-MANAGER-COHORT compiler=$($entry.compiler) completed=true admission=false"
}
'HC074-MANAGER-LOCAL completed=true server_ssh=false service_mutation=false original_join=false start=false'
