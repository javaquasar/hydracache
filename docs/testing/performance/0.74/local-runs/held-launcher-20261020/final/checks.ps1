param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-f0-9]{40}$')][string]$Source,
    [Parameter(Mandatory=$true)][ValidateSet('windows','194','188','repository')][string]$Mode
)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../../..')).Path
Push-Location $taskRoot
try {
    function Frozen {
        if((& git rev-parse HEAD).Trim() -cne $Source){throw 'Source changed'}
        $taskPaths=@('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','crates/xtask/src','crates/xtask/tests','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
        & git diff --exit-code $Source -- @taskPaths | Out-Null
        if($LASTEXITCODE -ne 0 -or (& git status --porcelain -- @taskPaths)){throw 'Frozen inputs changed'}
        if((Get-FileHash docs/testing/performance/0.74/qualification-manifest.toml -Algorithm SHA256).Hash.ToLowerInvariant() -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Qualification changed'}
    }
    function Run([string]$Name,[string]$Program,[string[]]$CommandArgs,[int]$Passed=-1,[int]$Exit=0){
        $taskLog=Join-Path $PSScriptRoot "$Mode-$Name.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Retain existing log'}
        Write-Host "Running $Mode-$Name"
        & $Program @CommandArgs 2>&1 | Tee-Object -FilePath $taskLog | Out-Null
        if($LASTEXITCODE -ne $Exit){throw "Refused: $Mode-$Name"}
        if($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath $taskLog).Contains("test result: ok. $Passed passed; 0 failed; 0 ignored;")){throw 'Wrong test selection'}
    }
    Frozen
    $taskRecipe=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;helper_binary_built=($Mode -cne 'repository');original_join=$false;watchdogs_armed=$false;all_writer_revocation=$false;trusted_role_build_production=$false;original_send=$false;server=$false;supervisor=$false;product=$false;full_joined_sigkill=$false;clock_changed=$false} | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot "$Mode-identity.json") -Encoding utf8NoBOM
    if($Mode -ceq 'repository'){
        Run 'source-guards' 'cargo' @('test','--locked','-p','xtask','--test','diagnostic_artifact_fixture_074','--','--nocapture') 10
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
            $taskSelect=@('--bin','diagnostic_held_launcher','--test','held_build')
        }else{
            $taskVersion=if($Mode -ceq '194'){'1.94.0'}else{'1.88.0'}
            $taskProgram='wsl.exe'
            $taskPrefix=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Mode-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
            Run 'compiler' 'wsl.exe' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$taskVersion",'-Vv')
            $taskSelect=@('--bin','diagnostic_held_launcher','--test','held_launcher','--test','held_build')
        }
        Run 'build' $taskProgram ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074')+$taskSelect+@('--no-run','--message-format=json'))
        $taskRows=@(Get-Content (Join-Path $PSScriptRoot "$Mode-build.log") | Where-Object {$_.StartsWith('{',[StringComparison]::Ordinal)} | ForEach-Object {$_|ConvertFrom-Json})
        if(@($taskRows | Where-Object {$_.reason -ceq 'build-finished' -and $_.success -ceq $true}).Count -ne 1){throw 'Build incomplete'}
        function Image([string]$Name,[bool]$IsTest){
            $taskImages=@($taskRows | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq $Name -and $_.profile.test -ceq $IsTest -and $_.executable})
            if($taskImages.Count -ne 1){throw "Wrong image: $Name/$IsTest"};$taskImages[0].executable
        }
        function ImageHash([string]$Path){
            if($Mode -ceq 'windows'){return (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
            $taskLine=(& wsl.exe -d Ubuntu -- sha256sum $Path).Trim()
            if($LASTEXITCODE -ne 0 -or $taskLine -notmatch '^[0-9a-f]{64}  /'){throw 'Missing Linux image'};$taskLine.Substring(0,64)
        }
        $taskHelper=Image 'diagnostic_held_launcher' $false
        $taskHelperHash=ImageHash $taskHelper
        $taskUnit=Image 'diagnostic_held_launcher' $true
        $taskBuild=Image 'held_build' $true
        $taskImages=@([ordered]@{role='helper';path=$taskHelper;sha256=$taskHelperHash},[ordered]@{role='unit';path=$taskUnit;sha256=(ImageHash $taskUnit)},[ordered]@{role='build-authority-tests';path=$taskBuild;sha256=(ImageHash $taskBuild)})
        if($Mode -cne 'windows'){$taskMatrix=Image 'held_launcher' $true;$taskImages+=@([ordered]@{role='process-matrix';path=$taskMatrix;sha256=(ImageHash $taskMatrix)})}
        [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;images=$taskImages;role_receipt_issued=$false} | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $PSScriptRoot "$Mode-images.json") -Encoding utf8NoBOM
        if($Mode -ceq 'windows'){
            Run 'fallback' $taskHelper @() -1 2
            Run 'held-build' $taskBuild @('--nocapture','--test-threads=1') 16
        }else{
            Run 'unit' 'wsl.exe' @('-d','Ubuntu','--',$taskUnit,'--nocapture','--test-threads=1') 3
            foreach($taskRound in 1..2){
                Run "matrix-$taskRound" 'wsl.exe' @('-d','Ubuntu','--',$taskMatrix,'--ignored','--exact','owned_launcher_process_fault_matrix','--nocapture','--test-threads=1') 1
                $taskCases=@(Select-String -LiteralPath (Join-Path $PSScriptRoot "$Mode-matrix-$taskRound.log") -Pattern 'held_launcher seed=0x74e7ec case=')
                if($taskCases.Count -ne 27){throw 'Wrong process case selection'}
            }
            Run 'held-build' 'wsl.exe' @('-d','Ubuntu','--',$taskBuild,'--nocapture','--test-threads=1') 16
            Run 'clippy' $taskProgram ($taskPrefix+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
        }
        Run 'docs' $taskProgram ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc','diagnostic_builder::held_build::','--','--nocapture')) 2
        foreach($taskImage in $taskImages){if((ImageHash $taskImage.path) -cne $taskImage.sha256){throw 'Image changed'}}
    }
    Frozen
    [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;passed=$true;matrix_rounds=$(if($Mode -in @('194','188')){2}else{0});cases_per_round=$(if($Mode -in @('194','188')){27}else{0});original_join=$false;watchdogs_armed=$false;all_writer_revocation=$false;trusted_role_build_production=$false;original_send=$false;server=$false;supervisor=$false;product=$false;full_joined_sigkill=$false;clock_changed=$false} | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot "$Mode-receipt.json") -Encoding utf8NoBOM
}finally{Pop-Location}
