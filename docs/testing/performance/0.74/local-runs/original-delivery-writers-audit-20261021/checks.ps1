$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Value,[string]$Message){if(-not $Value){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function WriteNew([string]$Path,$Value){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject $Value -Depth 30)+"`n")
    $taskStream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
$taskFiles=@(
    'diagnostic_consuming_held_join.rs','diagnostic_fixed_placement.rs','diagnostic_owned_placement.rs',
    'diagnostic_owned_root_manager.rs','diagnostic_original_manager_join.rs','diagnostic_original_manager_message.rs',
    'diagnostic_original_watchdog_context.rs','diagnostic_original_pending_watchdog.rs','diagnostic_original_root_guardians.rs',
    'diagnostic_manager_channel.rs','diagnostic_manager_channel_kernel.rs','diagnostic_manager_receiver_pin.rs',
    'diagnostic_dispatch_exchange.rs','diagnostic_dispatch_exchange_tests.rs','diagnostic_dispatch_reply_state_tests.rs',
    'diagnostic_dispatch_reply_flags_tests.rs','diagnostic_original_descriptor_plan.rs','diagnostic_original_custody_join.rs',
    'diagnostic_original_enrollment_frame.rs','diagnostic_original_enrollment_frame_build.rs',
    'diagnostic_original_barred_custody.rs','diagnostic_original_barred_collector.rs','diagnostic_original_barred_collector_kernel.rs',
    'diagnostic_original_collector_custody_storage.rs','diagnostic_collector_starting_storage.rs',
    'diagnostic_collector_starting_publication.rs','diagnostic_two_phase_launch.rs','diagnostic_two_phase_launch_tests.rs',
    'diagnostic_held_placement.rs','diagnostic_held_placement_tests.rs','diagnostic_held_barrier.rs',
    'diagnostic_held_launcher_kernel.rs','diagnostic_root_enrollment.rs','diagnostic_root_enrollment_tests.rs',
    'diagnostic_cgroup_root.rs','diagnostic_fixed_slice.rs','diagnostic_root_guardian.rs','diagnostic_root_guardian_raw.rs',
    'diagnostic_root_guardian_original_kernel.rs','diagnostic_root_placement_binding.rs',
    'diagnostic_original_root_retirement_tests.rs','diagnostic_lease.rs','diagnostic_ipc.rs','server.rs',
    'bin/diagnostic_held_launcher.rs')
function Context{
    @{source=(& git rev-parse HEAD).Trim();qualification_sha256=(Hash 'docs/testing/performance/0.74/qualification-manifest.toml');
      source_files=@(foreach($taskName in $taskFiles){$taskPath="tools/long-run-supervisor-074/src/$taskName";@{path=$taskPath;sha256=(Hash $taskPath)}});
      fixture_sha256=(Hash 'tools/long-run-supervisor-074/tests/support/consuming_held_live.rs');recipe_sha256=(Hash $PSCommandPath)}
}
$taskSource='3107fe33360f97a17ae336f2da6714832924dd68'
Require ((& git rev-parse HEAD).Trim() -ceq $taskSource) 'Unexpected audit source'
& git diff --quiet 3d3a2180c6b54ea9156ae2bac609f36966359908 HEAD -- tools/long-run-supervisor-074 crates Cargo.lock docs/testing/gated-test-registry.toml
Require ($LASTEXITCODE -eq 0) 'Runtime source changed'
$taskInitial=Context
WriteNew (Join-Path $PSScriptRoot 'context-before.json') $taskInitial
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $PSScriptRoot "$Name.log"
    Require (-not(Test-Path $taskLog)) 'Never overwrite audit evidence'
    & $Call *> $taskLog
    $taskExit=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskExit;log=[IO.Path]::GetFileName($taskLog);sha256=(Hash $taskLog)}
    Require ($taskExit -eq 0) "Diagnostic control refused $Name"
}
$taskSelectors=@(
    @('reply-flags','dispatch_reply_flags_',5),@('reply-state','dispatch_reply_state_',5),
    @('uncertain','dispatch_arms_uncertain_before_transport_and_never_rearms',1),
    @('lost-ack','dispatch_delayed_reply_cannot_heal_a_closed_unknown_outcome',1),
    @('hidden-alias','dispatch_job_and_drop_do_not_prove_cessation_of_hidden_message_aliases',1),
    @('owner-drop','dispatch_failure_and_uncertain_drop_retain_pipe_writers_until_owner_destruction',1),
    @('manager-error','dispatch_manager_error_is_retained_without_exposing_its_payload_or_retry',1),
    @('root-claims','root_enrollment_',5),@('placement-claims','held_placement_',6),
    @('launch-claims','two_phase_launch_',5),
    @('retirement','original_root_retirement_refuses_owner_first_and_cannot_retry_pending_or_consumed',1))
try{
    Record 'prior-review' { & pwsh -NoProfile -File docs/testing/performance/0.74/local-runs/held-original-faults-20261021/final-review.ps1 -Mode Review }
    $taskMetadata=Get-Content docs/testing/performance/0.74/local-runs/held-original-faults-20261021/metadata.json -Raw|ConvertFrom-Json
    $taskPreparation=Get-Content docs/testing/performance/0.74/local-runs/held-original-faults-20261021/held-original-faults-20261021-preparation.json -Raw|ConvertFrom-Json
    foreach($taskLane in @('194','188')){
        $taskImage=$taskPreparation.$taskLane.image
        $taskImageHash=& wsl.exe -d Ubuntu -u artur -- sha256sum -- $taskImage
        Require ($LASTEXITCODE -eq 0 -and $taskImageHash.Split(' ')[0] -ceq $taskMetadata.images.$taskLane) 'Pinned default image changed'
        foreach($taskSelector in $taskSelectors){
            Record "$taskLane-$($taskSelector[0])" { & wsl.exe -d Ubuntu -u artur -- /usr/bin/timeout --signal=TERM --kill-after=5s 120s $taskImage $taskSelector[1] --nocapture --test-threads=1 }
            $taskText=Get-Content (Join-Path $PSScriptRoot "$taskLane-$($taskSelector[0]).log") -Raw
            Require ($taskText -match "test result: ok\. $($taskSelector[2]) passed; 0 failed; 0 ignored;") 'Unexpected selected control count'
        }
        $taskAfterHash=& wsl.exe -d Ubuntu -u artur -- sha256sum -- $taskImage
        Require ($LASTEXITCODE -eq 0 -and $taskAfterHash.Split(' ')[0] -ceq $taskMetadata.images.$taskLane) 'Image changed during audit'
    }
}catch{$taskFailure=$_.Exception.Message}
$taskFinal=Context
WriteNew (Join-Path $PSScriptRoot 'context-after.json') $taskFinal
Require ((ConvertTo-Json -InputObject $taskInitial -Depth 30 -Compress) -ceq (ConvertTo-Json -InputObject $taskFinal -Depth 30 -Compress)) 'Audit source changed'
WriteNew (Join-Path $PSScriptRoot 'checks.json') @{source=$taskSource;steps=$taskSteps;failure=$taskFailure;selectors=$taskSelectors;
    new_original_fixture_invocations=0;root_execution=$false;original_send=$false;full_sigkill_matrix=$false;native_gate_open=$false;server=$false;product=$false}
if($taskFailure){throw $taskFailure}
