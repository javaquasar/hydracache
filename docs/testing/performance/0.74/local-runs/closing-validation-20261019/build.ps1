param([Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,[Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
if(Test-Path -LiteralPath $taskOut){throw 'Retain existing cohort'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try {
    $taskSource=(& git rev-parse HEAD).Trim()
    function Stable {
        if((& git status --porcelain -- tools/long-run-supervisor-074/src) -or (& git rev-parse HEAD).Trim() -cne $taskSource){throw 'Source drift'}
    }
    Stable
    $taskSuffix=if($Compiler -eq '1.94.0'){'194'}else{'188'}
    $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskSuffix-check",'/home/artur/.cargo/bin/cargo',"+$Compiler")
    $taskImage=if($Compiler -eq '1.94.0'){'/home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098'}else{'/home/artur/.cache/hydracache-074-188-check/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234'}
    function Run([string]$Name,[string[]]$Arguments,[int]$Passed=-1){
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log")
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
        if($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath (Join-Path $taskOut "$Name.log")).Contains("test result: ok. $Passed passed; 0 failed;")){throw 'Wrong test selection'}
    }
    Run 'clippy' ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
    Run 'owned-units' ($taskCargo+@('test','--release','--locked','-p','hydracache-long-run-supervisor-074','--lib','owned_root_manager_','--','--nocapture','--test-threads=1')) 2
    $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
    if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Image missing'}
    Run 'origin-units' @('-d','Ubuntu','--',$taskImage,'diagnostic_kernel_admission::origin::','--nocapture','--test-threads=1') 18
    Run 'closing-units' @('-d','Ubuntu','--',$taskImage,'::closing::closing_','--nocapture','--test-threads=1') 4
    Run 'root-units' @('-d','Ubuntu','--',$taskImage,'root_guardian_','--nocapture','--test-threads=1')
    if((Get-Content -Raw -LiteralPath (Join-Path $taskOut 'root-units.log')) -notmatch 'test result: ok\. [1-9][0-9]* passed; 0 failed;'){throw 'No root units'}
    Stable
    if($taskHash -cne (& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()){throw 'Image drift'}
    [ordered]@{source=$taskSource;compiler=$Compiler;image=$taskHash;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();owned_units=2;origin_units=18;profile=$false;server=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
