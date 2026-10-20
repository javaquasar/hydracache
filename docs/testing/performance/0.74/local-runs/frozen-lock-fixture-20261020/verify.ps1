param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
$taskInventory=Join-Path $PSScriptRoot 'inventory.json'
function Require([bool]$Value,[string]$Reason){if(-not $Value){throw $Reason}}
function Json([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path) | ConvertFrom-Json}
function Raw([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path)}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
$taskClosed=@('runtime_changed','production_pin_changed','workspace_lock_changed','new_live_matrix','new_fault_cohort','clock_changed','fixed_root_admitted','artifact_objects_admitted','held_unit_projected','original_send','barrier_release','completion','fence_release','server','installation','supervisor','product','qualification','performance_claim','previous_321_suite_rerun','whole_linux_package_passed')
function Metadata($Value){
    Require ($Value.source -ceq '84fee2cce1d139d4361ae5cccc09d147eba357ab' -and $Value.baseline -ceq '23c57b77523ad1cc16b1a79fc90b02a125f8d9b8') 'Source mismatch'
    Require ($Value.scoped_checks_passed -ceq $true -and $Value.windows_package_passed -ceq $true) 'Check result mismatch'
    Require ($Value.runtime_sources_unchanged -ceq 437 -and $Value.scope_guards -ceq 4) 'Source scope mismatch'
    Require ($Value.windows_ordinary_tests -ceq 296 -and $Value.windows_doc_tests -ceq 20 -and $Value.windows_ignored -ceq 0) 'Windows scope mismatch'
    Require ($Value.linux_artifact_tests_per_compiler -ceq 31 -and $Value.linux_receipt_tests_per_compiler -ceq 9 -and $Value.seeded_corruptions_per_compiler -ceq 32) 'Linux selection mismatch'
    foreach($field in $taskClosed){Require ($Value.$field -ceq $false) "Forbidden promotion: $field"}
}
function CheckRaw {
    $value=Json 'metadata.json'
    Metadata $value
    Require ((Raw 'baseline-red.log').Contains('test result: FAILED. 0 passed; 1 failed;') -and (Json 'baseline.json').expected_red -ceq $true) 'Lost first baseline red'
    Require ((Raw 'new-regressions-red.log').Contains('test result: FAILED. 0 passed; 2 failed;')) 'Lost regression red'
    Require ((Raw 'windows-package.log').Contains('test result: FAILED. 0 passed; 8 failed;') -and (Json 'windows-package.json').all_ordinary_package_tests_passed -ceq $false) 'Lost second consumer red'
    Require ((Json 'receipts-baseline.json').baseline_test_diff_exit -ceq 0 -and (Json 'receipts-baseline.json').expected_red -ceq $true) 'Second baseline mismatch'
    Require ((Raw 'source-scope-both-consumers.log').Contains('test result: ok. 4 passed; 0 failed;')) 'Scope checks incomplete'
    $frozen=Json 'frozen-inputs.json'
    Require ($frozen.source -ceq $value.source -and $frozen.frozen_inputs_diff_exit -ceq 0 -and $frozen.runtime_sources -ceq 437 -and $frozen.fixture_bytes -ceq 175300 -and $frozen.fixture_sha256 -ceq 'be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19') 'Frozen inputs mismatch'
    $repository=Join-Path $PSScriptRoot '../../../../../..'
    Require ((Hash (Join-Path $repository 'tools/long-run-supervisor-074/tests/fixtures/diagnostic-pilot-root-Cargo.lock')) -ceq $frozen.fixture_sha256) 'Fixture bytes changed'
    Require ((Hash (Join-Path $PSScriptRoot '../../qualification-manifest.toml')) -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
    $win=Json 'windows-confirmed/receipt.json'
    Require ($win.source -ceq $value.source -and $win.package_passed -ceq $true -and $win.passed -ceq 316 -and $win.ignored -ceq 0 -and $win.images -ceq 79 -and $win.summaries.Count -eq 80 -and $win.recipe -ceq (Hash (Join-Path $PSScriptRoot 'windows.ps1'))) 'Windows package receipt mismatch'
    $images=@(Json 'windows-confirmed/images.json')
    Require ($images.Count -eq 79 -and @($images | Where-Object { $_.sha256 -cnotmatch '^[0-9a-f]{64}$' }).Count -eq 0) 'Windows image receipt mismatch'
    $summaries=@([regex]::Matches((Raw 'windows-confirmed/package.log'),'test result: (ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored;'))
    Require ($summaries.Count -eq 80 -and @($summaries | Where-Object { $_.Groups[1].Value -cne 'ok' -or $_.Groups[3].Value -cne '0' -or $_.Groups[4].Value -cne '0' }).Count -eq 0) 'Windows raw gate mismatch'
    Require ((($summaries | ForEach-Object {[int]$_.Groups[2].Value}) | Measure-Object -Sum).Sum -eq 316) 'Windows raw count mismatch'
    foreach($compiler in @('gnu194-final','gnu188-final')){
        $receipt=Json "$compiler/receipt.json"
        Require ($receipt.source -ceq $value.source -and $receipt.ordinary_targets_passed -ceq $true -and $receipt.artifact_tests -ceq 31 -and $receipt.receipt_tests -ceq 9 -and $receipt.seeded_mutations -ceq 32 -and $receipt.targets.Count -eq 2 -and $receipt.recipe -ceq (Hash (Join-Path $PSScriptRoot 'linux.ps1'))) 'Linux receipt mismatch'
        foreach($target in $receipt.targets){
            $identity=Json "$compiler/$($target.target)-identity.json"
            Require ($target.image -cmatch '^[0-9a-f]{64}  /home/artur/\.cache/' -and $target.image -ceq $identity.image -and $target.passed -ceq $identity.expected_tests) 'Linux image mismatch'
            Require ((Raw "$compiler/$($target.target).log").Contains("test result: ok. $($target.passed) passed; 0 failed; 0 ignored;")) 'Linux raw target mismatch'
            Require ([regex]::Matches((Raw "$compiler/$($target.target)-list.log"),'(?m)^.+: test\r?$').Count -eq $target.passed) 'Linux listing mismatch'
        }
        Require ((Raw "$compiler/diagnostic_artifacts.log").Contains('HC074-FROZEN-LOCK seed=0x074f10c mutations=32 production_pin_changed=false execution=false')) 'Seeded mutation scope mismatch'
        Require ((Raw "$compiler/diagnostic_receipts.log").Contains('synthetic_valid_report_survives_readonly_spool_and_offline_verification ... ok')) 'Linux-only receipt case missing'
    }
    foreach($cohort in @('gnu194-confirmed','gnu188-confirmed')){
        Require ((Json "$cohort/identity.json").recipe -ceq (Hash (Join-Path $PSScriptRoot 'linux-eight-case-oracle.ps1')) -and (Raw "$cohort/diagnostic_receipts.log").Contains('test result: ok. 9 passed; 0 failed;')) 'Historical selection refusal lost'
    }
    return $value
}
$taskMetadata=CheckRaw
if($Seal){
    Require (-not (Test-Path -LiteralPath $taskInventory)) 'Never reseal existing evidence'
    $taskPaths=[string[]]@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | ForEach-Object FullName)
    [Array]::Sort($taskPaths,[StringComparer]::Ordinal)
    $taskFiles=@($taskPaths | ForEach-Object {[ordered]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_).Replace('\','/');sha256=(Hash $_);bytes=(Get-Item -LiteralPath $_).Length}})
    [ordered]@{source=$taskMetadata.source;files=$taskFiles} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $taskInventory -Encoding utf8NoBOM
}
$taskIndex=Json 'inventory.json'
Require ($taskIndex.source -ceq $taskMetadata.source) 'Inventory source mismatch'
$taskSeen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($file in $taskIndex.files){
    Require ($file.path -cnotmatch '(^/|\\|(^|/)\.\.?(/|$)|:)' -and $taskSeen.Add($file.path) -and $file.path -cne 'inventory.json') 'Unsafe or duplicate inventory path'
    $path=Join-Path $PSScriptRoot $file.path
    Require ((Get-Item -LiteralPath $path).Length -ceq $file.bytes -and (Hash $path) -ceq $file.sha256) "Changed raw bytes: $($file.path)"
}
Require (@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse).Count -eq $taskSeen.Count+1) 'Unlisted evidence'
$taskMutations=0
if($SelfTest){
    foreach($field in $taskClosed){
        $changed=$taskMetadata | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        $changed.$field=$true
        $refused=$false
        try{Metadata $changed}catch{$refused=$true}
        Require $refused "Accepted promotion: $field"
        $taskMutations++
    }
    foreach($field in @('source','baseline','windows_ordinary_tests','linux_receipt_tests_per_compiler')){
        $changed=$taskMetadata | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        $changed.$field='changed'
        $refused=$false
        try{Metadata $changed}catch{$refused=$true}
        Require $refused "Accepted identity/count drift: $field"
        $taskMutations++
    }
}
Write-Output "frozen lock fixture evidence: $($taskSeen.Count) hashed files; $taskMutations mutation refusals; runtime and admission unchanged"
