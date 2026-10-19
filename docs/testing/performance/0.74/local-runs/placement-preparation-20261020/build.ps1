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
    Run 'compiler' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$Compiler",'-Vv')
    Run 'clippy' ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
    Run 'trace-units' ($taskCargo+@('test','--release','--locked','-p','hydracache-long-run-supervisor-074','--lib','preparation_trace::tests::','--','--nocapture','--test-threads=1')) 5
    $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
    if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Image missing'}
    foreach($taskTest in @(@('placement-units','::placement_tests::',8),@('launch-units','diagnostic_two_phase_launch::tests::',11),@('enrollment-units','diagnostic_two_phase_launch::root_enrollment::tests::',5),@('owned-units','owned_root_manager_',2),@('origin-units','diagnostic_kernel_admission::origin::',22),@('closing-units','::closing::closing_',4),@('root-units','root_guardian_',9))){
        Run $taskTest[0] @('-d','Ubuntu','--',$taskImage,$taskTest[1],'--nocapture','--test-threads=1') $taskTest[2]
    }
    Run 'doc' ($taskCargo+@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc')) 247
    Stable
    if($taskHash -cne (& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()){throw 'Image drift'}
    [ordered]@{source=$taskSource;compiler=$Compiler;image=$taskHash;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();trace_units=5;selected_units=66;doc_tests=247;profile=$false;runtime_changed=$false;original_send=$false;server=$false;supervisor=$false;product=$false;clock_changed=$false;new_fault_cohort=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
