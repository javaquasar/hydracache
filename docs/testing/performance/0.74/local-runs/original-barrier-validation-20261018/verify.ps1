param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
function Require([bool]$Condition, [string]$Message) { if (!$Condition) { throw $Message } }
function Text([string]$Name) { [IO.File]::ReadAllText((Join-Path $packet $Name)).Replace("`r`n", "`n") }
function HashText([string]$Value) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()
}
function GitText([string[]]$Arguments) {
    $start = [Diagnostics.ProcessStartInfo]::new('git')
    $start.WorkingDirectory = $root
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($start)
    $result = $process.StandardOutput.ReadToEnd()
    $process.WaitForExit()
    Require ($process.ExitCode -eq 0) 'git read failed'
    $result
}
function VerifyHost([string]$Log) {
    Require (!$Log.Contains('FAILED')) 'failed host test'
    $cohorts = [regex]::Matches($Log, '(?s)HC074-ORIGINAL-BARRIER-COHORT compiler=(194|188) begin=true(.*?)HC074-ORIGINAL-BARRIER-COHORT compiler=\1 completed=true')
    Require ($cohorts.Count -eq 2) 'missing/duplicate cohort'
    Require ($cohorts[0].Groups[1].Value -eq '194' -and $cohorts[1].Groups[1].Value -eq '188') 'cohort order'
    foreach ($cohort in $cohorts) {
        $body = $cohort.Groups[2].Value
        Require ($body.Contains('test result: ok. 4 passed; 0 failed;')) 'missing fast units'
        $last = 0
        foreach ($phase in 'healthy','refusals','sigkill') {
            $begin = $body.IndexOf("HC074-ORIGINAL-BARRIER-PHASE name=$phase begin=true", $last)
            Require ($begin -ge $last) "missing/out-of-order $phase"
            $end = $body.IndexOf("HC074-ORIGINAL-BARRIER-PHASE name=$phase completed=true", $begin)
            Require ($end -gt $begin) "incomplete $phase"
            Require ($body.Substring($begin, $end-$begin).Contains('test result: ok. 1 passed; 0 failed;')) "no executed $phase test"
            $last = $end
        }
        $controls = [regex]::Matches($body, 'HC074-ORIGINAL-BARRIER-CONTROL seed=0x0740b018 sample=([0-2]) case=(normal|ack-lost) frame_ipc=34 barred_during_frame=true eof=true unchanged=true send=false')
        Require ($controls.Count -eq 6) 'healthy count'
        foreach ($sample in 0..2) { foreach ($case in 'normal','ack-lost') {
            Require (@($controls | Where-Object { $_.Groups[1].Value -eq "$sample" -and $_.Groups[2].Value -eq $case }).Count -eq 1) 'duplicate/missing healthy cell'
        } }
        $refusals = [regex]::Matches($body, 'HC074-ORIGINAL-BARRIER-REFUSAL seed=0x0740b118 case=([^ ]+) reason=([^\n]+?) sticky=true command=false retained=true unchanged=true send=false')
        Require ($refusals.Count -eq 9) 'refusal count'
        $reasons = @{ trust='Input(Invalid)'; output='Outputs(Invalid)'; writer='Invalid'; message='Binding'; alarm='Binding'; 'custody-flags'='Storage(Document)'; 'custody-object'='Storage(Document)'; 'sink-offset'='Binding'; 'final-expiry'='Original(Expired)' }
        foreach ($case in $reasons.Keys) {
            $found = @($refusals | Where-Object { $_.Groups[1].Value -eq $case })
            Require ($found.Count -eq 1) "missing refusal $case"
            $reason = $found[0].Groups[2].Value
            Require ($reason.Contains($reasons[$case]) -and !$reason.Contains('Time')) "wrong refusal $case"
        }
        $cuts = [regex]::Matches($body, 'HC074-ORIGINAL-BARRIER-SIGKILL seed=0x0740b218 repeat=([1-3]) cut=([^ ]+) frame_ipc=(0|34) parent_retained=(true|false) signal=9 unchanged=true completion=false send=false')
        Require ($cuts.Count -eq 12) 'SIGKILL count'
        foreach ($repeat in 1..3) { foreach ($cut in 'before-frame','after-frame','after-alias-close','after-eof') {
            $found = @($cuts | Where-Object { $_.Groups[1].Value -eq "$repeat" -and $_.Groups[2].Value -eq $cut })
            Require ($found.Count -eq 1) 'duplicate/missing cut'
            Require ($found[0].Groups[3].Value -eq $(if ($cut -eq 'before-frame') { '0' } else { '34' })) 'wrong cut/frame'
            Require ($found[0].Groups[4].Value -eq $(if ($cut -in 'before-frame','after-frame') { 'true' } else { 'false' })) 'wrong parent custody'
        } }
        Require ($body.Contains('HC074-ORIGINAL-BARRIER-SIGKILL completed=true cases=12 product=false')) 'incomplete cuts'
    }
    Require ($Log.Contains('HC074-ORIGINAL-BARRIER-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false')) 'missing final host guard'
}
$manifest = Text 'manifest.json' | ConvertFrom-Json
Require ($manifest.schema -eq 'hc074-original-barrier-validation-v1' -and $manifest.admission -eq $false) 'schema/admission'
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | Where-Object Name -ne 'manifest.json' | ForEach-Object { [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/') })
Require (@(Compare-Object @($manifest.files.path | Sort-Object) @($actual | Sort-Object)).Count -eq 0) 'file inventory drift'
foreach ($file in $manifest.files) {
    Require ((Get-FileHash -LiteralPath (Join-Path $packet $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -eq $file.sha256) "digest drift: $($file.path)"
}
Require ($manifest.source_files.Count -eq 6) 'source file count'
foreach ($file in $manifest.source_files) {
    Require ((HashText (GitText @('show', "$($manifest.source_commit):$($file.path)"))) -eq $file.sha256) "committed source drift: $($file.path)"
    Require ((HashText ([IO.File]::ReadAllText((Join-Path $root $file.path)).Replace("`r`n","`n"))) -eq $file.sha256) "current source drift: $($file.path)"
}
$patch = GitText (@('diff','--binary',$manifest.base_commit,$manifest.source_commit,'--') + @($manifest.source_files.path))
Require ($patch -eq (Text 'implementation.patch')) 'complete source patch differs'
$inputs = [regex]::Matches((Text 'build-inputs.log'), '(?m)^([0-9a-f]{64})  ([^\n]+)$')
Require ((Text 'build-inputs.log') -eq (Text 'build-inputs-after.log')) 'build inputs changed during tests'
Require ($inputs.Count -gt 100) 'incomplete raw build input inventory'
foreach ($entry in $inputs) {
    $path = $entry.Groups[2].Value
    $blob = GitText @('show', "$($manifest.source_commit):$path")
    $recorded = $entry.Groups[1].Value
    Require ($recorded -eq (HashText $blob) -or $recorded -eq (HashText ($blob.Replace("`n","`r`n")))) "raw build input differs: $path"
    Require ([IO.File]::ReadAllText((Join-Path $root $path)).Replace("`r`n","`n") -eq $blob) "current build input differs: $path"
}
$qualified = Join-Path $root 'docs/testing/performance/0.74/qualification-manifest.toml'
Require ((Get-FileHash -LiteralPath $qualified -Algorithm SHA256).Hash.ToLowerInvariant() -eq $manifest.qualification_sha256) 'frozen qualification drift'
foreach ($compiler in '194','188') {
    $hash = $manifest."rust${compiler}_binary_sha256"
    Require ((Text 'binaries.log').Contains("$hash  tests-$compiler")) 'binary provenance'
    Require ((Text 'screen.log').Contains("HC074-ORIGINAL-BARRIER-SCREEN compiler=$compiler completed=true")) 'missing executed screen'
    Require ((Text 'screen.log').Contains("$hash  tests-$compiler")) 'screen binary differs'
    Require ((Text 'host.log').Contains("$hash  tests-$compiler")) 'matrix binary differs'
    Require ((Text "$compiler-build.log").Contains('Executable unittests src/lib.rs')) 'build incomplete'
    Require ((Text "$compiler-green.log").Contains('Finished')) 'check/clippy incomplete'
}
foreach ($script in 'guard.sh','screen.sh','run.sh') {
    $hash = (Get-FileHash -LiteralPath (Join-Path $packet $script) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ((Text 'binaries.log').Contains("$hash  $script")) "executed script differs: $script"
}
Require ((Text 'static-final.log').Contains('test result: ok. 210 passed; 0 failed;')) 'static suite incomplete'
Require ((Text 'windows-check.log').Contains('Finished')) 'Windows check incomplete'
Require ((Text 'windows-clippy.log').Contains('Finished')) 'Windows strict Clippy incomplete'
Require ((Text 'toolchains.log').Contains('rustc 1.94.0') -and (Text 'toolchains.log').Contains('rustc 1.88.0')) 'compiler identity missing'
Require ((Text 'registry.log').Contains('OK')) 'registry incomplete'
Require ((Text 'doc.log').Contains('OK')) 'docs incomplete'
Require ((Text 'contract.log').Contains('OK')) 'local performance contract incomplete'
Require ((Text 'postflight.log').Contains('supervisor_pid=8839 restarts=0')) 'missing final preflight'
VerifyHost (Text 'host.log')
if ($SelfTest) {
    $log = Text 'host.log'
    $mutations = @(
        @('compiler=194 begin=true','compiler=188 begin=true'),
        @('frame_ipc=34 barred_during_frame=true','frame_ipc=0 barred_during_frame=true'),
        @('case=ack-lost frame_ipc=34','case=normal frame_ipc=34'),
        @('case=trust reason=','case=unknown reason='),
        @('command=false retained=true','command=true retained=true'),
        @('cut=before-frame frame_ipc=0','cut=before-frame frame_ipc=34'),
        @('signal=9 unchanged=true','signal=15 unchanged=true'),
        @('parent_retained=true','parent_retained=false'),
        @('cases=12 product=false','cases=11 product=false'),
        @('no_fixture_executable=true','no_fixture_executable=false')
    )
    foreach ($mutation in $mutations) {
        $changed = $log.Replace($mutation[0],$mutation[1])
        Require ($changed -ne $log) 'ineffective mutation'
        $rejected = $false
        try { VerifyHost $changed } catch { $rejected = $true }
        Require $rejected "accepted mutation: $($mutation[0])"
    }
    Write-Output "HC074-ORIGINAL-BARRIER verifier_selftest=true positive=1 negative=$($mutations.Count)"
}
Write-Output "HC074-ORIGINAL-BARRIER verified=true healthy=12 refusals=18 selected_sigkill=24 cleanup_sigkill=18 admission=false"
