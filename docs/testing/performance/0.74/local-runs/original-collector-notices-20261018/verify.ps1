param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
function Require([bool]$Condition,[string]$Reason) { if (-not $Condition) { throw $Reason } }
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
    $value = $process.StandardOutput.ReadToEnd()
    $process.WaitForExit()
    Require ($process.ExitCode -eq 0) 'git failed'
    $value
}
function CheckManifest($Manifest) {
    Require ($Manifest.schema -ceq 'hc074-original-collector-notices-v1') 'schema'
    Require ($Manifest.admission -ceq $false -and $Manifest.scope -ceq 'local-parser-units-only') 'scope'
    Require ($Manifest.source_files.Count -eq 4 -and $Manifest.build_inputs.Count -eq 321) 'source inventory'
    Require ($Manifest.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification'
}
function CheckUnits([string]$Raw) {
    Require ($Raw -cnotmatch 'FAILED|panicked') 'unit failure'
    Require ([regex]::Matches($Raw,'test result: ok. 8 passed;').Count -eq 2) 'unit cohorts'
    foreach ($compiler in @('194','188')) {
        Require ($Raw.Contains("HC074-COLLECTOR-NOTICES compiler=$compiler begin=true admission=false")) 'unit cohort begin'
        Require ($Raw.Contains("HC074-COLLECTOR-NOTICES compiler=$compiler completed=true admission=false")) 'unit cohort completion'
    }
}
function CheckContracts([string]$Raw) {
    Require ($Raw.Contains('test result: ok. 221 passed; 0 failed;')) 'static contracts'
}
$manifest = Get-Content (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
CheckManifest $manifest
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | ForEach-Object {
    [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
} | Where-Object {$_ -ne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($manifest.files.path | Sort-Object) -join "`n")) 'closed inventory'
foreach ($file in $manifest.files) {
    Require ($file.path -cnotmatch '(^/|\\|\.\.)') 'unsafe packet path'
    Require ((Get-FileHash -LiteralPath (Join-Path $packet $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) "file digest $($file.path)"
}
$changes = @((GitText @('diff','--name-only',$manifest.base_commit,$manifest.source_commit)).Trim() -split "`n")
Require ((($changes | Sort-Object) -join "`n") -ceq (($manifest.source_files.path | Sort-Object) -join "`n")) 'source paths'
foreach ($file in $manifest.source_files) {
    Require ((HashText (GitText @('show',"$($manifest.source_commit):$($file.path)"))) -ceq $file.sha256) 'source binding'
}
foreach ($file in $manifest.build_inputs) {
    Require ((HashText (GitText @('show',"$($manifest.source_commit):$($file.path)"))) -ceq $file.lf_sha256) 'input binding'
}
$rows = @($manifest.build_inputs | ForEach-Object {"$($_.raw_sha256)  $($_.path)"})
Require ([IO.File]::ReadAllText((Join-Path $packet 'build-inputs-final.log')) -ceq (($rows -join "`n")+"`n")) 'raw inputs'
Require ([IO.File]::ReadAllText((Join-Path $packet 'implementation.patch')) -ceq (GitText @('diff','--binary',$manifest.base_commit,$manifest.source_commit))) 'implementation patch'
$fast = [IO.File]::ReadAllText((Join-Path $packet 'fast.log'))
$contracts = [IO.File]::ReadAllText((Join-Path $packet 'contracts.log'))
CheckUnits $fast
CheckContracts $contracts
foreach ($compiler in @('194','188')) {
    $hash = (Get-Content (Join-Path $packet "binary-$compiler.sha256") -Raw).Trim()
    Require ($hash -cmatch '^[0-9a-f]{64}$' -and $fast.Contains($hash)) 'compiler image binding'
}
foreach ($name in @('check194','check188','clippy194','clippy188','windows-check','windows-clippy')) {
    $raw = [IO.File]::ReadAllText((Join-Path $packet "$name.log"))
    Require ($raw.Contains('Finished') -and $raw -cnotmatch 'error:') "check $name"
}
Require ((Get-FileHash -LiteralPath (Join-Path $root 'docs/testing/performance/0.74/qualification-manifest.toml') -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $manifest.qualification_sha256) 'qualification drift'
if ($SelfTest) {
    $negatives = 0
    foreach ($bad in @($fast.Replace('8 passed','7 passed'),$fast.Replace('compiler=188 completed=true','compiler=188 completed=false'),($fast+"`nFAILED"))) {
        try {CheckUnits $bad} catch {$negatives++; continue}; throw 'unit mutation escaped'
    }
    try {CheckContracts ($contracts.Replace('221 passed','220 passed'))} catch {$negatives++}
    Require ($negatives -eq 4) 'contract mutation escaped'
    foreach ($kind in @('admission','inputs')) {
        $bad = $manifest | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        switch ($kind) {admission {$bad.admission=$true} inputs {$bad.build_inputs=@($bad.build_inputs | Select-Object -Skip 1)}}
        try {CheckManifest $bad} catch {$negatives++; continue}; throw 'manifest mutation escaped'
    }
    "HC074-COLLECTOR-NOTICES-OFFLINE positive=1 negatives=$negatives admission=false"
} else {'HC074-COLLECTOR-NOTICES-OFFLINE verified=true admission=false'}
