param([switch]$SelfTest, [switch]$NegativeOnly)
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
    Require (!$Log.Contains('FAILED')) 'failed positive host test'
    $cohorts = [regex]::Matches($Log, '(?s)HC074-BARRED-CUSTODY-COHORT compiler=(194|188) begin=true(.*?)HC074-BARRED-CUSTODY-COHORT compiler=\1 completed=true')
    Require ($cohorts.Count -eq 2) 'cohort count'
    Require ($cohorts[0].Groups[1].Value -eq '194' -and $cohorts[1].Groups[1].Value -eq '188') 'cohort order'
    foreach ($cohort in $cohorts) {
        $body = $cohort.Groups[2].Value
        Require ($body.Contains('test result: ok. 5 passed; 0 failed;')) 'missing fast units'
        $last = 0
        foreach ($phase in 'healthy','refusals','process_matrix') {
            $begin = $body.IndexOf("HC074-BARRED-CUSTODY-PHASE name=$phase begin=true",$last)
            Require ($begin -ge $last) "missing/out-of-order $phase"
            $end = $body.IndexOf("HC074-BARRED-CUSTODY-PHASE name=$phase completed=true",$begin)
            Require ($end -gt $begin) "incomplete $phase"
            Require ($body.Substring($begin,$end-$begin).Contains('test result: ok. 1 passed; 0 failed;')) "unexecuted $phase"
            $last = $end
        }
        $controls = [regex]::Matches($body,'HC074-BARRED-CUSTODY-CONTROL sample=([0-2]) create_ipc=34 validate_ipc=34 inherited=3 mask_exact=true unchanged=true start=false')
        Require ($controls.Count -eq 3) 'healthy count'
        foreach ($sample in 0..2) { Require (@($controls | Where-Object { $_.Groups[1].Value -eq "$sample" }).Count -eq 1) 'missing/duplicate control' }
        $refusals = [regex]::Matches($body,'HC074-BARRED-CUSTODY-REFUSAL seed=0x0740bc18 case=([^ ]+) reason=([^\n]+?) mask_exact=true sticky=true unchanged=true start=false')
        Require ($refusals.Count -eq 8) 'refusal count'
        $reasons = @{ 'clone-error'='code: 11'; 'restore-error'='code: 5'; trust='Input(Invalid)'; output='Outputs(Invalid)'; message='Binding'; 'custody-flags'='Storage(Document)'; 'child-kill'='Child'; 'final-expiry'='Original(Original(Expired))' }
        foreach ($case in $reasons.Keys) {
            $found = @($refusals | Where-Object { $_.Groups[1].Value -eq $case })
            Require ($found.Count -eq 1) "missing/duplicate $case"
            $reason = $found[0].Groups[2].Value
            Require ($reason.Contains($reasons[$case]) -and !$reason.Contains('Time')) "wrong refusal $case"
        }
        $processes = [regex]::Matches($body,'HC074-BARRED-CUSTODY-PROCESS seed=0x0740bd18 repeat=([1-3]) case=(parent-alias-close|autonomous-expiry|peer-lost) inherited=3 retained=true signal=9 unchanged=true start=false')
        Require ($processes.Count -eq 9) 'process count'
        foreach ($repeat in 1..3) { foreach ($case in 'parent-alias-close','autonomous-expiry','peer-lost') {
            Require (@($processes | Where-Object { $_.Groups[1].Value -eq "$repeat" -and $_.Groups[2].Value -eq $case }).Count -eq 1) 'missing/duplicate process cell'
        } }
        Require ($body.Contains('HC074-BARRED-CUSTODY-PROCESS completed=true cases=9 product=false')) 'process summary'
    }
    Require ($Log.Contains('HC074-BARRED-CUSTODY-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false')) 'host final guard'
}
function VerifyNegative {
    $folder = Join-Path $packet 'negative-four-frame'
    $saved = [IO.File]::ReadAllText((Join-Path $folder 'source-manifest.json')) | ConvertFrom-Json
    Require ($saved.sources.Count -eq 9) 'negative capsule file count'
    Require ((Get-FileHash -LiteralPath (Join-Path $folder 'candidate.patch')).Hash.ToLowerInvariant() -eq $saved.patch_sha256) 'negative patch drift'
    $map = @{}
    foreach ($source in $saved.sources) {
        $file = Join-Path $folder $source.saved
        Require ((Get-FileHash -LiteralPath $file).Hash.ToLowerInvariant() -eq $source.sha256) "negative source drift: $($source.path)"
        $map[$source.path] = [IO.File]::ReadAllText($file)
    }
    $inputs = [regex]::Matches([IO.File]::ReadAllText((Join-Path $folder 'build-inputs.log')).Replace("`r`n","`n"),'(?m)^([0-9a-f]{64})  ([^\n]+)$')
    Require ($inputs.Count -gt 300) 'negative build input inventory'
    foreach ($entry in $inputs) {
        $path = $entry.Groups[2].Value
        $blob = if ($map.ContainsKey($path)) { $map[$path] } else { GitText @('show',"$($saved.base):$path") }
        $digest = $entry.Groups[1].Value
        Require ($digest -eq (HashText $blob) -or $digest -eq (HashText ($blob.Replace("`r`n","`n").Replace("`n","`r`n")))) "negative build source differs: $path"
    }
    $runtime = 'tools/long-run-supervisor-074/src/diagnostic_original_barred_custody.rs'
    $old = $map[$runtime].Replace("`r`n","`n")
    $now = [IO.File]::ReadAllText((Join-Path $root $runtime)).Replace("`r`n","`n")
    $marker = '/// Pending custody cannot detach, start, collect or send in an ordinary build.'
    Require ($old.Substring(0,$old.IndexOf($marker)) -eq $now.Substring(0,$now.IndexOf($marker))) 'kernel body changed with wrapper fix'
    Require ($old.Contains('self.plan.revalidate()?;') -and $now.Contains('self.original.validate(self.allow_fixture)?;')) 'negative/positive wrapper distinction'
    foreach ($compiler in '194','188') {
        $screen = [IO.File]::ReadAllText((Join-Path $folder "screen-$compiler.log"))
        Require ($screen.Contains('Causal(Time)') -and $screen.Contains('0 passed; 1 failed;')) "negative screen differs: $compiler"
        $hash = if ($compiler -eq '194') { '7a51b58d084d0fc4582b55c70d9c64fe37b7fd794852216aec815613835f090a' } else { 'f6c589649c923a2e91be5545d3c155baef1afccb72abd9c2dbea8557a28b6788' }
        Require ($screen.Contains("$hash  tests-$compiler")) 'negative binary provenance'
    }
    Require ([IO.File]::ReadAllText((Join-Path $folder 'postflight.log')).Contains('supervisor_pid=8839 restarts=0')) 'negative final host guard'
}
if ($NegativeOnly) {
    VerifyNegative
    Write-Output 'HC074-BARRED-CUSTODY negative_verified=true complete_sources=9 screens=2 kernel_unchanged=true'
    exit
}
$manifest = Text 'manifest.json' | ConvertFrom-Json
Require ($manifest.schema -eq 'hc074-original-barred-custody-v1' -and $manifest.admission -eq $false) 'schema/admission'
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | Where-Object Name -ne 'manifest.json' | ForEach-Object { [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/') })
Require (@(Compare-Object @($manifest.files.path | Sort-Object) @($actual | Sort-Object)).Count -eq 0) 'file inventory drift'
foreach ($file in $manifest.files) {
    Require ((Get-FileHash -LiteralPath (Join-Path $packet $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -eq $file.sha256) "digest drift: $($file.path)"
}
Require ($manifest.source_files.Count -eq 9) 'source file count'
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
    Require ((Text 'screen.log').Contains("HC074-BARRED-CUSTODY-SCREEN compiler=$compiler completed=true")) 'missing executed screen'
    Require ((Text 'screen.log').Contains("$hash  tests-$compiler")) 'screen binary differs'
    Require ((Text 'host.log').Contains("$hash  tests-$compiler")) 'matrix binary differs'
    Require ((Text "$compiler-build.log").Contains('Executable unittests src/lib.rs')) 'build incomplete'
    Require ((Text "$compiler-build.log").Contains('Finished')) 'check/clippy incomplete'
}
foreach ($script in 'guard.sh','screen.sh','run.sh') {
    $hash = (Get-FileHash -LiteralPath (Join-Path $packet $script) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ((Text 'binaries.log').Contains("$hash  $script")) "executed script differs: $script"
}
Require ((Text 'static-final.log').Contains('test result: ok. 213 passed; 0 failed;')) 'static suite incomplete'
Require ((Text 'windows-check.log').Contains('Finished')) 'Windows check incomplete'
Require ((Text 'windows-clippy.log').Contains('Finished')) 'Windows strict Clippy incomplete'
Require ((Text 'toolchains.log').Contains('rustc 1.94.0') -and (Text 'toolchains.log').Contains('rustc 1.88.0')) 'compiler identity missing'
Require ((Text 'registry.log').Contains('OK')) 'registry incomplete'
Require ((Text 'doc.log').Contains('OK')) 'docs incomplete'
Require ((Text 'contract.log').Contains('OK')) 'local performance contract incomplete'
Require ((Text 'postflight.log').Contains('supervisor_pid=8839 restarts=0')) 'missing final preflight'
Require ((Text 'postflight.log').Contains('old_fixture_absent=true new_fixture_absent=true supervisor_unchanged=true admission=false')) 'surviving old/new fixture'
$postflightHash = (Get-FileHash -LiteralPath (Join-Path $packet 'postflight.sh')).Hash.ToLowerInvariant()
Require ((Text 'postflight.log').Contains("$postflightHash  postflight.sh")) 'executed postflight differs'
VerifyNegative
VerifyHost (Text 'host.log')
if ($SelfTest) {
    $log = Text 'host.log'
    $mutations = @(
        @('compiler=194 begin=true','compiler=188 begin=true'),
        @('create_ipc=34 validate_ipc=34','create_ipc=17 validate_ipc=34'),
        @('inherited=3','inherited=6'),
        @('mask_exact=true','mask_exact=false'),
        @('case=trust reason=','case=unknown reason='),
        @('sticky=true unchanged=true','sticky=false unchanged=true'),
        @('case=autonomous-expiry','case=parent-alias-close'),
        @('signal=9 unchanged=true','signal=15 unchanged=true'),
        @('cases=9 product=false','cases=8 product=false'),
        @('no_fixture_executable=true','no_fixture_executable=false')
    )
    foreach ($mutation in $mutations) {
        $changed = $log.Replace($mutation[0],$mutation[1])
        Require ($changed -ne $log) 'ineffective mutation'
        $rejected = $false
        try { VerifyHost $changed } catch { $rejected = $true }
        Require $rejected "accepted mutation: $($mutation[0])"
    }
    Write-Output "HC074-BARRED-CUSTODY selftest=true positive=1 negative=10"
}
Write-Output "HC074-BARRED-CUSTODY verified=true healthy=6 refusals=16 selected_process_kills=18 negative_screens=2 admission=false"
