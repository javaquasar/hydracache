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
        if((& git status --porcelain -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests Cargo.lock) -or (& git rev-parse HEAD).Trim() -cne $taskSource){throw 'Source drift'}
    }
    Stable
    $taskSuffix=if($Compiler -eq '1.94.0'){'194'}else{'188'}
    $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskSuffix-check",'/home/artur/.cargo/bin/cargo',"+$Compiler")
    function Run([string]$Name,[string[]]$Arguments){
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log")
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
    }
    Run 'compiler' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$Compiler",'-Vv')
    Run 'clippy' ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
    Run 'build-json' ($taskCargo+@('test','--locked','-p','hydracache-long-run-supervisor-074','--test','diagnostic_artifacts','--no-run','--message-format=json'))
    $taskRows=@(Get-Content -LiteralPath (Join-Path $taskOut 'build-json.log') | Where-Object { $_.StartsWith('{',[StringComparison]::Ordinal) } | ForEach-Object {$_ | ConvertFrom-Json})
    $taskImages=@($taskRows | Where-Object { $_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'diagnostic_artifacts' -and $_.profile.test -ceq $true -and $_.executable })
    if($taskImages.Count -ne 1 -or @($taskRows | Where-Object { $_.reason -ceq 'build-finished' -and $_.success -ceq $true }).Count -ne 1){throw 'Wrong build selection'}
    $taskImage=$taskImages[0].executable
    $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
    if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Image missing'}
    Run 'artifacts' @('-d','Ubuntu','--',$taskImage,'--nocapture','--test-threads=1')
    $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut 'artifacts.log')
    if(-not $taskRaw.Contains('test result: ok. 31 passed; 0 failed; 0 ignored;') -or -not $taskRaw.Contains('HC074-FROZEN-LOCK seed=0x074f10c mutations=32 production_pin_changed=false execution=false')){throw 'Artifact target incomplete'}
    Stable
    if($taskHash -cne (& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()){throw 'Image drift'}
    [ordered]@{source=$taskSource;compiler=$Compiler;image=$taskHash;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();artifact_tests=31;seeded_mutations=32;ordinary_target_passed=$true;runtime_changed=$false;production_pin_changed=$false;new_live_matrix=$false;clock_changed=$false;original_send=$false;server=$false;supervisor=$false;product=$false;qualification=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
