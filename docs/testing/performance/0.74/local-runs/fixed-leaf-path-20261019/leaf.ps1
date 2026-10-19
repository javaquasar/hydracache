param(
    [Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label,
    [Parameter(Mandatory)][ValidateSet('build','measure')][string]$Stage
)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
Push-Location $taskRoot
try {
    $taskSource=(& git rev-parse HEAD).Trim()
    $taskSelected=@('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/fixed-leaf-path-contract.toml')
    function Stable {
        if((& git status --porcelain -- @taskSelected) -or (& git rev-parse HEAD).Trim() -cne $taskSource){throw 'Selected source drift'}
    }
    Stable
    $taskSuffix=if($Compiler -eq '1.94.0'){'194'}else{'188'}
    $taskCache="/home/artur/.cache/hydracache-074-$taskSuffix-check"
    $taskImage=if($Compiler -eq '1.94.0'){"$taskCache/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098"}else{"$taskCache/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234"}
    $taskRecipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    function HashImage {
        $taskValue=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
        if($LASTEXITCODE -ne 0 -or $taskValue -notmatch '^[0-9a-f]{64}  /'){throw 'Image unavailable'}
        $taskValue
    }
    function Run([string]$Name,[string[]]$Arguments,[int]$Passed=-1){
        $taskLog=Join-Path $taskOut "$Name.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Retain previous evidence'}
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath $taskLog
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
        if($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath $taskLog).Contains("test result: ok. $Passed passed; 0 failed;")){throw 'Zero/wrong test selection'}
    }
    if($Stage -eq 'build'){
        if(Test-Path -LiteralPath $taskOut){throw 'Do not overwrite cohort'}
        New-Item -ItemType Directory -Path $taskOut | Out-Null
        $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=$taskCache",'/home/artur/.cargo/bin/cargo',"+$Compiler")
        Run 'clippy' ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
        Run 'leaf-units' ($taskCargo+@('test','--release','--locked','-p','hydracache-long-run-supervisor-074','--lib','fixed_leaf_path_','--','--nocapture','--test-threads=1')) 2
        $taskHash=HashImage
        Run 'origin-regressions' @('-d','Ubuntu','--',$taskImage,'diagnostic_kernel_admission::origin::','--nocapture','--test-threads=1') 18
        Stable
        [ordered]@{source=$taskSource;compiler=$Compiler;image=$taskHash;recipe=$taskRecipe;server=$false;product=$false;leaf_units=2;origin_units=18} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'build.json') -Encoding utf8NoBOM
        return
    }
    $taskBuild=Get-Content -Raw -LiteralPath (Join-Path $taskOut 'build.json') | ConvertFrom-Json
    if($taskBuild.source -cne $taskSource -or $taskBuild.compiler -cne $Compiler -or $taskBuild.recipe -cne $taskRecipe -or $taskBuild.image -cne (HashImage)){throw 'Build mismatch'}
    foreach($taskRepeat in 1..3){
        Run "leaf-$taskRepeat" @('-d','Ubuntu','--',$taskImage,'fixed_leaf_path_cost_abba','--ignored','--nocapture','--test-threads=1') 1
        if(-not (Get-Content -Raw -LiteralPath (Join-Path $taskOut "leaf-$taskRepeat.log")).Contains('HC074-LEAF-PATH-CLOSED observations_unchanged=true cached=false product=false')){throw 'Incomplete paired run'}
    }
    Stable
    if($taskBuild.image -cne (HashImage)){throw 'Image drift'}
    [ordered]@{source=$taskSource;compiler=$Compiler;image=$taskBuild.image;recipe=$taskRecipe;repeats=3;blocks_per_leaf=45;iterations=16384;server=$false;product=$false;admission=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
} finally {Pop-Location}
