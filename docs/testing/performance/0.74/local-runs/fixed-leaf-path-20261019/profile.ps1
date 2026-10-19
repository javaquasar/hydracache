param([Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,[Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
if(Test-Path -LiteralPath $taskOut){throw 'Do not overwrite profile'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try{
    $taskSource=(& git rev-parse HEAD).Trim()
    function Stable {
        if((& git status --porcelain -- tools/long-run-supervisor-074 docs/testing/gated-test-registry.toml docs/testing/performance/0.74/fixed-leaf-path-contract.toml) -or (& git rev-parse HEAD).Trim() -cne $taskSource){throw 'Selected source drift'}
    }
    Stable
    $taskSuffix=if($Compiler -eq '1.94.0'){'194'}else{'188'}
    $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskSuffix-check",'/home/artur/.cargo/bin/cargo',"+$Compiler")
    function Run([string]$Name,[string[]]$Arguments,[int]$Passed=-1){
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log")
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
        if($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath (Join-Path $taskOut "$Name.log")).Contains("test result: ok. $Passed passed; 0 failed;")){throw 'Wrong or zero test selection'}
    }
    Run 'clippy' ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
    Run 'build' ($taskCargo+@('test','--release','--locked','-p','hydracache-long-run-supervisor-074','--features','fixed-leaf-alloc-probe','--lib','--no-run','--message-format=json'))
    $taskArtifacts=@(Get-Content -LiteralPath (Join-Path $taskOut 'build.log') | Where-Object { $_.StartsWith('{') } | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'hydracache_long_run_supervisor_074' -and $_.executable })
    if($taskArtifacts.Count -ne 1){throw 'Ambiguous profile image'}
    $taskImage=$taskArtifacts[0].executable
    $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
    if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Image missing'}
    Run 'counter-units' @('-d','Ubuntu','--',$taskImage,'fixed_leaf_alloc_counter_','--nocapture','--test-threads=1') 2
    Run 'origin-units' @('-d','Ubuntu','--',$taskImage,'diagnostic_kernel_admission::origin::','--nocapture','--test-threads=1') 20
    Run 'allocation' @('-d','Ubuntu','--',$taskImage,'fixed_leaf_path_allocation_attribution','--ignored','--nocapture','--test-threads=1') 1
    Stable
    if($taskHash -cne (& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()){throw 'Image drift'}
    [ordered]@{source=$taskSource;compiler=$Compiler;image=$taskHash;profile=$true;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();samples=3;iterations=1024;counter_units=2;origin_units=20;retirement_evidence=$false;server=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
