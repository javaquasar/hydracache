param([Parameter(Mandatory)][ValidateSet('Baseline','Candidate')][string]$Cohort,
      [Parameter(Mandatory)][ValidateSet('Build','Probe')][string]$Stage,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
function Frozen {
    if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml)){throw 'Source changed'}
}
function Run([string]$Name,[string[]]$Arguments){
    $taskPrefix="$PSScriptRoot/$Cohort-$Name"
    if(Test-Path "$taskPrefix-exit.json"){throw 'Never replay a recorded invocation'}
    & wsl.exe @Arguments 2>&1|Tee-Object "$taskPrefix.log"|Out-Null
    $taskExit=$LASTEXITCODE
    [ordered]@{source=$Source;selection=$Name;exit=$taskExit;server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
    return $taskExit
}
Frozen
foreach($taskLane in '194','188'){
    $taskVersion=if($taskLane -ceq '194'){'1.94.0'}else{'1.88.0'}
    $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskLane-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
    if($Stage -ceq 'Build'){
        foreach($taskFlavor in 'plain','alloc'){
            $taskArgs=@('test','--locked','--release','-p','hydracache-long-run-supervisor-074','--lib','--no-run','--message-format=json')
            if($taskFlavor -ceq 'alloc'){$taskArgs+=@('--features','fixed-leaf-alloc-probe')}
            if((Run "$taskLane-$taskFlavor-build" ($taskCargo+$taskArgs)) -ne 0){throw 'Build refused'}
            $taskRows=@(Get-Content "$PSScriptRoot/$Cohort-$taskLane-$taskFlavor-build.log"|Where-Object{$_.StartsWith('{',[StringComparison]::Ordinal)}|ForEach-Object{$_|ConvertFrom-Json})
            $taskImage=@($taskRows|Where-Object{$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
            if($taskImage.Count -ne 1){throw 'Image selection'}
            $taskPath=$taskImage[0].executable
            $taskHash=((& wsl.exe -d Ubuntu -- sha256sum $taskPath).Trim().Substring(0,64))
            [ordered]@{source=$Source;lane=$taskLane;flavor=$taskFlavor;path=$taskPath;sha256=$taskHash;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$Cohort-$taskLane-$taskFlavor-image.json"
        }
        if($Cohort -ceq 'Candidate'){
            $taskImage=Get-Content -Raw "$PSScriptRoot/$Cohort-$taskLane-plain-image.json"|ConvertFrom-Json
            foreach($taskSelection in @(@('fresh-tests','fresh_kernel_cost::'),@('kernel','diagnostic_kernel_admission::'),@('bridge','consuming_held'),@('trust','held_runtime::tests::'),@('observer','preparation_cost_observer'))){
                $taskArgs=@('-d','Ubuntu','--',$taskImage.path,$taskSelection[1])
                if($taskSelection[0] -ceq 'trust'){$taskArgs+=@('--skip','consuming_held_bridge')}
                if((Run "$taskLane-$($taskSelection[0])" ($taskArgs+@('--nocapture','--test-threads=1'))) -ne 0){throw 'Targeted check refused'}
            }
            if((Run "$taskLane-lint" ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))) -ne 0){throw 'Lint refused'}
        }
    }else{
        # Run only after all own builds, lint and tests have finished.
        foreach($taskFlavor in 'plain','alloc'){
            $taskImage=Get-Content -Raw "$PSScriptRoot/$Cohort-$taskLane-$taskFlavor-image.json"|ConvertFrom-Json
            if($taskImage.source -cne $Source -or $taskImage.recipe_sha256 -cne (Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()){throw 'Image source/recipe mismatch'}
            if($taskFlavor -ceq 'plain'){
                if((Run "$taskLane-attribution" @('-d','Ubuntu','--',$taskImage.path,'fresh_kernel_observation_attribution','--ignored','--nocapture','--test-threads=1')) -ne 0){throw 'Fresh attribution refused'}
                $taskExit=Run "$taskLane-runtime-binding" @('-d','Ubuntu','--',$taskImage.path,'fresh_status_runtime_uses_the_measured_fixed_openers','--nocapture','--test-threads=1')
                $taskExpected=if($Cohort -ceq 'Baseline'){101}else{0}
                if($taskExit -ne $taskExpected){throw 'Runtime binding test-first outcome changed'}
            }else{
                if((Run "$taskLane-allocation" @('-d','Ubuntu','--',$taskImage.path,'fresh_status_opener_allocation_attribution','--ignored','--nocapture','--test-threads=1')) -ne 0){throw 'Allocation control refused'}
            }
            if(((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64)) -cne $taskImage.sha256){throw 'Image changed'}
        }
    }
}
Frozen
