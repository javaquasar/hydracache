param([switch]$SelfTest)
$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
function Require([bool]$Value,[string]$Reason) {if (!$Value) {throw $Reason}}
function HashText([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
function GitText([string]$Ref,[string]$Path) {
    $start=[Diagnostics.ProcessStartInfo]::new('git');$start.WorkingDirectory=$root;$start.UseShellExecute=$false
    $start.RedirectStandardOutput=$true;$start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
    $start.ArgumentList.Add('show');$start.ArgumentList.Add("${Ref}:$Path")
    $process=[Diagnostics.Process]::Start($start);$value=$process.StandardOutput.ReadToEnd();$process.WaitForExit()
    Require ($process.ExitCode -eq 0) 'git source unavailable';$value.Replace("`r`n","`n")
}
function Raw([string]$Name) {[IO.File]::ReadAllText((Join-Path $PSScriptRoot $Name))}
$falseFlags=@('admission','original_start','writer_cessation','server_install','supervisor_changed','product_workload','local_fixture_cohort_pass','positive_production_join')
function CheckManifest($m) {
    Require ($m.schema -ceq 'hc074-original-manager-join-v1') 'schema'
    foreach ($flag in $falseFlags) {Require ($m.$flag -is [bool] -and $m.$flag -ceq $false) "scope $flag"}
    Require ($m.host_fixture_cohort_pass -is [bool] -and $m.host_fixture_cohort_pass -ceq $true) 'host scope'
    Require ($m.source_commit -cmatch '^[0-9a-f]{40}$' -and $m.base_commit -ceq '3e7a70a83c478da6869701d5ce78adda8a09ab16') 'source identity'
    Require ($m.source_files.Count -eq 18 -and $m.build_inputs.Count -eq 340 -and $m.images.Count -eq 2 -and $m.first_images.Count -eq 2) 'source/input/image count'
    Require ((($m.images.compiler | Sort-Object) -join ',') -ceq '188,194') 'compiler cohorts'
    Require ($m.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification identity'
}
function CheckPass([string]$Log,[int]$Count) {
    Require ($Log -cnotmatch 'FAILED|panicked|Exception:|error:') 'passing log failure'
    Require ($Log.Contains("test result: ok. $Count passed; 0 failed;")) 'passing log count'
}
function CheckHost([string]$Log) {
    CheckPass $Log 1
    Require ([regex]::Matches($Log,'HC074-ORIGINAL-MANAGER-CONTROL sample=').Count -eq 6) 'host controls'
    Require ([regex]::Matches($Log,'HC074-ORIGINAL-MANAGER-REFUSAL case=').Count -eq 20) 'host refusals'
    foreach ($case in @('channel-before','channel-after','context-before','context-after','trust','message','output','collector-kill','custodian-kill','final-expiry')) {
        Require ([regex]::Matches($Log,"HC074-ORIGINAL-MANAGER-REFUSAL case=$case ").Count -eq 2) "missing host $case"
    }
    Require ([regex]::Matches($Log,'fixed_refused=true original_owner_retained=true').Count -eq 2) 'host fixed refusal'
    foreach ($compiler in @('194','188')) {Require ($Log.Contains("HC074-ORIGINAL-MANAGER-HOST compiler=$compiler completed=true start=false")) 'host compiler'}
    Require ($Log.Contains('HC074-ORIGINAL-MANAGER-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false')) 'host final guard'
}
$m=Raw 'manifest.json' | ConvertFrom-Json
CheckManifest $m
$actual=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | ForEach-Object {[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/')} | Where-Object {$_ -cne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($m.files.path | Sort-Object) -join "`n")) 'closed inventory'
foreach ($entry in $m.files) {
    Require ($entry.path -cnotmatch '(^/|\\|\.\.)') 'unsafe path'
    Require ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $entry.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $entry.sha256) "file hash $($entry.path)"
}
foreach ($entry in $m.source_files) {
    $capsule='source-capsule/'+$entry.path.Replace('/','__')+'.source'
    Require ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $capsule) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $entry.raw_sha256) 'raw source'
    $lf=(Raw $capsule).Replace("`r`n","`n")
    Require ((HashText $lf) -ceq $entry.lf_sha256 -and (GitText $m.source_commit $entry.path) -ceq $lf) 'source binding'
}
$changes=@(git -C $root diff --name-only $m.base_commit $m.source_commit)
Require ($LASTEXITCODE -eq 0 -and (($changes | Sort-Object) -join "`n") -ceq (($m.source_files.path | Sort-Object) -join "`n")) 'source changes'
$parent=(git -C $root rev-parse "$($m.source_commit)^").Trim()
Require ($LASTEXITCODE -eq 0 -and $parent -ceq $m.base_commit) 'source parent'
foreach ($entry in $m.build_inputs) {Require ((HashText (GitText $m.source_commit $entry.path)) -ceq $entry.lf_sha256) 'build binding'}
$expectedInputs=@(git -C $root ls-tree -r --name-only $m.source_commit -- tools/long-run-supervisor-074/src | Where-Object {$_ -cmatch '^tools/long-run-supervisor-074/src/[^/]+\.rs$'})
Require ($LASTEXITCODE -eq 0) 'source inventory unavailable'
$expectedInputs+=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
Require ((($expectedInputs | Sort-Object) -join "`n") -ceq (($m.build_inputs.path | Sort-Object) -join "`n")) 'complete build inventory'
$first=(Raw 'second-host/frame-tests.source').Replace("`r`n","`n")
# Restrict the true/false edit to the nonroot test, not the healthy controls.
$split='fn original_manager_join_nonroot_fixed_refusal()'
$oldParts=$first.Split($split);Require ($oldParts.Count -eq 2) 'first source split'
$corrected=$oldParts[0]+$split+$oldParts[1].Replace('with_original_at(false, None, |original, _paths, receiver, sender, model| {','with_original_at(true, None, |original, _paths, receiver, sender, model| {').Replace('format!("{:?}", failure)','format!("{failure:?}")')
Require ($corrected -ceq (GitText $m.source_commit 'tools/long-run-supervisor-074/src/diagnostic_original_manager_frame_tests.rs')) 'only two test corrections'
foreach ($entry in $m.images) {
    Require ($entry.compiler -cin @('194','188') -and $entry.sha256 -cmatch '^[a-f0-9]{64}$') 'image identity'
    Require ((Raw "Candidate-build-$($entry.compiler).jsonl").Contains($entry.image)) 'compiler artifact'
    $units=Raw "Candidate-$($entry.compiler).log"
    foreach ($n in @(1,3,10,19)) {CheckPass $units $n}
    $baseline=Raw "Baseline-$($entry.compiler).log"
    foreach ($n in @(9,19)) {CheckPass $baseline $n}
    CheckPass (Raw "readonly-$($entry.compiler).log") 1
    Require ((Raw "readonly-$($entry.compiler).log").Contains("image=$($entry.sha256) compiler=$($entry.compiler)") -and (Raw "readonly-$($entry.compiler).log").Contains('image_unchanged=true')) 'read-only image binding'
    Require ((Raw "readonly-$($entry.compiler).log").Contains('full_admission=false start=false')) 'read-only scope'
    Require ((Raw "gnu-$($entry.compiler).log").Contains("HC074-ORIGINAL-MANAGER-GNU compiler=$($entry.compiler) complete=true admission=false")) 'GNU checks'
    Require ((Raw 'host.log').Contains($entry.sha256+"  ./tests-$($entry.compiler)")) 'host image binding'
    $old=@($m.first_images | Where-Object {$_.compiler -ceq $entry.compiler})
    Require ($old.Count -eq 1 -and $old[0].sha256 -cne $entry.sha256) 'distinct first images'
    Require ((Raw 'second-host/host.log').Contains($old[0].sha256+"  ./tests-$($entry.compiler)")) 'first host image binding'
}
Require ((Raw 'Red-build-194.jsonl').Contains('E0599') -and (Raw 'Red-build-194.jsonl').Contains('bind_fixed_manager')) 'retained expected red'
Require ((Raw 'fixtures-194.log').Contains('Original(Challenge(Time))') -and (Raw 'fixtures-194.log').Contains('FAILED')) 'stopped local retained'
Require (!(Test-Path -LiteralPath (Join-Path $PSScriptRoot 'fixtures-188.log'))) 'local cohort must remain stopped'
Require ((Raw 'first-host/host.log').Contains('FAILED')) 'first host retained'
Require ((Raw 'host-umask-attribution.log').Contains('775:1000:1000')) 'umask attribution'
Require ((Raw 'second-host/host.log').Contains('left: true') -and (Raw 'second-host/host.log').Contains('right: false')) 'second host retained'
Require ((Raw 'first-contracts.log').Contains('236 passed; 2 failed;')) 'first contracts retained'
CheckHost (Raw 'host.log')
Require ((Raw 'host-postflight.log').Contains('preflight=ok supervisor_pid=8839 restarts=0')) 'postflight'
CheckPass (Raw 'contracts.log') 238
CheckPass (Raw 'windows-tests.log') 44
Require ((Raw 'registry.log').Contains('184 discovered gates, all registered')) 'registry discovery'
$registry=GitText $m.source_commit 'docs/testing/gated-test-registry.toml'
Require ([regex]::Matches($registry,'(?m)^\[\[gate\]\]$').Count -eq 312) 'registry entries'
foreach ($id in @('frame-controls','frame-refusals','nonroot-fixed-refusal','root-readonly-context')) {
    Require ($registry.Contains('id = "tool.hydracache-long-run-supervisor-074.original-manager-'+$id+'"')) 'explicit manual registration'
}
Require ((Raw 'final-checks.log').Contains('HC074-ORIGINAL-MANAGER-CHECKS complete=true admission=false')) 'step checks'
if ($SelfTest) {
    $count=0
    foreach ($flag in $falseFlags) {
        $bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json;$bad.$flag=$true
        $rejected=$false;try {CheckManifest $bad} catch {$rejected=$true};Require $rejected "negative $flag";$count++
    }
    $bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json;$bad.build_inputs=@($bad.build_inputs | Select-Object -Skip 1)
    $rejected=$false;try {CheckManifest $bad} catch {$rejected=$true};Require $rejected 'missing input';$count++
    foreach ($name in @('fixtures-194.log','second-host/host.log','first-host/host.log')) {
        $rejected=$false;try {CheckHost (Raw $name)} catch {$rejected=$true};Require $rejected "negative promoted $name";$count++
    }
    "HC074-ORIGINAL-MANAGER-OFFLINE negative_controls=$count rejected=true"
}
'HC074-ORIGINAL-MANAGER-OFFLINE verified=true read_only=true admission=false'
