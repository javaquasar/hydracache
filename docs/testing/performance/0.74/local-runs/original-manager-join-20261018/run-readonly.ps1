$ErrorActionPreference='Stop'
$images=Get-Content (Join-Path $PSScriptRoot 'Candidate-images.json') -Raw | ConvertFrom-Json
foreach ($entry in $images) {
    $receipt=(& wsl -d Ubuntu -- sha256sum -- $entry.image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $receipt -cne "$($entry.sha256)  $($entry.image)") {throw 'read-only image differs'}
    "HC074-ORIGINAL-MANAGER-READONLY image=$($entry.sha256) compiler=$($entry.compiler)" | Tee-Object -FilePath (Join-Path $PSScriptRoot "readonly-$($entry.compiler).log")
    & wsl -d Ubuntu -u root -- env HC074_LOCAL_MANAGER_READONLY=1 $entry.image 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::validation_frame::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::barred_custody::barred_collector::enrollment_frame::manager_channel::original_join::tests::original_manager_join_root_readonly_receiver_context' --ignored --exact --nocapture --test-threads=1 2>&1 | Tee-Object -Append -FilePath (Join-Path $PSScriptRoot "readonly-$($entry.compiler).log")
    if ($LASTEXITCODE -ne 0) {throw 'separate root read-only component'}
    $receipt=(& wsl -d Ubuntu -- sha256sum -- $entry.image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $receipt -cne "$($entry.sha256)  $($entry.image)") {throw 'read-only image changed'}
    'HC074-ORIGINAL-MANAGER-READONLY image_unchanged=true' | Tee-Object -Append -FilePath (Join-Path $PSScriptRoot "readonly-$($entry.compiler).log")
}

