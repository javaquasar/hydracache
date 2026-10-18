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
    Require ($process.ExitCode -eq 0) 'git source unavailable';$value
}
$falseFlags=@('admission','original_start','writer_cessation','server_install','product_workload')
function CheckManifest($m) {
    Require ($m.schema -ceq 'hc074-exchange-reply-flags-v1') 'schema'
    foreach ($flag in $falseFlags) {Require ($m.$flag -is [bool] -and $m.$flag -ceq $false) "scope $flag"}
    Require ($m.source_commit -cmatch '^[0-9a-f]{40}$' -and $m.base_commit -ceq 'ff665c3b765ce0db61d54eb28d8e9e99a7d9b78e') 'source identity'
    Require ($m.source_files.Count -eq 6 -and $m.build_inputs.Count -eq 334 -and $m.images.Count -eq 2) 'source/input/image count'
    Require ((($m.images.compiler | Sort-Object) -join ',') -ceq '188,194') 'distinct compiler cohorts'
    Require ($m.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification identity'
}
function CheckPass([string]$Raw,[int]$Count) {
    Require ($Raw -cnotmatch 'FAILED|panicked|Exception:') 'passing log failure'
    Require ($Raw.Contains("test result: ok. $Count passed; 0 failed;")) 'passing log count'
}
$m=Get-Content (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json
CheckManifest $m
$actual=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | ForEach-Object {[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/')} | Where-Object {$_ -cne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($m.files.path | Sort-Object) -join "`n")) 'closed inventory'
foreach ($entry in $m.files) {
    Require ($entry.path -cnotmatch '(^/|\\|\.\.)') 'unsafe path'
    Require ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $entry.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $entry.sha256) "file hash $($entry.path)"
}
foreach ($entry in $m.source_files) {
    $path=Join-Path $PSScriptRoot ('source-capsule/'+$entry.path.Replace('/','__')+'.source')
    Require ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $entry.raw_sha256) 'raw source'
    $lf=[IO.File]::ReadAllText($path).Replace("`r`n","`n")
    Require ((HashText $lf) -ceq $entry.lf_sha256 -and (GitText $m.source_commit $entry.path).Replace("`r`n","`n") -ceq $lf) 'source binding'
}
$changes=@(git -C $root diff --name-only $m.base_commit $m.source_commit)
Require ($LASTEXITCODE -eq 0 -and (($changes | Sort-Object) -join "`n") -ceq (($m.source_files.path | Sort-Object) -join "`n")) 'source changes'
$parent=(git -C $root rev-parse "$($m.source_commit)^").Trim()
Require ($LASTEXITCODE -eq 0 -and $parent -ceq $m.base_commit) 'source parent'
foreach ($entry in $m.build_inputs) {
    Require ((HashText ((GitText $m.source_commit $entry.path).Replace("`r`n","`n"))) -ceq $entry.lf_sha256) 'build binding'
}
$expectedInputs=@(git -C $root ls-tree -r --name-only $m.source_commit -- tools/long-run-supervisor-074/src | Where-Object {$_ -cmatch '^tools/long-run-supervisor-074/src/[^/]+\.rs$'})
Require ($LASTEXITCODE -eq 0) 'source inventory unavailable'
$expectedInputs+=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
Require ((($expectedInputs | Sort-Object) -join "`n") -ceq (($m.build_inputs.path | Sort-Object) -join "`n")) 'complete build inventory'
foreach ($entry in $m.images) {
    Require ($entry.compiler -cin @('194','188') -and $entry.sha256 -cmatch '^[a-f0-9]{64}$') 'image identity'
    $log=Get-Content (Join-Path $PSScriptRoot "Candidate-$($entry.compiler).log") -Raw
    CheckPass $log 19
    $build=Get-Content (Join-Path $PSScriptRoot "Candidate-build-$($entry.compiler).jsonl") -Raw
    Require ($build.Contains($entry.image)) 'image artifact'
    CheckPass (Get-Content (Join-Path $PSScriptRoot "Baseline-$($entry.compiler).log") -Raw) 14
    $checks=Get-Content (Join-Path $PSScriptRoot "gnu-$($entry.compiler).log") -Raw
    Require ($checks.Contains("HC074-EXCHANGE-GNU compiler=$($entry.compiler) completed=true admission=false")) 'GNU checks'
    $regression=Get-Content (Join-Path $PSScriptRoot "regression-$($entry.compiler).log") -Raw
    foreach ($n in @(9,17,32)) {CheckPass $regression $n}
}
$red=Get-Content (Join-Path $PSScriptRoot 'Red-194.log') -Raw
Require ($red.Contains('bits=1 error=Reply') -and $red.Contains('test result: FAILED. 0 passed; 1 failed;')) 'retained red'
$first=Get-Content (Join-Path $PSScriptRoot 'first-source-contracts.log') -Raw
Require ($first.Contains('233 passed; 2 failed;')) 'retained first source comparison'
CheckPass (Get-Content (Join-Path $PSScriptRoot 'source-contracts.log') -Raw) 235
CheckPass (Get-Content (Join-Path $PSScriptRoot 'windows-tests.log') -Raw) 44
$checks=Get-Content (Join-Path $PSScriptRoot 'final-checks.log') -Raw
Require ($checks.Contains('HC074-EXCHANGE-CHECKS completed=true admission=false')) 'final checks'
if ($SelfTest) {
    $count=0
    foreach ($flag in $falseFlags) {
        $bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json;$bad.$flag=$true
        $rejected=$false;try {CheckManifest $bad} catch {$rejected=$true};Require $rejected "negative $flag";$count++
    }
    $bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json;$bad.build_inputs=@($bad.build_inputs | Select-Object -Skip 1)
    $rejected=$false;try {CheckManifest $bad} catch {$rejected=$true};Require $rejected 'missing input';$count++
    $rejected=$false;try {CheckPass $red 19} catch {$rejected=$true};Require $rejected 'red promoted';$count++
    "HC074-EXCHANGE-OFFLINE negative_controls=$count rejected=true"
}
'HC074-EXCHANGE-OFFLINE verified=true read_only=true admission=false'
