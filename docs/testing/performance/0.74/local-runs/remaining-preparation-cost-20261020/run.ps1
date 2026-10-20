param([Parameter(Mandatory)][ValidateSet('Baseline','Final')][string]$Cohort,
      [Parameter(Mandatory)][ValidateSet('Build','Profile','Allocation','Matrix','Lint')][string]$Stage,
      [Parameter(Mandatory)][ValidateSet('194','188')][string]$Lane,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Frozen {
    if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml)){throw 'Selected source changed'}
}
function Run([string]$Name,[string[]]$Arguments){
    $taskPrefix="$PSScriptRoot/$Cohort-$Lane-$Name"
    if(Test-Path "$taskPrefix-exit.json"){throw 'Never replace a recorded invocation'}
    # No pipeline: capture the native exit immediately, without another command.
    & wsl.exe @Arguments *> "$taskPrefix.log"
    $taskExit=[int]$LASTEXITCODE
    [ordered]@{source=$Source;selection=$Name;exit=$taskExit;recipe_sha256=(Hash $PSCommandPath);server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
    Write-Host "$Cohort $Lane $Name exit=$taskExit"
    return $taskExit
}
Frozen
$taskVersion=if($Lane -ceq '194'){'1.94.0'}else{'1.88.0'}
$taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Lane-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
if($Stage -ceq 'Build'){
    foreach($taskFlavor in 'plain','alloc'){
        $taskArgs=@('test','--locked','--release','-p','hydracache-long-run-supervisor-074','--lib','--no-run','--message-format=json')
        if($taskFlavor -ceq 'alloc'){$taskArgs+=@('--features','fixed-leaf-alloc-probe')}
        if((Run "$taskFlavor-build" ($taskCargo+$taskArgs)) -ne 0){throw 'Build refused'}
        $taskRows=@(Get-Content "$PSScriptRoot/$Cohort-$Lane-$taskFlavor-build.log"|Where-Object{$_.StartsWith('{',[StringComparison]::Ordinal)}|ForEach-Object{$_|ConvertFrom-Json})
        $taskImages=@($taskRows|Where-Object{$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
        if($taskImages.Count -ne 1){throw 'Image selection'}
        $taskPath=$taskImages[0].executable
        [ordered]@{source=$Source;lane=$Lane;flavor=$taskFlavor;path=$taskPath;sha256=(& wsl.exe -d Ubuntu -- sha256sum $taskPath).Trim().Substring(0,64);recipe_sha256=(Hash $PSCommandPath);server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$Cohort-$Lane-$taskFlavor-image.json"
        if((Run "$taskFlavor-differential" @('-d','Ubuntu','--',$taskPath,'projection_fields_','--skip','runtime_binding','--nocapture','--test-threads=1')) -ne 0){throw 'Differential refused'}
        if($taskFlavor -ceq 'plain' -and $Cohort -ceq 'Final'){
            foreach($taskSelection in @(@('kernel','diagnostic_kernel_admission::'),@('bridge','consuming_held'),@('trust','held_runtime::tests::'),@('observer','preparation_cost_observer'),@('boundaries','custody_boundary_capture'))){
                $taskArgs=@('-d','Ubuntu','--',$taskPath,$taskSelection[1]);if($taskSelection[0] -ceq 'trust'){$taskArgs+=@('--skip','consuming_held_bridge')}
                if((Run $taskSelection[0] ($taskArgs+@('--nocapture','--test-threads=1'))) -ne 0){throw 'Regression refused'}
            }
        }
    }
}elseif($Stage -ceq 'Lint'){
    if((Run 'lint' ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))) -ne 0){throw 'Strict lint refused'}
}else{
    $taskFlavor=if($Stage -ceq 'Allocation'){'alloc'}else{'plain'}
    $taskImage=Get-Content -Raw "$PSScriptRoot/$Cohort-$Lane-$taskFlavor-image.json"|ConvertFrom-Json
    if($taskImage.source -cne $Source -or $taskImage.recipe_sha256 -cne (Hash $PSCommandPath) -or (& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64) -cne $taskImage.sha256){throw 'Image/recipe binding'}
    if($Stage -ceq 'Allocation'){
        if((Run 'allocation' @('-d','Ubuntu','--',$taskImage.path,'projection_fields_allocation_attribution','--ignored','--nocapture','--test-threads=1')) -ne 0){throw 'Allocation refused'}
        $taskExit=Run 'runtime-binding' @('-d','Ubuntu','--',$taskImage.path,'projection_fields_runtime_binding','--nocapture','--test-threads=1')
        if($taskExit -ne $(if($Cohort -ceq 'Baseline'){101}else{0})){throw 'Test-first binding mismatch'}
    }else{
        $taskEnv=@('HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017')
        if($Stage -ceq 'Profile'){$taskEnv+=@('HC074_CONSUMING_HELD_LIVE_CASE=healthy','HC074_CONSUMING_HELD_COST=1');$taskSamples=0..2}else{$taskSamples=@('full');$taskEnv=@('-u','HC074_CONSUMING_HELD_LIVE_CASE','-u','HC074_CONSUMING_HELD_COST')+$taskEnv}
        foreach($taskSample in $taskSamples){
            if((Run "$Stage-$taskSample" (@('-d','Ubuntu','-u','root','--','env')+$taskEnv+@('/usr/bin/timeout','--signal=TERM','--kill-after=5s','1200s',$taskImage.path,'privileged_consuming_held_live_matrix','--ignored','--nocapture','--test-threads=1'))) -ne 0){throw 'Actual local preparation refused; no retry'}
        }
    }
    if((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64) -cne $taskImage.sha256){throw 'Image changed'}
}
Frozen
