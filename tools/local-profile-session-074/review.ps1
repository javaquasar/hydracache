param([ValidateSet('Facts','WriteSummary','Seal','Review','Audit')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/single-session-rounds-bound-20261021'
$taskPrior=Join-Path (Split-Path $taskPacket) 'validation-round-residual-20261021'
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $taskPacket $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 90 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 90 -Compress)))}
Require ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq '15c86f32b3f56c0c86ac526ad72b54b32aa3cf75625543cea3dd3d9aee9c2807') 'Previous packet anchor changed'
& (Join-Path $taskPrior 'preflight-review.ps1') -Mode Review|Out-Null
# Reuse the exact sealed conservation predicates, not a reduced new reviewer.
# Only these function definitions are evaluated, never the old packet entrypoint.
$taskTokens=$null;$taskErrors=$null
$taskAst=[System.Management.Automation.Language.Parser]::ParseFile((Join-Path $taskPrior 'review.ps1'),[ref]$taskTokens,[ref]$taskErrors)
Require ($taskErrors.Count -eq 0) 'Pinned reviewer parse error'
foreach($taskName in @('Rows','Inactive','Validate-Phases')){
    $taskDefinition=@($taskAst.FindAll({param($Node) $Node -is [System.Management.Automation.Language.FunctionDefinitionAst]},$false)|Where-Object Name -ceq $taskName)
    Require ($taskDefinition.Count -eq 1) 'Pinned validation function missing'
    . ([scriptblock]::Create($taskDefinition[0].Extent.Text))
}
$taskOld=@(Rows (Join-Path $taskPrior '../validation-round-cost-20261020/194-Profile-0.log') 'HC074-HELD-PHASE ')
if($Mode -ceq 'Audit'){
    $taskControls=@()
    foreach($taskMutation in @('parent','missing','edge','scope')){
        $taskCopy=$taskOld|ConvertTo-Json -Depth 90|ConvertFrom-Json
        $taskRejected=$false
        try{Validate-Phases $taskCopy $taskOld $taskMutation|Out-Null}catch{$taskRejected=$true}
        Require $taskRejected "Accepted mutation $taskMutation"
        $taskControls+=@{mutation=$taskMutation;refused=$true}
    }
    @{mutations=$taskControls;server=$false;product=$false}|ConvertTo-Json -Depth 8
    exit
}
function Range($Values){@{minimum=($Values|Measure-Object -Minimum).Minimum;maximum=($Values|Measure-Object -Maximum).Maximum}}
function Facts{
    $taskResult=Json 'result.json';$taskIntent=Json 'intent.json';$taskPins=Json 'single-session-rounds-bound-20261021-pins.json'
    $taskSource=$taskResult.source
    foreach($taskName in @('intent.json','result.json')){
        Require ((Hash (Join-Path $taskPacket "tooling-refusals/verified/$taskName")) -ceq (Hash (Join-Path (Split-Path $taskPacket) "single-session-rounds-verified-20261021/$taskName"))) 'Original zero-profile refusal changed'
    }
    $taskRefused=Json 'tooling-refusals/verified/result.json'
    Require ($taskRefused.source -ceq 'deea7ea96a97119df13a9b0c0007e250c50f51aa' -and $taskRefused.measured_profile_invocations -eq 0 -and -not $taskRefused.profiles.Count -and $taskRefused.failure.Contains('Postflight: CalledProcessError') -and -not $taskRefused.schedule_completed) 'Git preflight refusal erased'
    Require ($taskSource -cmatch '^[0-9a-f]{40}$' -and $taskPins.source -ceq $taskSource -and $taskIntent.source -ceq $taskSource) 'Source binding changed'
    & git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d $taskSource -- tools/long-run-supervisor-074 crates Cargo.lock
    Require ($LASTEXITCODE -eq 0) 'Runtime/observer changed'
    Inactive $taskResult @('server','product','original_send','native_gate_open','full_matrix')
    Require ((Same @($taskIntent.original_windows_seconds) @(300,60,10)) -and $taskIntent.child_timeout_seconds -eq 1200 -and $taskIntent.kill_after_seconds -eq 5 -and $taskIntent.child_containment_seconds -eq 1210 -and $taskIntent.session_seconds -eq 7500 -and $taskIntent.max_log_bytes -eq 33554432) 'Original or wrapper bounds changed'
    $taskMetadata=Json 'metadata.json';$taskPostflight=Json 'postflight.json'
    Require ((Same $taskMetadata $taskPostflight) -and -not $taskMetadata.fixtures.Count -and -not $taskMetadata.tracked_dirty) 'Session continuity not proved'
    Require ($taskMetadata.source -ceq $taskSource -and $taskMetadata.pins_sha256 -ceq (Hash (Join-Path $taskPacket 'single-session-rounds-bound-20261021-pins.json')) -and $taskMetadata.driver_sha256 -ceq (Hash (Join-Path $taskPacket 'method/runner.py')) -and $taskMetadata.preparation_sha256 -ceq (Hash (Join-Path $taskPacket 'single-session-rounds-bound-20261021-preparation.json'))) 'Method or preparation bytes changed'
    foreach($taskName in @('runner.py','launch.ps1','test_runner.py','test-launch.ps1')){
        $taskBlob=(& git rev-parse "${taskSource}:tools/local-profile-session-074/$taskName").Trim()
        $taskStored=(& git hash-object --no-filters -- (Join-Path $taskPacket "method/$taskName")).Trim()
        Require ($taskBlob -ceq $taskStored) 'Retained method differs from measured source'
    }
    Require ($taskMetadata.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -and $taskMetadata.qualification_sha256 -ceq (Hash 'docs/testing/performance/0.74/qualification-manifest.toml')) 'Qualification changed'
    $taskOriginal=Get-Content (Join-Path $taskPrior 'context-Verified-Before.json') -Raw|ConvertFrom-Json
    Require ((Same $taskMetadata.bundle $taskOriginal.bundle) -and (Same $taskMetadata.images $taskPins.images)) 'Frozen inputs changed'
    $taskPreparation=Json 'single-session-rounds-bound-20261021-preparation.json'
    $taskLanes=@();$taskActual=@();$taskInvocations=0
    foreach($taskLane in @('194','188')){
        $taskBuild=$taskPreparation.$taskLane
        Require ($taskBuild.source -ceq $taskSource -and $taskBuild.exit -eq 0 -and $taskBuild.sha256 -ceq $taskPins.images.$taskLane -and $taskBuild.build_log_sha256 -ceq (Hash (Join-Path $taskPacket $taskBuild.build_log))) 'Fresh build binding changed'
        $taskStopped=$false;$taskSuccessful=@();$taskPartial=@()
        foreach($taskSample in 0..2){
            $taskPrefix=Join-Path $taskPacket "$taskLane-Profile-$taskSample"
            $taskExists=Test-Path "$taskPrefix.intent.json"
            Require (-not($taskStopped -and $taskExists)) 'Continuation after first refusal'
            if(-not $taskExists){continue}
            $taskInvocations++
            $taskRow=Json "$taskLane-Profile-$taskSample.exit.json"
            Require ($taskRow.lane -ceq $taskLane -and $taskRow.sample -eq $taskSample -and $taskRow.log_sha256 -ceq (Hash "$taskPrefix.log") -and -not $taskRow.containment_failure) 'Profile binding changed'
            foreach($taskStage in @('before','after')){Require (Same (Json "$taskLane-$taskSample-$taskStage.json") $taskMetadata) 'Profile context drift'}
            $taskActual+=$taskRow
            $taskPhases=@(Rows "$taskPrefix.log" 'HC074-HELD-PHASE ')
            if($taskRow.exit -eq 0){
                Validate-Phases $taskPhases $taskOld|Out-Null
                Require ((Get-Content "$taskPrefix.log" -Raw).Contains('actual_fixed=true actual_consuming=true original_bounds=true eof=true fence_retained=true custodians_terminal=true synthetic_images=true original_send=false completion=false server=false product=false')) 'Original terminal assertions missing'
                $taskSuccessful+=,@($taskPhases)
            }else{
                $taskStopped=$true
                $taskOriginalRows=@(Rows "$taskPrefix.log" 'HC074-HELD-LIVE-ORIGINAL ')
                Require ($taskOriginalRows.Count -eq 1 -and $taskOriginalRows[0].returned -eq $false) 'Original profile refusal missing'
                Inactive $taskOriginalRows[0] @('deadline_changed','guard_clock_added','server','product')
                foreach($taskPhaseRow in $taskPhases){Inactive $taskPhaseRow @('deadline_changed','guard_clock_added','server','product')}
                $taskPartial+=@{sample=$taskSample;exit=$taskRow.exit;partial_phase_count=$taskPhases.Count;original=$taskOriginalRows}
            }
        }
        Require ($taskStopped -or $taskSuccessful.Count -eq 3) 'Unexplained missing profile'
        $taskComplete=$taskSuccessful.Count -eq 3 -and -not $taskStopped
        $taskRanges=@()
        if($taskComplete){foreach($taskPhase in $taskOld.phase){
            $taskRows=@($taskSuccessful|ForEach-Object {$_}|Where-Object phase -ceq $taskPhase)
            $taskRanges+=@{phase=$taskPhase;wall_ns=(Range $taskRows.wall_ns);parent_process_cpu_ns=(Range $taskRows.process_cpu_ns)}
        }}
        $taskLanes+=@{lane=$taskLane;successful_samples=$taskSuccessful.Count;complete_profile_cohort=$taskComplete;stopped_on_refusal=$taskStopped;partial_profiles=$taskPartial;ranges=$taskRanges}
    }
    Require ((Same $taskActual @($taskResult.profiles)) -and $taskInvocations -eq $taskResult.measured_profile_invocations) 'Profile schedule/result differs'
    Require (-not $taskResult.failure -and $taskResult.schedule_completed -eq (@($taskLanes|Where-Object complete_profile_cohort -ne $true).Count -eq 0)) 'Result completeness changed'
    $taskLaunch=Json 'single-session-rounds-bound-20261021-launch-exit.json'
    Require ($taskLaunch.source -ceq $taskSource -and $taskLaunch.single_profile_session -eq $true -and $taskLaunch.session_log_sha256 -ceq (Hash (Join-Path $taskPacket 'single-session-rounds-bound-20261021-session.log')) -and $taskLaunch.recipe_sha256 -ceq (Hash (Join-Path $taskPacket 'method/launch.ps1')) -and $taskLaunch.exit -eq $(if($taskResult.schedule_completed){0}else{1})) 'Launcher receipt changed'
    @{source=$taskSource;runtime_changed=$false;prior_packets_verified=21;boot=$taskMetadata.boot;parent_pid=$taskMetadata.pid;parent_start_ticks=$taskMetadata.start_ticks;session_continuity_observed=$true;measured_profile_invocations=$taskInvocations;lanes=$taskLanes;complete_profile_cohort=$taskResult.schedule_completed;old_time_refusals_retained=$true;native_gate_open=$false;server=$false;product=$false}
}
function Inventory{@{files=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($taskPacket.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
$taskFacts=Facts
if($Mode -ceq 'Facts'){$taskFacts|ConvertTo-Json -Depth 90;exit}
if($Mode -ceq 'WriteSummary'){
    Require (-not(Test-Path (Join-Path $taskPacket 'summary.json'))) 'Never replace summary'
    $taskFacts|ConvertTo-Json -Depth 90|Set-Content -Encoding utf8NoBOM (Join-Path $taskPacket 'summary.json')
}else{Require (Same $taskFacts (Json 'summary.json')) 'Summary changed'}
if($Mode -in @('Seal','Review')){
    $taskChecks=Json 'final-check-exit.json'
    Require (-not $taskChecks.failure -and ($taskChecks.steps.name -join ',') -ceq 'format,docs,registry,performance,diff' -and @($taskChecks.steps|Where-Object exit -ne 0).Count -eq 0 -and $taskChecks.recipe_sha256 -ceq (Hash (Join-Path $taskPacket 'final-check.ps1'))) 'Repository checks incomplete'
    foreach($taskStep in $taskChecks.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $taskPacket $taskStep.log))) 'Repository log changed'}
    Require ((Get-Content (Join-Path $taskPacket 'single-session-method-root-tests-bound-verified.log') -Raw).Contains('Ran 29 tests') -and (Get-Content (Join-Path $taskPacket 'single-session-method-root-tests-bound-verified.log') -Raw).Contains("`nOK")) 'Linux controls incomplete'
    Require ((Get-Content (Join-Path $taskPacket 'single-session-method-windows-tests-bound-verified.log') -Raw).Contains('OK (skipped=7)')) 'Windows controls incomplete'
    Require ((Get-Content (Join-Path $taskPacket 'launcher-controls.log') -Raw).Contains('launcher-controls: 2 passed')) 'Launcher controls incomplete'
    $taskAudit=Json 'reviewer-controls.json'
    Require (($taskAudit.mutations.mutation -join ',') -ceq 'parent,missing,edge,scope' -and @($taskAudit.mutations|Where-Object refused -ne $true).Count -eq 0) 'Conservation controls incomplete'
}
if($Mode -ceq 'Seal'){
    Require (-not(Test-Path (Join-Path $taskPacket 'inventory.json'))) 'Never reseal'
    Inventory|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $taskPacket 'inventory.json')
}
if($Mode -ceq 'Review'){Require (Same (Inventory) (Json 'inventory.json')) 'Inventory changed'}
@{mode=$Mode;integrity_verified=$true;complete_profile_cohort=$taskFacts.complete_profile_cohort;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
