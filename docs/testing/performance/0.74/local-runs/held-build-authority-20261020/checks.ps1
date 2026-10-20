param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-f0-9]{40}$')][string]$Source,
    [Parameter(Mandatory=$true)][ValidateSet('windows','194','188','repository')][string]$Mode
)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Push-Location $taskRoot
try {
    function Frozen {
        if((& git rev-parse HEAD).Trim() -cne $Source){throw 'Source changed'}
        $taskPaths=@('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','crates/xtask/src','crates/xtask/tests','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
        & git diff --exit-code $Source -- @taskPaths | Out-Null
        if($LASTEXITCODE -ne 0 -or (& git status --porcelain -- @taskPaths)){throw 'Frozen input changed'}
        if((Get-FileHash -LiteralPath 'docs/testing/performance/0.74/qualification-manifest.toml' -Algorithm SHA256).Hash.ToLowerInvariant() -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Qualification changed'}
    }
    function Run([string]$Name,[string]$Program,[string[]]$CommandArgs,[int]$Passed=-1){
        $taskLog=Join-Path $PSScriptRoot "$Mode-$Name.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Retain existing log'}
        Write-Host "Running $Mode-$Name"
        & $Program @CommandArgs 2>&1 | Tee-Object -FilePath $taskLog | Out-Null
        if($LASTEXITCODE -ne 0){throw "Refused: $Mode-$Name"}
        if($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath $taskLog).Contains("test result: ok. $Passed passed; 0 failed; 0 ignored;")){throw 'Wrong test selection'}
    }
    Frozen
    $taskRecipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;local_topology_approved=$true;static_build_authority=$true;actual_role_build=$false;original_join=$false;original_send=$false;server=$false;supervisor=$false;product=$false;new_live_or_fault_matrix=$false;clock_changed=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot "$Mode-identity.json") -Encoding utf8NoBOM
    if($Mode -ceq 'repository'){
        Run 'source-guards' 'cargo' @('test','--locked','-p','xtask','--test','diagnostic_artifact_fixture_074','--','--nocapture') 8
        Run 'old-bridge' 'cargo' @('test','--locked','-p','xtask','--test','performance_contract_074','fixed_slice_','--','--nocapture') 3
        Run 'clippy' 'cargo' @('clippy','--locked','-p','hydracache-long-run-supervisor-074','-p','xtask','--all-targets','--all-features','--','-D','warnings')
        Run 'registry' 'cargo' @('xtask','gated-test-check')
        Run 'docs' 'cargo' @('xtask','doc-check')
        Run 'contract' 'cargo' @('xtask','performance-contract-check','--release','0.74')
        & cargo fmt -p xtask -p hydracache-long-run-supervisor-074 -- --check
        if($LASTEXITCODE -ne 0){throw 'Formatting'}
    }else{
        if($Mode -ceq 'windows'){
            $taskProgram='cargo';$taskPrefix=@()
            Run 'compiler' 'rustc' @('-Vv')
        }else{
            $taskVersion=if($Mode -ceq '194'){'1.94.0'}else{'1.88.0'}
            $taskProgram='wsl.exe'
            $taskPrefix=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Mode-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
            Run 'compiler' 'wsl.exe' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$taskVersion",'-Vv')
        }
        Run 'build' $taskProgram ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--test','held_build','--test','diagnostic_artifacts','--test','diagnostic_receipts','--no-run','--message-format=json'))
        $taskRows=@(Get-Content -LiteralPath (Join-Path $PSScriptRoot "$Mode-build.log") | Where-Object { $_.StartsWith('{',[StringComparison]::Ordinal) } | ForEach-Object {$_ | ConvertFrom-Json})
        if(@($taskRows | Where-Object { $_.reason -ceq 'build-finished' -and $_.success -ceq $true }).Count -ne 1){throw 'Build incomplete'}
        foreach($taskName in @('held_build','diagnostic_artifacts','diagnostic_receipts')){
            $taskImages=@($taskRows | Where-Object { $_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq $taskName -and $_.profile.test -ceq $true -and $_.executable })
            if($taskImages.Count -ne 1){throw 'Wrong artifact image'}
            $taskImage=$taskImages[0].executable
            $taskPassed=if($taskName -ceq 'held_build'){16}elseif($taskName -ceq 'diagnostic_artifacts'){if($Mode -ceq 'windows'){17}else{31}}else{if($Mode -ceq 'windows'){8}else{9}}
            function ImageHash {
                if($Mode -ceq 'windows'){return (Get-FileHash -LiteralPath $taskImage -Algorithm SHA256).Hash.ToLowerInvariant()}
                $taskLine=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
                if($LASTEXITCODE -ne 0 -or $taskLine -notmatch '^[0-9a-f]{64}  /'){throw 'Missing Linux image'}
                return $taskLine.Substring(0,64)
            }
            $taskHash=ImageHash
            [ordered]@{source=$Source;image=$taskImage;sha256=$taskHash;expected_tests=$taskPassed;actual_role_build=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot "$Mode-$taskName-image.json") -Encoding utf8NoBOM
            if($Mode -ceq 'windows'){Run $taskName $taskImage @('--nocapture','--test-threads=1') $taskPassed}
            else{Run $taskName 'wsl.exe' @('-d','Ubuntu','--',$taskImage,'--nocapture','--test-threads=1') $taskPassed}
            if((ImageHash) -cne $taskHash){throw 'Image changed'}
        }
        Run 'docs' $taskProgram ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc','diagnostic_builder::held_build::','--','--nocapture')) 2
        if($Mode -cne 'windows'){
            Run 'clippy' $taskProgram ($taskPrefix+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
        }
    }
    Frozen
    [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;passed=$true;original_join=$false;original_send=$false;actual_role_build=$false;production_enrollment=$false;server=$false;supervisor=$false;product=$false;new_live_or_fault_matrix=$false;clock_changed=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot "$Mode-receipt.json") -Encoding utf8NoBOM
}finally{Pop-Location}
