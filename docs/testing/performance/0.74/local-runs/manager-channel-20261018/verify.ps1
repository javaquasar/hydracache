param([switch]$SelfTest)
$ErrorActionPreference='Stop'
$packet=$PSScriptRoot
$root=(Resolve-Path (Join-Path $packet '../../../../../..')).Path
function Require([bool]$Condition,[string]$Reason) {if (-not $Condition) {throw $Reason}}
function HashText([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
function TextDigest([string]$Text,[string]$Expected) {Require ((HashText $Text) -ceq $Expected) 'text digest'}
function GitText([string]$Ref,[string]$Path) {
    $start=[Diagnostics.ProcessStartInfo]::new('git');$start.WorkingDirectory=$root;$start.UseShellExecute=$false
    $start.RedirectStandardOutput=$true;$start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
    $start.ArgumentList.Add('show');$start.ArgumentList.Add("${Ref}:$Path")
    $process=[Diagnostics.Process]::Start($start);$value=$process.StandardOutput.ReadToEnd();$process.WaitForExit()
    Require ($process.ExitCode -eq 0) 'git source unavailable';$value
}
function Count([string]$Raw,[string]$Needle) {[regex]::Matches($Raw,[regex]::Escape($Needle)).Count}
$falseFlags=@('admission','original_join','original_start','writer_cessation','server_install','product_workload','original_host_context')
function CheckManifest($m) {
    Require ($m.schema -ceq 'hc074-fixed-manager-channel-v1' -and $m.status -ceq 'local-read-only-producer' -and $m.selected_gate -ceq $true) 'schema/scope'
    foreach ($field in $falseFlags) {Require ($m.$field -is [bool] -and $m.$field -ceq $false) "false $field"}
    Require ($m.source_files.Count -eq 14 -and $m.build_inputs.Count -eq 333 -and $m.images.Count -eq 2) 'source/image inventory'
    Require ($m.source_commit -cmatch '^[a-f0-9]{40}$' -and $m.base_commit -ceq '271921396355b610d68f5346f94f538a64f38ce6' -and $m.first_base_commit -ceq 'ff4f3b7e99e3e52227864cc2344d9739c18539db') 'source identity'
    Require ($m.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification identity'
}
function CheckCohort([string]$Raw,$Images) {
    Require ($Raw -cnotmatch 'FAILED|panicked|Exception:') 'cohort failure'
    Require ((Count $Raw 'HC074-MANAGER-LOCAL completed=true server_ssh=false service_mutation=false original_join=false start=false') -eq 1) 'cohort completion'
    foreach ($entry in $Images) {
        $begin="HC074-MANAGER-COHORT compiler=$($entry.compiler) begin=true";$end="HC074-MANAGER-COHORT compiler=$($entry.compiler) completed=true admission=false"
        Require ((Count $Raw $begin) -eq 1 -and (Count $Raw $end) -eq 1) 'compiler cohort'
        $part=$Raw.Substring($Raw.IndexOf($begin),$Raw.IndexOf($end)-$Raw.IndexOf($begin))
        foreach ($n in @(9,14,8,32)) {Require ((Count $part "test result: ok. $n passed; 0 failed;") -eq 1) 'test inventory'}
        Require ((Count $part 'test result: ok. 1 passed; 0 failed; 0 ignored;') -eq 2) 'manual case inventory'
        Require ((Count $part 'HC074-MANAGER-CHANNEL nonroot=true namespace_refused=true original_join=false start=false') -eq 1) 'nonroot namespace refusal'
        Require ((Count $part 'HC074-MANAGER-CHANNEL root_read_only=true same_connection=true manager_pid=1 closed_sticky=true original_join=false start=false') -eq 1) 'root context/closed latch'
        Require ((Count $Raw "$($entry.sha256)  $($entry.image)") -eq 2) 'image binding'
    }
}
$m=Get-Content (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json;CheckManifest $m
$actual=@(Get-ChildItem -LiteralPath $packet -Recurse -File | ForEach-Object {[IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')} | Where-Object {$_ -cne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($m.files.path | Sort-Object) -join "`n")) 'closed file inventory'
foreach ($file in $m.files) {
    Require ($file.path -cnotmatch '(^/|\\|\.\.)') 'unsafe file path'
    Require ((Get-FileHash -LiteralPath (Join-Path $packet $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) "file digest $($file.path)"
}
$sources=@{}
foreach ($file in $m.source_files) {
    Require (-not $sources.ContainsKey($file.path)) 'duplicate source'
    $path=Join-Path $packet ('source-capsule/'+$file.path.Replace('/','__')+'.source')
    Require ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.raw_sha256) 'raw capsule'
    $lf=[IO.File]::ReadAllText($path).Replace("`r`n","`n");TextDigest $lf $file.lf_sha256
    Require ((GitText $m.source_commit $file.path).Replace("`r`n","`n") -ceq $lf) 'accepted source differs'
    $sources[$file.path]=$lf
}
$changes=@(git -C $root diff --name-only $m.base_commit $m.source_commit)
Require ($LASTEXITCODE -eq 0 -and (($changes | Sort-Object) -join "`n") -ceq (($m.source_files.path | Sort-Object) -join "`n")) 'changes outside capsule'
$parent=(git -C $root rev-parse "$($m.source_commit)^").Trim();Require ($LASTEXITCODE -eq 0 -and $parent -ceq $m.base_commit) 'source parent'
$inputNames=@{}
foreach ($file in $m.build_inputs) {
    Require (-not $inputNames.ContainsKey($file.path)) 'duplicate build input';$inputNames[$file.path]=$true
    $lf=if ($sources.ContainsKey($file.path)) {$sources[$file.path]} else {GitText $m.base_commit $file.path}
    TextDigest $lf $file.lf_sha256
}
$rows=@($m.build_inputs | ForEach-Object {"$($_.raw_sha256)  $($_.path)"})
Require ([IO.File]::ReadAllText((Join-Path $packet 'build-inputs.log')) -ceq (($rows -join "`n")+"`n")) 'raw build bindings'
Require ((($m.images.compiler | Sort-Object) -join ',') -ceq '188,194') 'compiler inventory'
foreach ($entry in $m.images) {
    Require ($entry.sha256 -cmatch '^[a-f0-9]{64}$' -and $entry.image -cmatch ('^/home/artur/\.cache/hydracache-074-'+$entry.compiler+'-check/release/deps/hydracache_long_run_supervisor_074-[a-f0-9]{16}$')) 'image identity'
    $artifacts=@(Get-Content (Join-Path $packet "build-$($entry.compiler).jsonl") | ForEach-Object {$_ | ConvertFrom-Json} | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
    Require ($artifacts.Count -eq 1 -and $artifacts[0].executable -ceq $entry.image) 'Cargo executable binding'
    $gnu=Get-Content (Join-Path $packet "gnu-$($entry.compiler).log") -Raw
    Require ($gnu.Contains('test result: ok. 2 passed; 0 failed;') -and $gnu.Contains("HC074-MANAGER-GNU compiler=$($entry.compiler) completed=true admission=false")) 'GNU checks/Clippy/doctests'
}
$cohort=Get-Content (Join-Path $packet 'local-cohort.log') -Raw;CheckCohort $cohort $m.images
Require ((Get-Content (Join-Path $packet 'contracts.log') -Raw).Contains('test result: ok. 233 passed; 0 failed;')) 'full source contracts'
Require ((Get-Content (Join-Path $packet 'final-source-contracts.log') -Raw).Contains('test result: ok. 4 passed; 0 failed;')) 'final canaries'
$local=Get-Content (Join-Path $packet 'local-checks.log') -Raw
Require ($local.Contains('test result: ok. 44 passed;') -and $local.Contains('HC074-MANAGER-CHECKS completed=true admission=false')) 'local checks'
$docs=Get-Content (Join-Path $packet 'documentation-final.log') -Raw
Require ($docs.Contains('doc-check: OK') -and $docs.Contains('performance-contract-check 0.74: OK (local, non-promotable)')) 'final docs/local contract'
$first=Get-Content (Join-Path $packet 'first-attempt/local-cohort.log') -Raw
Require ($first.Contains('test result: FAILED. 0 passed; 1 failed;') -and $first.Contains('Err` value: Reply') -and -not $first.Contains('compiler=188 begin=true') -and -not $first.Contains('completed=true admission=false')) 'stopped first cohort'
Require ((Get-Content (Join-Path $packet 'first-attempt/contracts.log') -Raw).Contains('230 passed; 3 failed;')) 'three legacy edge refusals'
$trace=Get-Content (Join-Path $packet 'attribution.log') -Raw
Require ($trace.Contains('returned=true method_return=true flags=1 sender_bus=true destination_client=true serial=true error=false path=false interface=false member_header=false fd_count=0 unix_fds=0 signature_expected=true bounded=true') -and $trace.Contains('expected_exit=101 replacement_gate=false start=false')) 'reply attribution'
$firstSources=@{}
foreach ($file in Get-ChildItem (Join-Path $packet 'first-attempt/source-capsule') -File) {$firstSources[$file.Name.Substring(0,$file.Name.Length-7).Replace('__','/')]=[IO.File]::ReadAllText($file.FullName).Replace("`r`n","`n")}
Require ($firstSources.Count -eq 11) 'first source inventory'
$firstInputs=@(Get-Content (Join-Path $packet 'first-attempt/build-bindings.json') -Raw | ConvertFrom-Json);Require ($firstInputs.Count -eq 333) 'first inputs'
foreach ($file in $firstInputs) {$lf=if ($firstSources.ContainsKey($file.path)) {$firstSources[$file.path]} else {GitText $m.first_base_commit $file.path};TextDigest $lf $file.lf_sha256}
Require ((Get-Content (Join-Path $packet 'missing-api.log') -Raw).Contains('error[E0425]')) 'missing API proof'
if ($SelfTest) {
    $negatives=0
    foreach ($field in $falseFlags) {$bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json;$bad.$field=$true;$rejected=$false;try {CheckManifest $bad} catch {$rejected=$true};Require $rejected "negative $field";$negatives++}
    $bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json;$bad.admission='False';$rejected=$false;try {CheckManifest $bad} catch {$rejected=$true};Require $rejected 'string boolean';$negatives++
    $bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json;$bad.build_inputs=@($bad.build_inputs | Select-Object -Skip 1);$rejected=$false;try {CheckManifest $bad} catch {$rejected=$true};Require $rejected 'missing input';$negatives++
    $rejected=$false;try {CheckCohort ($cohort.Replace('HC074-MANAGER-CHANNEL root_read_only=true same_connection=true manager_pid=1 closed_sticky=true original_join=false start=false','removed')) $m.images} catch {$rejected=$true};Require $rejected 'missing root case';$negatives++
    $path='tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs';$rejected=$false
    try {TextDigest ($sources[$path]+'mutation') ($m.source_files | Where-Object {$_.path -ceq $path}).lf_sha256} catch {$rejected=$true};Require $rejected 'changed source';$negatives++
    "HC074-MANAGER-OFFLINE negative_controls=$negatives rejected=true"
}
'HC074-MANAGER-OFFLINE verified=true read_only=true original_join=false admission=false'
