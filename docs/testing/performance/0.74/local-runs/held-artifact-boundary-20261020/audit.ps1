$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskSource='148bb3fe25cb94d329e6a35468ecd6838a88fed7'
Push-Location $taskRoot
try {
    function Frozen {
        if((& git rev-parse HEAD).Trim() -cne $taskSource){throw 'Source changed'}
        $taskPaths=@('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','crates/xtask/src','crates/xtask/tests','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
        & git diff --exit-code $taskSource -- @taskPaths | Out-Null
        if($LASTEXITCODE -ne 0 -or (& git status --porcelain -- @taskPaths)){throw 'Frozen input changed'}
    }
    Frozen
    function Run([string]$Name,[string]$Program,[string[]]$CommandArgs,[int]$Passed=-1){
        $taskLog=Join-Path $PSScriptRoot "$Name.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Retain existing log'}
        Write-Host "Running $Name"
        & $Program @CommandArgs 2>&1 | Tee-Object -FilePath $taskLog | Out-Null
        if($LASTEXITCODE -ne 0){throw "Refused: $Name"}
        if($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath $taskLog).Contains("test result: ok. $Passed passed; 0 failed; 0 ignored;")){throw 'Wrong test selection'}
    }
    [ordered]@{source=$taskSource;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();runtime_changed=$false;held_artifact_admission=$false;topology_approved=$false;original_send=$false;server=$false;supervisor=$false;product=$false;new_live_or_fault_matrix=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'identity.json') -Encoding utf8NoBOM
    Run 'windows-compiler' 'rustc' @('-Vv')
    Run 'windows-build' 'cargo' @('test','--locked','-p','hydracache-long-run-supervisor-074','--test','diagnostic_artifacts','--no-run','--message-format=json')
    function Image([string]$Log){
        $taskRows=@(Get-Content -LiteralPath (Join-Path $PSScriptRoot "$Log.log") | Where-Object { $_.StartsWith('{',[StringComparison]::Ordinal) } | ForEach-Object {$_ | ConvertFrom-Json})
        if(@($taskRows | Where-Object { $_.reason -ceq 'build-finished' -and $_.success -ceq $true }).Count -ne 1){throw 'Build incomplete'}
        $taskImages=@($taskRows | Where-Object { $_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'diagnostic_artifacts' -and $_.profile.test -ceq $true -and $_.executable })
        if($taskImages.Count -ne 1){throw 'Wrong artifact image'}
        $taskImages[0].executable
    }
    $taskImage=Image 'windows-build'
    $taskHash=(Get-FileHash -LiteralPath $taskImage -Algorithm SHA256).Hash.ToLowerInvariant()
    [ordered]@{source=$taskSource;image=$taskImage;sha256=$taskHash;expected_tests=17} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'windows-image.json') -Encoding utf8NoBOM
    Run 'windows-artifacts' $taskImage @('--nocapture','--test-threads=1') 17
    if((Get-FileHash -LiteralPath $taskImage -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskHash){throw 'Image drift'}
    foreach($taskPair in @(@('1.94.0','194'),@('1.88.0','188'))){
        $taskVersion=$taskPair[0];$taskSuffix=$taskPair[1]
        $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskSuffix-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
        Run "$taskSuffix-compiler" 'wsl.exe' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$taskVersion",'-Vv')
        Run "$taskSuffix-build" 'wsl.exe' ($taskCargo+@('test','--locked','-p','hydracache-long-run-supervisor-074','--test','diagnostic_artifacts','--no-run','--message-format=json'))
        $taskImage=Image "$taskSuffix-build"
        $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
        if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Missing image'}
        $taskOld=Get-Content -Raw -LiteralPath "docs/testing/performance/0.74/local-runs/fixed-slice-namespace-20261020/gnu$taskSuffix-final/image.json" | ConvertFrom-Json
        $taskProtoImage=$taskOld.image.Substring(66)
        if((& wsl.exe -d Ubuntu -- sha256sum $taskProtoImage).Trim() -cne $taskOld.image){throw 'Retained protocol image changed'}
        [ordered]@{source=$taskSource;compiler=$taskVersion;artifact_image=$taskHash;expected_artifact_tests=31;protocol_image=$taskOld.image;protocol_source_reference=$taskOld.source;expected_protocol_tests=11} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot "$taskSuffix-images.json") -Encoding utf8NoBOM
        Run "$taskSuffix-artifacts" 'wsl.exe' @('-d','Ubuntu','--',$taskImage,'--nocapture','--test-threads=1') 31
        Run "$taskSuffix-protocol" 'wsl.exe' @('-d','Ubuntu','--',$taskProtoImage,'diagnostic_two_phase_launch::tests::','--nocapture','--test-threads=1') 11
        if((& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim() -cne $taskHash -or (& wsl.exe -d Ubuntu -- sha256sum $taskProtoImage).Trim() -cne $taskOld.image){throw 'Linux image drift'}
    }
    Run 'windows-clippy' 'cargo' @('clippy','--locked','-p','hydracache-long-run-supervisor-074','-p','xtask','--all-targets','--all-features','--','-D','warnings')
    Run 'docs' 'cargo' @('xtask','doc-check')
    Run 'contract' 'cargo' @('xtask','performance-contract-check','--release','0.74')
    & cargo fmt -p xtask -p hydracache-long-run-supervisor-074 -- --check
    if($LASTEXITCODE -ne 0){throw 'Formatting'}
    Frozen
    $taskQualification=(Get-FileHash -LiteralPath 'docs/testing/performance/0.74/qualification-manifest.toml' -Algorithm SHA256).Hash.ToLowerInvariant()
    if($taskQualification -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Qualification changed'}
    [ordered]@{source=$taskSource;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();windows_artifact_tests=17;artifact_tests_per_linux_compiler=31;protocol_tests_per_retained_linux_image=11;runtime_sources=437;runtime_changed=$false;qualification_manifest_sha256=$taskQualification;existing_regressions_passed=$true;held_artifact_admission=$false;topology_approved=$false;original_send=$false;server=$false;supervisor=$false;product=$false;new_live_or_fault_matrix=$false;clock_changed=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
