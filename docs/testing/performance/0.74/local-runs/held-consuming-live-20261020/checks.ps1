param([Parameter(Mandatory)][ValidateSet('194','188')][string]$Mode,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
Push-Location (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
try {
    if((& git rev-parse HEAD).Trim() -cne $Source){throw 'Source changed'}
    $taskPaths=@('tools/long-run-supervisor-074','crates/xtask','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
    if((& git status --porcelain -- @taskPaths)){throw 'Selected inputs dirty'}
    $taskVersion=if($Mode -ceq '194'){'1.94.0'}else{'1.88.0'}
    $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Mode-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
    function Run([string]$Name,[string[]]$CommandArgs){
        $taskLog=Join-Path $PSScriptRoot "$Mode-$Name.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Retain existing log'}
        & wsl.exe @CommandArgs 2>&1 | Tee-Object -FilePath $taskLog | Out-Null
        $taskExit=$LASTEXITCODE
        [ordered]@{source=$Source;mode=$Mode;selection=$Name;exit=$taskExit}|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot "$Mode-$Name-exit.json") -Encoding utf8NoBOM
        return $taskExit
    }
    if((Run 'build' ($taskCargo+@('test','--locked','--release','-p','hydracache-long-run-supervisor-074','--lib','--no-run','--message-format=json'))) -ne 0){throw 'Build refused'}
    $taskRows=@(Get-Content (Join-Path $PSScriptRoot "$Mode-build.log") | Where-Object {$_.StartsWith('{',[StringComparison]::Ordinal)}|ForEach-Object{$_|ConvertFrom-Json})
    $taskImages=@($taskRows|Where-Object{$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
    if($taskImages.Count -ne 1){throw 'Wrong image'}
    $taskImage=$taskImages[0].executable
    $taskHash=((& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim().Substring(0,64))
    [ordered]@{source=$Source;mode=$Mode;path=$taskImage;sha256=$taskHash;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();release=$true;server=$false;product=$false;original_send=$false;synthetic_role_images=$true}|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot "$Mode-image.json") -Encoding utf8NoBOM
    $taskResults=[ordered]@{}
    $taskResults['models']=Run 'models' @('-d','Ubuntu','--',$taskImage,'consuming_held','--nocapture','--test-threads=1')
    $taskResults['trust']=Run 'trust' @('-d','Ubuntu','--',$taskImage,'held_runtime::tests::','--skip','consuming_held_bridge','--nocapture','--test-threads=1')
    $taskRootPrefix=@('-d','Ubuntu','-u','root','--','env','HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017')
    $taskResults['live']=Run 'live' ($taskRootPrefix+@('/usr/bin/timeout','--signal=TERM','--kill-after=5s','1200s',$taskImage,'privileged_consuming_held_live_matrix','--ignored','--nocapture','--test-threads=1'))
    $taskResults['attribution']=Run 'attribution' ($taskRootPrefix+@('HC074_CONSUMING_HELD_LIVE_CASE=healthy','HC074_CONSUMING_HELD_COST=1','/usr/bin/timeout','--signal=TERM','--kill-after=5s','1200s',$taskImage,'privileged_consuming_held_live_matrix','--ignored','--nocapture','--test-threads=1'))
    $taskResults['lint']=Run 'lint' ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
    if(((& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim().Substring(0,64)) -cne $taskHash){throw 'Image changed'}
    if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- @taskPaths)){throw 'Source changed during checks'}
    [ordered]@{source=$Source;mode=$Mode;results=$taskResults;live_matrix_passed=($taskResults.live -eq 0);attribution_completed=($taskResults.attribution -eq 0);server=$false;product=$false;original_send=$false;full_joined_sigkill=$false;real_role_image_cost=$false}|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot "$Mode-result.json") -Encoding utf8NoBOM
}finally{Pop-Location}
