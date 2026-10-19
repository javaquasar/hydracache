param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
$taskInventory=Join-Path $PSScriptRoot 'inventory.json'
function Require([bool]$Condition,[string]$Reason){if(-not $Condition){throw $Reason}}
function Metadata($Value){
    Require ($Value.source -ceq 'c59b677f7dc1b058aa692edfafc4d4679931fe00') 'Source mismatch'
    Require ($Value.baseline -ceq 'a33bd9a4d0e910ec71b78f640f19b8404517ce2f') 'Baseline mismatch'
    Require ($Value.source_contracts -ceq 321 -and $Value.previous_sources -ceq 432) 'Source scope mismatch'
    Require ($Value.selected_units_per_compiler -ceq 77 -and $Value.doc_tests_per_compiler -ceq 247) 'Linux scope mismatch'
    Require ($Value.readonly_checks_per_compiler -ceq 1 -and $Value.windows_units -ceq 60 -and $Value.windows_docs -ceq 20) 'Check scope mismatch'
    Require ($Value.old_bounds_ns.Count -eq 3 -and ($Value.old_bounds_ns -join ',') -ceq '300000000000,60000000000,10000000000') 'Original bounds mismatch'
    Require ($Value.scoped_local_checks_passed -ceq $true -and $Value.runtime_changed -ceq $true -and $Value.frozen_lock_fixture_mismatch -ceq $true) 'Result mismatch'
    foreach($field in @('fixed_root_admitted','actual_systemd_placement','exclusive_population','artifact_admission','held_unit_projected','original_send','barrier_release','completion','fence_release','server','supervisor','product','qualification','clock_changed','new_fault_cohort','performance_claim','full_live_matrix_at_this_source','broader_windows_suite_passed')){
        Require ($Value.$field -ceq $false) "Forbidden promotion: $field"
    }
}
function CheckRaw {
    $value=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'metadata.json') | ConvertFrom-Json
    Metadata $value
    foreach($compiler in @('gnu194-confirmed','gnu188-confirmed')){
        $receipt=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$compiler/receipt.json") | ConvertFrom-Json
        Require ($receipt.source -ceq $value.source -and $receipt.selected_units -ceq 77 -and $receipt.doc_tests -ceq 247 -and $receipt.readonly_anchor_checks -ceq 1) 'Compiler receipt mismatch'
        Require ($receipt.image -cmatch '^[0-9a-f]{64}  /home/artur/\.cache/') 'Image receipt mismatch'
        Require ($receipt.recipe -ceq (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'build.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()) 'Recipe mismatch'
        Require ($receipt.clock_changed -ceq $false -and $receipt.original_send -ceq $false -and $receipt.fixed_root_admitted -ceq $false) 'Receipt promotion'
        foreach($check in @(@('fixed-units',9),@('placement-units',8),@('trace-units',5),@('cost-units',2),@('launch-units',11),@('enrollment-units',5),@('owned-units',2),@('origin-units',22),@('closing-units',4),@('root-units',9),@('readonly-anchors',1),@('doc',247))){
            $raw=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$compiler/$($check[0]).log")
            Require ($raw.Contains("test result: ok. $($check[1]) passed; 0 failed;")) 'Test selection mismatch'
        }
        $raw=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$compiler/readonly-anchors.log")
        Require ($raw -cmatch 'HC074-FIXED-SLICE-ANCHORS device=[0-9]+ inode=[0-9]+ readonly=true fixed_root_admitted=false original_send=false server=false product=false') 'Anchor claim mismatch'
    }
    Require ((Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'source-full.log')).Contains('test result: ok. 321 passed; 0 failed;')) 'Full source gate incomplete'
    $windows=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'windows-lib.log')
    $docs=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'windows-doc.log')
    Require ($windows.Contains('test result: ok. 60 passed; 0 failed;') -and $docs.Contains('test result: ok. 20 passed; 0 failed;')) 'Portable scoped gate incomplete'
    $broad=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'windows-tests.log')
    Require ($broad.Contains('test result: FAILED. 2 passed; 13 failed;') -and $broad.Contains('629b483af08c2bb9137e9bd8fd120fde6c7a3d33ca9afba07e7de98097ed7b99')) 'Lost broader negative'
    $baseline=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'frozen-fixture-baseline-analysis.json') | ConvertFrom-Json
    Require ($baseline.baseline -ceq $value.baseline -and $baseline.matches_test_actual -ceq $true -and $baseline.baseline_broad_test_executed -ceq $false -and $baseline.frozen_hash_changed -ceq $false -and $baseline.existing_tests_changed -ceq $false) 'Baseline diagnosis mismatch'
    $diffs=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'identity-diffs.json') | ConvertFrom-Json
    Require ($diffs.source -ceq $value.source -and $diffs.baseline -ceq $value.baseline) 'Diff identity mismatch'
    foreach($field in @('baseline_images_source_diff_exit','frozen_fixture_diff_exit','previous_evidence_diff_exit','qualification_diff_exit','format_exit')){Require ($diffs.$field -ceq 0) "Identity drift: $field"}
    $manifest=Join-Path $PSScriptRoot '../../qualification-manifest.toml'
    Require ((Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash.ToLowerInvariant() -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
    return $value
}
$taskMetadata=CheckRaw
if($Seal){
    Require (-not (Test-Path -LiteralPath $taskInventory)) 'Never reseal existing evidence'
    $taskPaths=[string[]]@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | ForEach-Object FullName)
    [Array]::Sort($taskPaths,[StringComparer]::Ordinal)
    $taskFiles=@($taskPaths | ForEach-Object {
        [ordered]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant();bytes=(Get-Item -LiteralPath $_).Length}
    })
    [ordered]@{source=$taskMetadata.source;files=$taskFiles} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $taskInventory -Encoding utf8NoBOM
}
$taskIndex=Get-Content -Raw -LiteralPath $taskInventory | ConvertFrom-Json
Require ($taskIndex.source -ceq $taskMetadata.source) 'Inventory source mismatch'
$taskSeen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($file in $taskIndex.files){
    Require ($file.path -cnotmatch '(^/|\\|(^|/)\.\.?(/|$)|:)' -and $taskSeen.Add($file.path) -and $file.path -cne 'inventory.json') 'Unsafe or duplicate inventory path'
    $path=Join-Path $PSScriptRoot $file.path
    Require ((Get-Item -LiteralPath $path).Length -ceq $file.bytes -and (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) "Changed raw bytes: $($file.path)"
}
Require (@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse).Count -eq $taskSeen.Count+1) 'Unlisted evidence'
$taskMutations=0
if($SelfTest){
    foreach($field in @('fixed_root_admitted','actual_systemd_placement','exclusive_population','artifact_admission','held_unit_projected','original_send','barrier_release','completion','fence_release','server','supervisor','product','qualification','clock_changed','new_fault_cohort','performance_claim','full_live_matrix_at_this_source','broader_windows_suite_passed')){
        $changed=$taskMetadata | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        $changed.$field=$true
        $rejected=$false
        try{Metadata $changed}catch{$rejected=$true}
        Require $rejected "Mutation accepted: $field"
        $taskMutations++
    }
    foreach($field in @('source','baseline','source_contracts','old_bounds_ns')){
        $changed=$taskMetadata | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        $changed.$field=if($field -ceq 'old_bounds_ns'){@(300000000000,60000000000,11000000000)}else{'changed'}
        $rejected=$false
        try{Metadata $changed}catch{$rejected=$true}
        Require $rejected "Mutation accepted: $field"
        $taskMutations++
    }
}
Write-Output "fixed-slice evidence: $($taskSeen.Count) hashed files; $taskMutations metadata mutation refusals; no actual fixed-root admission"
