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
        if((& git status --porcelain -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests Cargo.lock docs/testing/gated-test-registry.toml crates/xtask/tests) -or (& git rev-parse HEAD).Trim() -cne $taskSource){throw 'Source drift'}
    }
    Stable
    [ordered]@{source=$taskSource;compiler=$Compiler;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();production_runtime_changed=$false;original_join=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'identity.json') -Encoding utf8NoBOM
    $taskSuffix=if($Compiler -eq '1.94.0'){'194'}else{'188'}
    $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskSuffix-check",'/home/artur/.cargo/bin/cargo',"+$Compiler")
    function Run([string]$Name,[string[]]$Arguments){
        Write-Host "Running $Compiler $Name"
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log") | Out-Null
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
    }
    Run 'compiler' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$Compiler",'-Vv')
    Run 'clippy' ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
    Run 'build' ($taskCargo+@('test','--release','--locked','-p','hydracache-long-run-supervisor-074','--lib','--no-run','--message-format=json'))
    $taskRows=@(Get-Content -LiteralPath (Join-Path $taskOut 'build.log') | Where-Object { $_.StartsWith('{',[StringComparison]::Ordinal) } | ForEach-Object {$_ | ConvertFrom-Json})
    if(@($taskRows | Where-Object { $_.reason -ceq 'build-finished' -and $_.success -ceq $true }).Count -ne 1){throw 'Build incomplete'}
    $taskImages=@($taskRows | Where-Object { $_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable })
    if($taskImages.Count -ne 1){throw 'Wrong test image'}
    $taskImage=$taskImages[0].executable
    $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
    if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Image missing'}
    [ordered]@{source=$taskSource;compiler=$Compiler;image=$taskHash;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'image.json') -Encoding utf8NoBOM
    Run 'list' @('-d','Ubuntu','--',$taskImage,'privileged_fixed_slice_namespace_matrix','--list')
    if([regex]::Matches((Get-Content -Raw -LiteralPath (Join-Path $taskOut 'list.log')),'(?m)^.+: test\r?$').Count -ne 1){throw 'Unexpected fixture selection'}
    Run 'ordinary' @('-d','Ubuntu','--',$taskImage,'::fixed_slice::tests::','--nocapture','--test-threads=1')
    if(-not (Get-Content -Raw -LiteralPath (Join-Path $taskOut 'ordinary.log')).Contains('test result: ok. 9 passed; 0 failed; 2 ignored;')){throw 'Ordinary controls incomplete'}
    Stable
    Run 'kernel' @('-d','Ubuntu','-u','root','--','env','HC074_FIXED_SLICE_NAMESPACE=1',$taskImage,'privileged_fixed_slice_namespace_matrix','--ignored','--nocapture','--test-threads=1')
    $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut 'kernel.log')
    foreach($taskCase in @('healthy','different-attempt','named-root-replaced','system-parent-replaced','root-mode-changed','parent-mode-changed','mount-anchor-replaced','original-refused','same-name-foreign-parent')){
        if([regex]::Matches($taskRaw,[regex]::Escape("HC074-FIXED-NAMESPACE case=$taskCase positive_pin=true checked=true observer_readonly=true original_join=false manager_ownership=false send=false")).Count -ne 1){throw "Missing/duplicate case $taskCase"}
    }
    if(-not $taskRaw.Contains('HC074-FIXED-NAMESPACE cases=9 host_anchors_unchanged=true cleanup=true original_join=false server=false supervisor=false product=false')){throw 'Kernel fixture incomplete'}
    if([regex]::Matches($taskRaw,'test result: ok. 1 passed; 0 failed; 0 ignored;').Count -ne 10){throw 'Unexpected parent/child results'}
    Run 'doc' ($taskCargo+@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc'))
    if(-not (Get-Content -Raw -LiteralPath (Join-Path $taskOut 'doc.log')).Contains('test result: ok. 247 passed; 0 failed;')){throw 'Doc controls incomplete'}
    Stable
    if($taskHash -cne (& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()){throw 'Image drift'}
    Run 'cleanup' @('-d','Ubuntu','--','find','/sys/fs/cgroup','-maxdepth','1','-name','hc074-fixed-slice-*','-print')
    $taskCleanup=Join-Path $taskOut 'cleanup.log'
    if((Test-Path -LiteralPath $taskCleanup) -and (Get-Content -Raw -LiteralPath $taskCleanup)){throw 'Temporary cgroup remains'}
    [ordered]@{source=$taskSource;compiler=$Compiler;image=$taskHash;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();ordinary_tests=9;kernel_cases=9;doc_tests=247;host_anchors_unchanged=$true;cleanup=$true;production_runtime_changed=$false;original_join=$false;manager_ownership=$false;exclusive_population=$false;artifact_admission=$false;original_send=$false;server=$false;supervisor=$false;product=$false;qualification=$false;clock_changed=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
