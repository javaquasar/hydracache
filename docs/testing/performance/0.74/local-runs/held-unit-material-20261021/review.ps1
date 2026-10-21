param([ValidateSet('Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
Set-Location (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
function Need([bool]$Value,[string]$Message){if(-not $Value){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function TextHash([string]$Value){[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.UTF8Encoding]::new($false).GetBytes($Value))).ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 60 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 60 -Compress)))}
function WriteNew([string]$Name,$Value){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject $Value -Depth 30)+"`n")
    $taskStream=[IO.File]::Open((Join-Path $PSScriptRoot $Name),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
foreach($taskPrefix in @('','final/')){
    $taskContext=Json "${taskPrefix}194-before.json"
    foreach($taskLane in @('194','188','host')){
        $taskBefore=Json "$taskPrefix$taskLane-before.json";$taskAfter=Json "$taskPrefix$taskLane-after.json"
        Need ((Same $taskBefore $taskAfter) -and (Same $taskContext $taskBefore)) 'Cohort source drift'
        $taskChecks=Json "$taskPrefix$taskLane-checks.json"
        $taskCount=if($taskLane -ceq 'host'){8}else{7}
        Need (-not $taskChecks.failure -and $taskChecks.steps.Count -eq $taskCount) 'Incomplete checks'
        foreach($taskFlag in @('new_original_live_fixture','server','original_send','product','performance_measurement')){Need ($taskChecks.$taskFlag -is [bool] -and -not $taskChecks.$taskFlag) 'Scope changed'}
        foreach($taskRow in $taskChecks.steps){Need ($taskRow.exit -eq 0 -and (Hash (Join-Path $PSScriptRoot "$taskPrefix$($taskRow.log)")) -ceq $taskRow.sha256) 'Check log changed'}
        if($taskLane -cne 'host'){
            foreach($taskCase in @(@('material',12),@('projection',10),@('consuming',10),@('reply',12),@('roles',38))){
                $taskText=Get-Content (Join-Path $PSScriptRoot "$taskPrefix$taskLane-$($taskCase[0]).log") -Raw
                Need ($taskText -match "test result: ok\. $($taskCase[1]) passed; 0 failed; 0 ignored;") 'Test count changed'
            }
        }
    }
}
$taskInitial=Json '194-before.json';$taskFinal=Json 'final/194-before.json'
Need ($taskInitial.Count -eq $taskFinal.Count) 'Source membership changed'
foreach($taskRow in $taskFinal){Need ((Hash $taskRow.path) -ceq $taskRow.sha256) 'Final source changed'}
$taskKernel='tools/long-run-supervisor-074/src/diagnostic_held_unit_material_kernel.rs'
$taskCanary='crates/xtask/tests/held_material_074.rs'
foreach($taskRow in $taskInitial){
    $taskCurrent=@($taskFinal|Where-Object path -CEQ $taskRow.path)
    Need ($taskCurrent.Count -eq 1) 'Missing source'
    if($taskRow.path -cne $taskKernel -and $taskRow.path -cne $taskCanary){Need ($taskRow.sha256 -ceq $taskCurrent[0].sha256) 'Unrelated source drift'}
}
$taskRaw=[IO.File]::ReadAllText((Join-Path (Get-Location) $taskKernel)).Replace("`r",'')
$taskNew="        let descriptors = fds.into_iter().map(stamp).collect::<io::Result<_>>()?;`n        owner(b)?;`n        Ok(descriptors)`n"
Need ($taskRaw.Split($taskNew,[StringSplitOptions]::None).Count -eq 2) 'Closing census check changed'
Need ((TextHash $taskRaw.Replace($taskNew,"        fds.into_iter().map(stamp).collect()`n")) -ceq @($taskInitial|Where-Object path -CEQ $taskKernel)[0].sha256) 'Initial kernel reconstruction differs'
$taskRaw=[IO.File]::ReadAllText((Join-Path (Get-Location) $taskCanary)).Replace("`r",'')
$taskMarker="`n#[test]`nfn held_material_inventory_requires_fresh_original_owner_and_deadline_after_census() {"
Need ($taskRaw.Split($taskMarker,[StringSplitOptions]::None).Count -eq 2 -and $taskRaw.Split("`nfn read(path:",[StringSplitOptions]::None).Count -eq 2) 'Exact new canary boundary changed'
$taskStart=$taskRaw.IndexOf($taskMarker,[StringComparison]::Ordinal);$taskEnd=$taskRaw.IndexOf("`nfn read(path:",[StringComparison]::Ordinal)
Need ($taskEnd -gt $taskStart) 'Canary boundary order changed'
Need ((TextHash $taskRaw.Remove($taskStart,$taskEnd-$taskStart+1)) -ceq @($taskInitial|Where-Object path -CEQ $taskCanary)[0].sha256) 'Initial canary reconstruction differs'
Need ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
$taskPrior='docs/testing/performance/0.74/local-runs/held-unit-projection-20261021'
Need ((Hash "$taskPrior/inventory.json") -ceq '65d6e666ff77130752a1a374f3562c06707c2a87711a267fa685542fbe62c3b4') 'Prior packet changed'
$taskPriorFiles=(Get-Content "$taskPrior/inventory.json" -Raw|ConvertFrom-Json).files
Need ($taskPriorFiles.Count -eq 39) 'Prior members changed'
foreach($taskRow in $taskPriorFiles){Need ((Hash "$taskPrior/$($taskRow.path)") -ceq $taskRow.sha256) 'Prior member changed'}
Need ((Get-Content (Join-Path $PSScriptRoot 'final-docs.log') -Raw) -match 'doc-check: OK') 'Final docs refused'
if($Mode -ceq 'Seal'){
    $taskReports=@('docs/testing/performance/0.74/held-unit-material-20261021.md','docs/articles/007-optimization-starts-with-measurement.md','docs/testing/performance/0.74/README.md','docs/testing/performance/0.74/diagnostic-server-readiness-intermediate-plan-20261012.md')
    WriteNew 'binding.json' @{runtime_source=(& git rev-parse HEAD).Trim();recipe_sha256=(Hash (Join-Path $PSScriptRoot 'checks.ps1'));reports=@($taskReports|ForEach-Object {@{path=$_;sha256=(Hash $_)}});
        original_windows_seconds=@(300,60,10);new_kernel_objects=6;extended_descriptor_roles=20;initial_candidate_accepted=$false;materialized=$true;original_owner_live_gate=$false;original_send=$false;server=$false;performance_claim=$false}
}
$taskBinding=Json 'binding.json'
Need ($taskBinding.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'checks.ps1'))) 'Recipe changed'
Need ((Same $taskBinding.original_windows_seconds @(300,60,10)) -and $taskBinding.materialized -is [bool] -and $taskBinding.materialized) 'Materialization/windows changed'
Need ($taskBinding.new_kernel_objects -eq 6 -and $taskBinding.extended_descriptor_roles -eq 20) 'Role count changed'
foreach($taskFlag in @('initial_candidate_accepted','original_owner_live_gate','original_send','server','performance_claim')){Need ($taskBinding.$taskFlag -is [bool] -and -not $taskBinding.$taskFlag) 'Admission scope changed'}
foreach($taskRow in $taskBinding.reports){Need ((Hash $taskRow.path) -ceq $taskRow.sha256) 'Report changed'}
function Inventory{@{files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
if($Mode -ceq 'Seal'){WriteNew 'inventory.json' (Inventory)}
Need (Same (Json 'inventory.json') (Inventory)) 'Inventory changed'
@{result='pass';members=(Json 'inventory.json').files.Count;inventory_sha256=(Hash (Join-Path $PSScriptRoot 'inventory.json'));runtime_source=$taskBinding.runtime_source;original_owner_live_gate=$false;server=$false}|ConvertTo-Json
