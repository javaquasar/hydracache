$ErrorActionPreference='Stop'
$images=Get-Content (Join-Path $PSScriptRoot 'Candidate-images.json') -Raw | ConvertFrom-Json
$bundle='/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017'
foreach ($entry in $images) {
    $log=Join-Path $PSScriptRoot "fixtures-$($entry.compiler).log"
    $receipt=(& wsl -d Ubuntu -- sha256sum -- $entry.image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $receipt -cne "$($entry.sha256)  $($entry.image)") {throw 'image differs'}
    foreach ($name in @('original_manager_join_frame_controls','original_manager_join_frame_refusals','original_manager_join_nonroot_fixed_refusal')) {
        & wsl -d Ubuntu -- env "HC074_START_MATERIAL_BUNDLE=$bundle" $entry.image ('diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::'+$name) --ignored --exact --nocapture --test-threads=1 2>&1 | Tee-Object -Append -FilePath $log
        if ($LASTEXITCODE -ne 0) {throw "fixture $name"}
    }
    # Only the exact read-only context case runs as WSL-root, never a fixture suite.
    & wsl -d Ubuntu -u root -- env HC074_LOCAL_MANAGER_READONLY=1 $entry.image 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::validation_frame::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::barred_custody::barred_collector::enrollment_frame::manager_channel::original_join::tests::original_manager_join_root_readonly_receiver_context' --ignored --exact --nocapture --test-threads=1 2>&1 | Tee-Object -Append -FilePath $log
    if ($LASTEXITCODE -ne 0) {throw 'root read-only context'}
    $receipt=(& wsl -d Ubuntu -- sha256sum -- $entry.image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $receipt -cne "$($entry.sha256)  $($entry.image)") {throw 'image changed'}
    "HC074-ORIGINAL-MANAGER-FIXTURES compiler=$($entry.compiler) complete=true start=false" | Tee-Object -Append -FilePath $log
}

