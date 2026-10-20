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
    [ordered]@{source=$taskSource;compiler=$Compiler;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();runtime_changed=$false;production_pin_changed=$false;original_send=$false;server=$false;supervisor=$false;product=$false;qualification=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'identity.json') -Encoding utf8NoBOM
    $taskSuffix=if($Compiler -eq '1.94.0'){'194'}else{'188'}
    $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskSuffix-check",'/home/artur/.cargo/bin/cargo',"+$Compiler")
    function Run([string]$Name,[string[]]$Arguments){
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log")
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
    }
    Run 'compiler' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$Compiler",'-Vv')
    Run 'clippy' ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
    Run 'build-json' ($taskCargo+@('test','--locked','-p','hydracache-long-run-supervisor-074','--test','diagnostic_artifacts','--test','diagnostic_receipts','--no-run','--message-format=json'))
    $taskRows=@(Get-Content -LiteralPath (Join-Path $taskOut 'build-json.log') | Where-Object { $_.StartsWith('{',[StringComparison]::Ordinal) } | ForEach-Object {$_ | ConvertFrom-Json})
    if(@($taskRows | Where-Object { $_.reason -ceq 'build-finished' -and $_.success -ceq $true }).Count -ne 1){throw 'Build incomplete'}
    $taskProofs=@()
    foreach($taskTarget in @(@('diagnostic_artifacts',31),@('diagnostic_receipts',9))){
        $taskImages=@($taskRows | Where-Object { $_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq $taskTarget[0] -and $_.profile.test -ceq $true -and $_.executable })
        if($taskImages.Count -ne 1){throw 'Wrong target selection'}
        $taskImage=$taskImages[0].executable
        $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
        if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Image missing'}
        [ordered]@{target=$taskTarget[0];image=$taskHash;expected_tests=$taskTarget[1]} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut "$($taskTarget[0])-identity.json") -Encoding utf8NoBOM
        Run "$($taskTarget[0])-list" @('-d','Ubuntu','--',$taskImage,'--list')
        $taskList=Get-Content -Raw -LiteralPath (Join-Path $taskOut "$($taskTarget[0])-list.log")
        if([regex]::Matches($taskList,'(?m)^.+: test\r?$').Count -ne $taskTarget[1]){throw 'Unexpected listed selection'}
        Run $taskTarget[0] @('-d','Ubuntu','--',$taskImage,'--nocapture','--test-threads=1')
        $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut "$($taskTarget[0]).log")
        if(-not $taskRaw.Contains("test result: ok. $($taskTarget[1]) passed; 0 failed; 0 ignored;")){throw 'Target incomplete'}
        if($taskTarget[0] -ceq 'diagnostic_artifacts' -and -not $taskRaw.Contains('HC074-FROZEN-LOCK seed=0x074f10c mutations=32 production_pin_changed=false execution=false')){throw 'Mutation controls missing'}
        if($taskHash -cne (& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()){throw 'Image drift'}
        $taskProofs+=,[ordered]@{target=$taskTarget[0];passed=$taskTarget[1];image=$taskHash}
    }
    Stable
    [ordered]@{source=$taskSource;compiler=$Compiler;targets=$taskProofs;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();artifact_tests=31;receipt_tests=9;seeded_mutations=32;ordinary_targets_passed=$true;runtime_changed=$false;production_pin_changed=$false;new_live_matrix=$false;clock_changed=$false;original_send=$false;server=$false;supervisor=$false;product=$false;qualification=$false} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
