$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskInventory=Join-Path $PSScriptRoot 'inventory.json'
if(Test-Path -LiteralPath $taskInventory){throw 'Never reseal a retained packet'}
Push-Location $taskRoot
try {
    $taskSource='2f27453f48e038457b3591e98c43ce1f40a91e31'
    if((& git rev-parse HEAD).Trim() -cne $taskSource){throw 'Wrong selected source'}
    foreach($taskLabel in @('gnu194-final','gnu188-final')){
        $taskReceipt=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$taskLabel/receipt.json") | ConvertFrom-Json
        if($taskReceipt.source -cne $taskSource -or $taskReceipt.kernel_cases -ne 9 -or $taskReceipt.cleanup -cne $true){throw 'Incomplete compiler cohort'}
    }
    $taskQualification=(Get-FileHash -LiteralPath 'docs/testing/performance/0.74/qualification-manifest.toml' -Algorithm SHA256).Hash.ToLowerInvariant()
    if($taskQualification -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Qualification changed'}
    [ordered]@{
        schema='fixed-slice-namespace-local-evidence-v1'
        source=$taskSource;baseline='9724e428b9850df8ff5aae34caa47ad8ba18e702'
        source_contracts_reference='75f3aeabde2c461bec9c4574c691b13e2bc1cc99'
        source_contracts_stable_head=$false
        kernel_cases_per_compiler=9;ordinary_tests_per_compiler=9;docs_per_compiler=247
        source_contracts=321;fixture_source_guards=6;windows_lib_tests=60;windows_doc_tests=20
        qualification_manifest_sha256=$taskQualification
        host_anchors_unchanged=$true;cleanup=$true;production_runtime_changed=$false
        original_join=$false;manager_ownership=$false;exclusive_population=$false
        artifact_admission=$false;original_send=$false;server=$false;supervisor=$false
        product=$false;qualification=$false;clock_changed=$false
    } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'metadata.json') -Encoding utf8NoBOM
    [string[]]$taskPaths=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | ForEach-Object {$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
    [Array]::Sort($taskPaths,[StringComparer]::Ordinal)
    $taskMembers=@(foreach($taskPath in $taskPaths){
        $taskFile=Get-Item -LiteralPath (Join-Path $PSScriptRoot $taskPath)
        [ordered]@{path=$taskPath;bytes=$taskFile.Length;sha256=(Get-FileHash -LiteralPath $taskFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
    })
    [ordered]@{schema='sha256-local-file-inventory-v1';files=$taskMembers} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $taskInventory -Encoding utf8NoBOM
    & (Join-Path $PSScriptRoot 'verify.ps1') -SelfTest
    if(-not $?){throw 'Packet verification refused'}
}finally{Pop-Location}
