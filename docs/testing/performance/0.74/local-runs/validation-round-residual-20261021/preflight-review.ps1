param([ValidateSet('Facts','WriteSummary','Seal','Review','Audit')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 30 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 30 -Compress)))}
function Inactive($Row,[string[]]$Flags){foreach($taskFlag in $Flags){Require ($Row.$taskFlag -is [bool] -and -not $Row.$taskFlag) "Scope changed: $taskFlag"}}
function Refusal($R){
    Require ($R.schema -ceq 'validation-round-residual-context-refusal-074-v1' -and $R.reason -ceq 'boot-changed-before-profile' -and $R.source -cmatch '^[0-9a-f]{40}$') 'Refusal identity changed'
    Require ($R.before_boot -cmatch '^[0-9a-f-]{36}$' -and $R.after_boot -cmatch '^[0-9a-f-]{36}$' -and $R.before_boot -cne $R.after_boot) 'Boot refusal erased'
    Require ($R.measured_profile_invocations -eq 0 -and $R.full_matrix_invocations -eq 0) 'Preflight cannot hide profiles'
    Inactive $R @('continued_after_context_refusal','boot_change_actor_proved','clock_or_service_changed','server','product','native_gate_open')
}
function FinalChecks{
    $taskFinal=Json 'final-check-exit.json'
    Require (-not $taskFinal.failure -and ($taskFinal.steps.name -join ',') -ceq 'format,docs,registry,performance,diff' -and @($taskFinal.steps|Where-Object exit -ne 0).Count -eq 0 -and $taskFinal.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'final-check.ps1'))) 'Final checks incomplete'
    foreach($taskStep in $taskFinal.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Final log changed'}
    Inactive $taskFinal @('server','product')
}
function Facts{
    $taskRefusal=Json 'context-refusal.json';Refusal $taskRefusal
    Require (@(Get-ChildItem -LiteralPath $PSScriptRoot -File|Where-Object {$_.Name -match '^(194|188)-(Profile-|cohort)'}).Count -eq 0) 'No profile or continuation after refusal'
    & git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d $taskRefusal.source -- tools/long-run-supervisor-074 crates/xtask Cargo.lock
    Require ($LASTEXITCODE -eq 0) 'Runtime/observer changed'
    $taskBefore=Json 'context-Verified-Before.json';$taskAfter=Json 'context-Verified-After.json'
    & git merge-base --is-ancestor $taskBefore.source $taskRefusal.source
    Require ($LASTEXITCODE -eq 0 -and $taskAfter.source -ceq $taskRefusal.source) 'Context source changed'
    Require (($taskBefore.commands|Where-Object name -ceq 'boot').output.Trim() -ceq $taskRefusal.before_boot -and ($taskAfter.commands|Where-Object name -ceq 'boot').output.Trim() -ceq $taskRefusal.after_boot) 'Raw boot differs'
    $taskOriginalInputs=Get-Content (Join-Path $PSScriptRoot '../validation-round-cost-20261020/inputs-before.json') -Raw|ConvertFrom-Json
    foreach($taskContext in @($taskBefore,$taskAfter)){
        Require ($taskContext.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'context-repair.ps1')) -and ($taskContext.commands.name -join ',') -ceq 'boot,config,kernel,fixtures,compiler194,compiler188' -and @($taskContext.commands|Where-Object {$_.exit -ne 0 -or $_.output.Contains('auto-installed')}).Count -eq 0) 'Verified context incomplete'
        Require (-not ($taskContext.commands|Where-Object name -ceq 'fixtures').output.Trim()) 'Owned residue'
        Require ($taskContext.bundle.Count -eq 9 -and (Same $taskContext.bundle $taskOriginalInputs.bundle) -and $taskContext.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Frozen inputs changed'
        Inactive $taskContext @('clocks_changed','services_changed','server','product')
    }
    Require ((Same $taskBefore.windows_config $taskAfter.windows_config) -and ($taskBefore.commands|Where-Object name -ceq 'config').output -ceq ($taskAfter.commands|Where-Object name -ceq 'config').output) 'Configuration changed'
    $taskInitial=Json 'context-Before.json';$taskCleanup=Json 'root-toolchain-cleanup.json'
    Require ($taskInitial.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'context.ps1')) -and @($taskInitial.commands|Where-Object {$_.output.Contains('has been auto-installed')}).Count -eq 2) 'Initial tool side effect erased'
    Require ($taskCleanup.exit -eq 0 -and $taskCleanup.list_exit -eq 0 -and $taskCleanup.after.Contains('no installed toolchains') -and $taskCleanup.preflight_auto_installed -eq $true -and $taskCleanup.read_only_query_was_false -eq $true) 'Root toolchain cleanup incomplete'
    Require (Same @($taskCleanup.removed_exact_root_toolchains) @('1.94.0-x86_64-unknown-linux-gnu','1.88.0-x86_64-unknown-linux-gnu')) 'Wrong cleanup target'
    Inactive $taskCleanup @('measured_invocations_started','artur_toolchains_touched','services_changed','clocks_changed','server','product')
    $taskAuthoring=Json 'authoring-refusal.json'
    Require ($taskAuthoring.measured_invocations_started -eq $false -and $taskAuthoring.refused_recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'review-refused.ps1'))) 'Authoring refusal erased'
    $taskAudit=Json 'review-audit.json'
    Require (($taskAudit.mutations.mutation -join ',') -ceq 'parent,missing,edge,scope,continue-after-refusal,source,image,bundle' -and @($taskAudit.mutations|Where-Object refused -ne $true).Count -eq 0) 'Round falsifying controls missing'
    $taskPreflightAudit=Json 'preflight-review-audit.json'
    Require (($taskPreflightAudit.mutations.mutation -join ',') -ceq 'profile-count,reason,admission,boot' -and @($taskPreflightAudit.mutations|Where-Object refused -ne $true).Count -eq 0) 'Preflight falsifying controls missing'
    $taskLanes=@()
    foreach($taskLane in @('194','188')){
        $taskOld=Json "check-$taskLane-exit.json"
        Require ($taskOld.failure.Contains('diagnostic_kernel_admission::.log') -and $taskOld.steps.Count -eq 7 -and $taskOld.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check.ps1'))) 'Filename refusal erased'
        $taskCheck=Json "check-Verified-$taskLane-exit.json"
        Require ($taskCheck.source -ceq $taskRefusal.source -and $taskCheck.lane -ceq $taskLane -and -not $taskCheck.failure -and $taskCheck.steps.Count -eq 11 -and $taskCheck.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check-repair.ps1'))) 'Verified checks incomplete'
        $taskPassed=0
        foreach($taskPair in @(@{receipt=$taskOld;old=$true},@{receipt=$taskCheck;old=$false})){
            Require (@($taskPair.receipt.steps|Where-Object exit -ne 0).Count -eq 0) 'Native focused test failed'
            foreach($taskStep in $taskPair.receipt.steps){
                $taskLog=Join-Path $PSScriptRoot $taskStep.log
                Require ((Hash $taskLog) -ceq $taskStep.sha256) 'Check log changed'
                if(-not $taskPair.old){foreach($taskMatch in [regex]::Matches((Get-Content $taskLog -Raw),'test result: ok\. ([0-9]+) passed;')){$taskPassed+=[int]$taskMatch.Groups[1].Value}}
            }
        }
        Require ($taskPassed -eq 155) 'Expected complete focused coverage'
        $taskImages=@(Get-Content (Join-Path $PSScriptRoot "check-Verified-$taskLane-build.log")|Where-Object {$_.StartsWith('{')}|ForEach-Object {$_|ConvertFrom-Json}|Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
        Require ($taskImages.Count -eq 1 -and $taskImages[0].features.Count -eq 0) 'Default test image missing'
        $taskLanes+=@{lane=$taskLane;focused_tests_passed=$taskPassed;check_and_strict_lint_passed=$true;profiles_started=0;complete_profile_cohort=$false;default_test_image=$taskImages[0].executable}
    }
    $taskPrior=Join-Path (Split-Path $PSScriptRoot) 'authority-open-name-20261021'
    Require ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq '7d5a88964d1df7498aced1d5ce81fbd8b4ad16fe693645bb5044503717a67ce8') 'Prior anchor changed'
    & (Join-Path $taskPrior 'final-review.ps1') -Mode Review|Out-Null
    @{schema='validation-round-residual-preflight-074-v1';source=$taskRefusal.source;runtime_baseline='b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d';context_refusal=$taskRefusal;lanes=$taskLanes;prior_packets_verified=20;source_runtime_changed=$false;complete_profile_cohort=$false;measured_profile_invocations=0;new_round_cost_measured=$false;preflight_toolchain_side_effect=$true;root_toolchain_cleanup_verified=$true;configuration_unchanged=$true;boot_continuity_proved=$false;old_time_refusals_retained=$true;native_gate_open=$false;server=$false;product=$false}
}
function Inventory{@{schema='sha256-local-file-inventory-v1';files=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
if($Mode -ceq 'Audit'){
    $taskAudits=@();$taskOriginal=Json 'context-refusal.json';Refusal $taskOriginal
    foreach($taskMutation in @('profile-count','reason','admission','boot')){
        $taskChanged=Json 'context-refusal.json'
        switch($taskMutation){'profile-count'{$taskChanged.measured_profile_invocations=1};'reason'{$taskChanged.reason='healthy'};'admission'{$taskChanged.native_gate_open=$true};'boot'{$taskChanged.after_boot=$taskChanged.before_boot}}
        $taskRefused=$false;try{Refusal $taskChanged}catch{$taskRefused=$true}
        Require $taskRefused "Accepted preflight mutation: $taskMutation";$taskAudits+=@{mutation=$taskMutation;refused=$true}
    }
    @{mutations=$taskAudits;server=$false;product=$false}|ConvertTo-Json -Depth 6
    exit
}
$taskFacts=Facts
if($Mode -ceq 'Facts'){$taskFacts|ConvertTo-Json -Depth 30;exit}
if($Mode -ceq 'WriteSummary'){
    Require (-not(Test-Path (Join-Path $PSScriptRoot 'summary.json')) -and -not(Test-Path (Join-Path $PSScriptRoot 'inventory.json'))) 'Never replace summary'
    $taskFacts|ConvertTo-Json -Depth 30|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'summary.json')
}else{Require (Same $taskFacts (Json 'summary.json')) 'Summary changed'}
if($Mode -in @('Seal','Review')){FinalChecks}
if($Mode -ceq 'Seal'){
    Require (-not(Test-Path (Join-Path $PSScriptRoot 'inventory.json'))) 'Never reseal'
    Inventory|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'inventory.json')
}
if($Mode -ceq 'Review'){Require (Same (Inventory) (Json 'inventory.json')) 'Inventory changed'}
@{mode=$Mode;integrity_verified=$true;complete_profile_cohort=$false;measured_profile_invocations=0;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
