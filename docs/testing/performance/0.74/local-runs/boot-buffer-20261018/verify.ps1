param([switch]$SelfTest)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
function Require([bool]$Condition,[string]$Message) {if (!$Condition) {throw $Message}}
function Hash([string]$Path) {(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
. (Join-Path $PSScriptRoot 'rows.ps1')
$m=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json -AsHashtable
Require ($m.schema -ceq 'hc074-boot-buffer-local-v1' -and $m.status -ceq 'experimental-local-only') 'manifest contract'
foreach ($flag in @('runtime_accepted','host_executed','full_preparation_passed','timing_claim','product_workload')) {Require ($m[$flag] -ceq $false) "unexpected claim $flag"}
Require ($m.baseline -ceq '3b8e73feec8412e4090bf05575f06a05901e1010') 'baseline pin'
Require ($m.approved_preparation_source -ceq '2ed04dc866b39fefea8d81be2328f7f1382e3d46') 'approved source changed'
$files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -File | Where-Object {$_.Name -cne 'manifest.json'} | ForEach-Object {[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/')} | Sort-Object)
Require (($files -join "`n") -ceq (($m.files.path | Sort-Object) -join "`n")) 'closed inventory'
foreach ($file in $m.files) {Require ((Hash (Join-Path $PSScriptRoot $file.path)) -ceq $file.sha256) "file hash $($file.path)"}
$binding=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'source-binding.json') -Raw | ConvertFrom-Json -AsHashtable
Require ($binding.source_commit -ceq $m.source_commit -and $m.source_commit -cmatch '^[a-f0-9]{40}$') 'source identity'
Require ($binding.sources.Count -eq 7 -and @($binding.sources.path | Select-Object -Unique).Count -eq 7) 'source slice census'
foreach ($source in $binding.sources) {
    $old=(& git -C $repo show "$($m.source_commit):$($source.path)") -join "`n"
    Require ($LASTEXITCODE -eq 0) 'source Git lookup'
    $capsule=[IO.File]::ReadAllText((Join-Path $PSScriptRoot $source.capsule)).Replace("`r",'').TrimEnd("`n")
    Require ($capsule -ceq $old.TrimEnd("`n")) "source binding $($source.path)"
}
$obs=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'observations.json') -Raw | ConvertFrom-Json -AsHashtable
Require (($obs.lane -join ',') -ceq 'windows,194,188') 'lane inventory'
Require (($binding.allocation_images.lane -join ',') -ceq 'windows,194,188') 'image inventory'
foreach ($lane in $obs) {
    $log=if ($lane.lane -ceq 'windows') {'windows-allocation.log'} else {"gnu-$($lane.lane)-allocation.log"}
    $raw=[IO.File]::ReadAllText((Join-Path $PSScriptRoot "checks/$log"))
    $rows=Read-AllocationRows $raw
    Require ($lane.whole_worker -ceq $false -and $lane.timing_claim -ceq $false -and $lane.rows.Count -eq 5) 'observation claim'
    for ($i=0; $i -lt 5; $i++) {
        foreach ($key in $rows[$i].Keys) {Require ($lane.rows[$i][$key] -ceq $rows[$i][$key]) "observation field $key"}
    }
    $image=@($binding.allocation_images | Where-Object {$_.lane -ceq $lane.lane})
    Require ($image.Count -eq 1 -and $image[0].sha256 -cmatch '^[a-f0-9]{64}$' -and $image[0].fresh_after_execution -ceq $true) 'image identity'
    $metadata=Get-Content -LiteralPath (Join-Path $PSScriptRoot "image-$($lane.lane).jsonl") | ForEach-Object {$_ | ConvertFrom-Json}
    $selected=@($metadata | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'diagnostic_boot_buffer_allocations' -and $_.profile.test -ceq $true -and $_.executable})
    Require ($selected.Count -eq 1 -and $selected[0].fresh -ceq $true -and $selected[0].executable -ceq $image[0].path) 'image artifact metadata'
    $normalized=$raw.Replace('\','/')
    $imageName=[IO.Path]::GetFileName($image[0].path.Replace('\','/'))
    Require ($normalized.Contains("/$imageName)")) 'executed allocation image name'
}
foreach ($lane in @('194','188')) {
    $raw=[IO.File]::ReadAllText((Join-Path $PSScriptRoot "checks/gnu-$lane-boot.log"))
    Require ($raw.Contains('test result: ok. 7 passed; 0 failed; 0 ignored;')) 'boot tests'
    Require ($raw.Contains('seed=0x0740b038 generated=1024 mutations=9472 single_read=true')) 'seeded inventory'
    $image=[IO.File]::ReadAllText((Join-Path $PSScriptRoot "checks/gnu-$lane-image.log")).Trim()
    Require ($image -cmatch '^[a-f0-9]{64}  /home/artur/\.cache/hydracache-074-(194|188)-check/release/deps/hydracache_long_run_supervisor_074-[a-f0-9]{16}$') 'image receipt'
    foreach ($filter in @('kernel_walk::tests::','live_walk::tests::','validation_trace_tests::','manager_channel::','dispatch_exchange::','original_pipe_message_','worker_context_','diagnostic_kernel_admission::tests::','diagnostic_kernel_admission::origin::tests::')) {
        $name=$filter.Replace(':','').Replace('_','-')
        $unit=Get-Content -LiteralPath (Join-Path $PSScriptRoot "checks/gnu-$lane-$name.log") -Raw
        Require ($unit -cmatch 'test result: ok\. [1-9][0-9]* passed; 0 failed;') "focused unit group $lane $filter"
    }
}
$contracts=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'checks/contracts.log') -Raw
Require ($contracts.Contains('test result: ok. 249 passed; 0 failed;')) 'source contracts'
$windows=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'checks/windows-tests.log') -Raw
Require ($windows.Contains('test result: ok. 44 passed; 0 failed;')) 'portable tests'
foreach ($path in @('windows-check','windows-clippy','gnu-194-check','gnu-194-clippy','gnu-188-check','gnu-188-clippy')) {
    $raw=Get-Content -LiteralPath (Join-Path $PSScriptRoot "checks/$path.log") -Raw
    Require ($raw -cmatch 'Finished `dev` profile') "check receipt $path"
}
foreach ($gate in @(
    @('registry','gated-test-check: OK (184 discovered gates, all registered)'),
    @('docs-final','doc-check: OK (releases.toml is consistent)'),
    @('performance-contract','performance-contract-check 0.74: OK (local, non-promotable)')
)) {
    $raw=Get-Content -LiteralPath (Join-Path $PSScriptRoot "checks/$($gate[0]).log") -Raw
    Require ($raw.Contains($gate[1])) "local gate $($gate[0])"
}
$red=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'red-behavior.log') -Raw
Require ($red.Contains('2 passed; 2 failed;') -and $red.Contains('left: 65537') -and $red.Contains('candidate_allocations=32')) 'behavioral red control'
$compile=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'red.log') -Raw
Require ($compile.Contains('error[E0308]')) 'initial harness error retained'
Require ($m.qualification_manifest_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification pin'
Require ((Hash (Join-Path $repo 'docs/testing/performance/0.74/qualification-manifest.toml')) -ceq $m.qualification_manifest_sha256) 'qualification changed'
if ($SelfTest) {
    $valid=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'checks/windows-allocation.log') -Raw
    $bad=@(
      $valid.Replace('candidate_allocations=0','candidate_allocations=1'),
      $valid.Replace('baseline_requested=2097184','baseline_requested=2097183'),
      $valid.Replace('scope=4','scope=3'),
      $valid.Replace('scope=4','scope=5'),
      $valid.Replace('whole_worker=false','whole_worker=true'),
      $valid.Replace('reads=32','reads=31'),
      $valid.Replace('candidate_deallocations=0','candidate_deallocations=1'),
      $valid.Replace('test result: ok.','test result: FAILED.'),
      $valid.Replace('HC074-BOOT-ALLOCATION scope=2','junk HC074-BOOT-ALLOCATION scope=2')
    )
    foreach ($raw in $bad) {
        $refused=$false
        try {Read-AllocationRows $raw | Out-Null} catch {$refused=$true}
        Require $refused 'negative parser control accepted'
    }
    'HC074-BOOT-VERIFY negative_parser_controls=9 refused=true'
}
'HC074-BOOT-VERIFY source_slice=7 allocation_scopes=15 contracts=249 runtime_accepted=false host_executed=false'
