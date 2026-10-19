param([Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label,[Parameter(Mandatory)][string]$Image,[Parameter(Mandatory)][string]$Source,[switch]$Matrix,[ValidateRange(1,3)][int]$Repeats=1)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
if(Test-Path -LiteralPath $taskOut){throw 'Retain existing cohort'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try {
    if((& git status --porcelain -- tools/long-run-supervisor-074/src)){throw 'Runtime dirty'}
    & git diff --quiet $Source -- tools/long-run-supervisor-074/src
    if($LASTEXITCODE -ne 0){throw 'Wrong selected source'}
    $taskHead=(& git rev-parse HEAD).Trim()
    $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $Image).Trim()
    if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Image missing'}
    function Run([string]$Name,[string[]]$Arguments,[int]$Passed=-1){
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log")
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
        if($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath (Join-Path $taskOut "$Name.log")).Contains("test result: ok. $Passed passed; 0 failed;")){throw 'Wrong test selection'}
    }
    $taskBefore=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if($LASTEXITCODE -ne 0 -or $taskBefore -cne 'active'){throw 'Require active local clock service'}
    & wsl.exe -d Ubuntu -u root -- systemctl stop systemd-timesyncd.service
    if($LASTEXITCODE -ne 0){throw 'Local clock stop failed'}
    try {
        Run 'clock-settle' @('-d','Ubuntu','--','python3','-u','-c',"import time
start=time.monotonic(); stable=start; prev=time.time_ns(); steps=0
while time.monotonic()-start<180:
 now=time.time_ns(); mono=time.monotonic()
 if now<prev: steps+=1; stable=mono
 if mono-stable>=65: print('settled=true stable_seconds=65 steps='+str(steps)+' attempt=false'); break
 prev=now; time.sleep(.01)
else: raise RuntimeError('clock not stable')")
        Run 'clock-probe' @('-d','Ubuntu','--','python3','-u','-c',"import time,pathlib,json
start=time.monotonic(); prev=time.time_ns(); n=0
while time.monotonic()-start<65:
 now=time.time_ns()
 if now<prev: raise RuntimeError('REALTIME backwards; no attempt')
 prev=now; n+=1; time.sleep(.01)
print(json.dumps(dict(backward_samples=0,samples=n,boot=pathlib.Path('/proc/sys/kernel/random/boot_id').read_text().strip())))")
        $taskFilter=if($Matrix){'privileged_original_root_manager_writer_matrix'}else{'privileged_original_root_manager_writer_screen'}
        foreach($taskRepeat in 1..$Repeats){
            Run "frame-$taskRepeat" @('-d','Ubuntu','-u','root','--','env','HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017',$Image,$taskFilter,'--ignored','--nocapture','--test-threads=1') 1
            $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut "frame-$taskRepeat.log")
            $taskExpected=if($Matrix){7}else{1}
            if(([regex]::Matches($taskRaw,'HC074-OWNED-ROOT case=')).Count -ne $taskExpected){throw 'Missing frame cases'}
        }
    }finally{
        & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
        if($LASTEXITCODE -ne 0){throw 'Clock restoration failed'}
        $taskAfter=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
        $taskAfter | Set-Content -LiteralPath (Join-Path $taskOut 'clock-restored.log') -Encoding utf8NoBOM
        if($LASTEXITCODE -ne 0 -or $taskAfter -cne 'active'){throw 'Clock restoration unverified'}
    }
    if((& git rev-parse HEAD).Trim() -cne $taskHead -or $taskHash -cne (& wsl.exe -d Ubuntu -- sha256sum $Image).Trim()){throw 'Source/image drift'}
    [ordered]@{source=$Source;image=$taskHash;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();matrix=[bool]$Matrix;repeats=$Repeats;clock_before=$taskBefore;clock_after=$taskAfter;fixed_manager=$true;real_root=$true;test_issuers=$true;original_send=$false;completion=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
