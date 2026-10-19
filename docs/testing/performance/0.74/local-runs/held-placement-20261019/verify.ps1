param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
function Raw([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path)}
function Json([string]$Path){Raw $Path | ConvertFrom-Json}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Check($Meta,$Inventory){
    if($Meta.schema -ne 1 -or $Meta.baseline -cne 'b23a503bb81c962d578143e72d0c3dbc7fd82fe0' -or $Meta.source -cne '11e8a588a3b05b60f5c466e3fce7ca7ecb4ab923' -or $Meta.original_files -ne 423 -or $Meta.source_contracts -ne 309 -or $Meta.effect_brackets -ne 46 -or ($Meta.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Original source/check schedule'}
    if($Meta.seed -cne '0x074e5a1c' -or $Meta.identities -ne 128 -or $Meta.packet_mutations -ne 104 -or $Meta.new_units -ne 6){throw 'Reader mutation coverage'}
    foreach($taskClosed in @('held_projection','original_send','barrier_release','completion','fence_release','production_enrollment','server','supervisor','product','qualification','clock_changed','new_fault_cohort','preparation_wall_speedup_claimed','allocation_reduction_claimed')){if($Meta.$taskClosed -isnot [bool] -or $Meta.$taskClosed -cne $false){throw "Boundary changed: $taskClosed"}}
    if($Meta.manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (Hash (Join-Path $PSScriptRoot '../../qualification-manifest.toml')) -cne $Meta.manifest_sha256){throw 'Qualification manifest drift'}
    if($Meta.prior_inventory_sha256 -cne '4cf7556728a2146a8b07e8edbffb64951f3cb4c96e3232c60c26abc1047e80d0' -or (Hash (Join-Path $PSScriptRoot '../boot-comparison-20261019/inventory.json')) -cne $Meta.prior_inventory_sha256){throw 'Prior sealed evidence drift'}
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object FullName -CNE (Join-Path $PSScriptRoot 'inventory.json'))
    if($Inventory.Count -ne $taskFiles.Count -or @($Inventory.path | Sort-Object -Unique).Count -ne $Inventory.Count){throw 'Missing/duplicate inventory'}
    foreach($taskFile in $taskFiles){
        $taskPath=[IO.Path]::GetRelativePath($PSScriptRoot,$taskFile.FullName).Replace('\','/')
        $taskEntry=@($Inventory | Where-Object path -CEQ $taskPath)
        if($taskEntry.Count -ne 1 -or $taskEntry[0].sha256 -cne (Hash $taskFile.FullName)){throw "Raw drift: $taskPath"}
    }
    foreach($taskTest in @(@('baseline194.log',5),@('baseline188.log',5),@('green-portable.log',6),@('source-screen.log',3),@('source-full.log',309),@('windows-unit.log',60),@('windows-doc.log',20))){if(-not (Raw $taskTest[0]).Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Missing local test receipt'}}
    foreach($taskLint in @('windows-clippy.log','xtask-clippy.log')){if(-not (Raw $taskLint).Contains('Finished `dev` profile')){throw 'Strict lint missing'}}
    if(-not (Raw 'registry.log').Contains('gated-test-check: OK (184 discovered gates, all registered)') -or -not (Raw 'docs-post-results.log').Contains('doc-check: OK') -or -not (Raw 'local-performance-post-results.log').Contains('OK (local, non-promotable)') -or (Raw 'fmt-result.log').Trim() -cne 'Formatting passed'){throw 'Local contracts'}
    if(-not (Raw 'red-missing-module.log').Contains('E0432') -or -not (Raw 'red-missing-module.log').Contains('held_placement')){throw 'Missing red-test evidence'}
    foreach($taskBaselineImage in @('0a70454a1bbf3b94257b5c9ba881794b29f8c6347622bfecffbd55863089698c','155de08349a7e7d8b28a95cc71ba893544e73707d1b7d13d4c978214b67c6ecb')){if(-not (Raw 'baseline-images.log').Contains($taskBaselineImage)){throw 'Baseline identity'}}
    foreach($taskCompiler in @('194','188')){
        $taskDir="gnu$taskCompiler-final"
        $taskBuild=Json "$taskDir/receipt.json"
        $taskVersion=if($taskCompiler -ceq '194'){'1.94.0'}else{'1.88.0'}
        if($taskBuild.source -cne $Meta.source -or $taskBuild.compiler -cne $taskVersion -or $taskBuild.image -cnotmatch '^[0-9a-f]{64}  /home/artur/\.cache/hydracache-074-' -or $taskBuild.recipe -cne (Hash (Join-Path $PSScriptRoot 'build.ps1')) -or $taskBuild.launch_units -ne 11 -or $taskBuild.enrollment_units -ne 5 -or $taskBuild.doc_tests -ne 247){throw 'Build identity'}
        foreach($taskClosed in @('profile','original_send','server','supervisor','product','clock_changed','new_fault_cohort')){if($taskBuild.$taskClosed -isnot [bool] -or $taskBuild.$taskClosed -cne $false){throw 'Build scope'}}
        foreach($taskTest in @(@('launch-units',11),@('enrollment-units',5),@('owned-units',2),@('origin-units',22),@('closing-units',4),@('root-units',9),@('doc',247))){if(-not (Raw "$taskDir/$($taskTest[0]).log").Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Linux units'}}
        if(-not (Raw "$taskDir/launch-units.log").Contains('HC074-HELD-PLACEMENT seed=0x074e5a1c identities=128 runtime_admission=false') -or -not (Raw "$taskDir/clippy.log").Contains('Finished `dev` profile') -or -not (Raw "$taskDir/compiler.log").Contains("release: $taskVersion")){throw 'Linux recipe receipt'}
    }
    $taskInitial=Json 'initial-wrapper-outcomes.json'
    if($taskInitial.source -cne $Meta.source -or $taskInitial.recipe -cne '58d4c2238ea688672207a236890a3aa1b2bdc0936bf6c1ba85461e962f3a5a8d' -or (Hash (Join-Path $PSScriptRoot 'build-initial.ps1')) -cne $taskInitial.recipe -or $taskInitial.expected_doc_tests -ne 20 -or $taskInitial.actual_doc_tests -ne 247 -or $taskInitial.wrapper_exit_code -ne 1 -or $taskInitial.runtime_changed -cne $false){throw 'Initial wrapper failure not retained'}
    foreach($taskDir in @('gnu194','gnu188')){if((Test-Path -LiteralPath (Join-Path $PSScriptRoot "$taskDir/receipt.json")) -or -not (Raw "$taskDir/doc.log").Contains('test result: ok. 247 passed; 0 failed;')){throw 'Initial raw doc result drift'}}
}
$taskInventoryPath=Join-Path $PSScriptRoot 'inventory.json'
$taskMeta=Json 'metadata.json'
if($Seal){
    if(Test-Path -LiteralPath $taskInventoryPath){throw 'Never reseal retained evidence'}
    $taskInventory=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Sort-Object FullName | ForEach-Object {[ordered]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=Hash $_.FullName}})
    Check $taskMeta $taskInventory
    $taskInventory | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $taskInventoryPath -Encoding utf8NoBOM
}
$taskInventory=@(Json 'inventory.json')
Check $taskMeta $taskInventory
if($SelfTest){
    $taskRejected=0
    foreach($taskField in @('source','baseline','original_files','source_contracts','effect_brackets','held_projection','original_send','barrier_release','completion','fence_release','server','product','preparation_wall_speedup_claimed')){
        $taskChanged=$taskMeta | ConvertTo-Json -Depth 30 | ConvertFrom-Json
        $taskValue=$taskChanged.$taskField
        $taskChanged.$taskField=if($taskValue -is [bool]){-not $taskValue}elseif($taskValue -is [string]){"${taskValue}x"}else{$taskValue+1}
        $taskRefused=$false;try{Check $taskChanged $taskInventory}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted mutation: $taskField"};$taskRejected++
    }
    foreach($taskKind in @('missing','duplicate','hash')){
        $taskChanged=@($taskInventory | ConvertTo-Json -Depth 30 | ConvertFrom-Json)
        switch($taskKind){'missing'{$taskChanged=$taskChanged[1..($taskChanged.Count-1)]};'duplicate'{$taskChanged+=,$taskChanged[0]};'hash'{$taskChanged[0].sha256='0'*64}}
        $taskRefused=$false;try{Check $taskMeta $taskChanged}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted inventory mutation: $taskKind"};$taskRejected++
    }
    if($taskRejected -ne 16){throw 'Mutation selection drift'}
    Write-Output "held-placement verifier mutations: $taskRejected refused"
}
Write-Output "held-placement evidence: $($taskInventory.Count) raw files verified; runtime admission remains closed"
