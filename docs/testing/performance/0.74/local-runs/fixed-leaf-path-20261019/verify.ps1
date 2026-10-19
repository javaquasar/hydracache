param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
function Json([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path) | ConvertFrom-Json}
function Raw([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path)}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Check($Meta,$Inventory){
    if($Meta.schema -ne 1 -or $Meta.candidate_source -cne '0317f3dc7b977741cfb9491ad2ac417d3a62704e' -or $Meta.baseline_source -cne '13cdf24e316bf25c4a5695d5a9702450607915fb'){throw 'Source mismatch'}
    foreach($taskClosed in @('server','supervisor','product','original_constructor','original_start','all_writer_cessation','completion','unlock','wall_speedup_claimed')){
        if($Meta.$taskClosed -cne $false){throw "Not closed: $taskClosed"}
    }
    if($Meta.allocation_rule_passed -cne $true -or $Meta.local_retirement_gate_passed -cne $true -or $Meta.retirement_scope -cne 'local-simulated-root-controls' -or $Meta.source_contracts -ne 297){throw 'Scope or checks mismatch'}
    $taskManifest=Join-Path $PSScriptRoot '../../qualification-manifest.toml'
    if($Meta.manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (Hash $taskManifest) -cne $Meta.manifest_sha256){throw 'Frozen manifest drift'}
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object Name -ne 'inventory.json')
    if($Inventory.Count -ne $taskFiles.Count -or @($Inventory.path | Sort-Object -Unique).Count -ne $Inventory.Count){throw 'Missing or duplicate inventory'}
    foreach($taskFile in $taskFiles){
        $taskPath=[IO.Path]::GetRelativePath($PSScriptRoot,$taskFile.FullName).Replace('\','/')
        $taskEntry=@($Inventory | Where-Object path -CEQ $taskPath)
        if($taskEntry.Count -ne 1 -or $taskEntry[0].sha256 -cne (Hash $taskFile.FullName)){throw "Raw drift: $taskPath"}
    }
    $taskAnalysis=Json 'allocation-analysis.json'
    if($taskAnalysis.endpoint -cne 'rust-allocation-requests-only' -or $taskAnalysis.allocation_rule_passed -cne $true -or $taskAnalysis.wall_speedup_claimed -cne $false -or $taskAnalysis.server -cne $false -or $taskAnalysis.product -cne $false){throw 'Wrong endpoint'}
    foreach($taskCompiler in @('194','188')){
        $taskExpectedCompiler=if($taskCompiler -ceq '194'){'1.94.0'}else{'1.88.0'}
        $taskOldRows=$null
        foreach($taskStage in @('baseline','candidate')){
            $taskDir="alloc-$taskStage-gnu$taskCompiler"
            if($taskStage -ceq 'candidate' -and $taskCompiler -ceq '194'){$taskDir+='-retry'}
            $taskReceipt=Json "$taskDir/receipt.json"
            $taskSource=if($taskStage -ceq 'baseline'){$Meta.baseline_source}else{$Meta.candidate_source}
            if(-not $taskReceipt.source.StartsWith($taskSource) -or $taskReceipt.compiler -cne $taskExpectedCompiler -or $taskReceipt.profile -cne $true -or $taskReceipt.retirement_evidence -cne $false -or $taskReceipt.server -cne $false -or $taskReceipt.product -cne $false -or $taskReceipt.recipe -cne (Hash (Join-Path $PSScriptRoot 'profile.ps1')) -or $taskReceipt.samples -ne 3 -or $taskReceipt.iterations -ne 1024 -or $taskReceipt.counter_units -ne 2 -or $taskReceipt.origin_units -ne 20 -or $taskReceipt.image -cnotmatch '^[0-9a-f]{64}  /'){throw 'Profile identity mismatch'}
            foreach($taskLog in @(@('counter-units',2),@('origin-units',20),@('allocation',1))){
                if(-not (Raw "$taskDir/$($taskLog[0]).log").Contains("test result: ok. $($taskLog[1]) passed; 0 failed;")){throw 'Zero or failed profile tests'}
            }
            if(-not (Raw "$taskDir/clippy.log").Contains('Finished `dev` profile')){throw 'Missing strict lint'}
            $taskRows=@([regex]::Matches((Raw "$taskDir/allocation.log"),'HC074-LEAF-ALLOC (\{[^\r\n]+\})') | ForEach-Object {$_.Groups[1].Value | ConvertFrom-Json} | Sort-Object leaf,sample)
            if($taskRows.Count -ne 6 -or @($taskRows | ForEach-Object {"$($_.leaf):$($_.sample)"} | Sort-Object -Unique).Count -ne 6){throw 'Missing or duplicate samples'}
            for($taskI=0;$taskI -lt 6;$taskI++){
                $taskRow=$taskRows[$taskI]
                if($taskRow.leaf -cnotin @('boot','namespace') -or $taskRow.sample -notin @(0,1,2) -or $taskRow.iterations -ne 1024 -or $taskRow.rust_requests_only -cne $true){throw 'Wrong sample'}
                foreach($taskField in @('allocations','reallocations','requested_bytes')){
                    if($taskRow.candidate.$taskField -gt $taskRow.reference.$taskField){throw 'Allocation cost increased'}
                    if($taskStage -ceq 'baseline' -and $taskRow.candidate.$taskField -ne $taskRow.reference.$taskField){throw 'A/A mismatch'}
                    if($taskStage -ceq 'candidate' -and $taskRow.reference.$taskField -ne $taskOldRows[$taskI].reference.$taskField){throw 'Reference drift'}
                }
                if($taskStage -ceq 'candidate'){
                    if($taskRow.candidate.allocations -ge $taskRow.reference.allocations -and $taskRow.candidate.reallocations -ge $taskRow.reference.reallocations){throw 'No allocation reduction'}
                    $taskReported=$taskAnalysis.compilers.$taskCompiler[$taskI]
                    if($taskReported.sample -ne $taskRow.sample -or $taskReported.leaf -cne $taskRow.leaf -or $taskReported.iterations -ne 1024){throw 'Analysis sample mismatch'}
                    foreach($taskField in @('allocations','reallocations','requested_bytes')){
                        if($taskReported.before.$taskField -ne $taskRow.reference.$taskField -or $taskReported.after.$taskField -ne $taskRow.candidate.$taskField){throw 'Analysis count mismatch'}
                    }
                }
            }
            if($taskStage -ceq 'baseline'){$taskOldRows=$taskRows}
        }
        $taskBuild=Json "final-gnu$taskCompiler/build.json"
        if(-not $taskBuild.source.StartsWith($Meta.candidate_source) -or $taskBuild.compiler -cne $taskExpectedCompiler -or $taskBuild.server -cne $false -or $taskBuild.product -cne $false -or $taskBuild.recipe -cne (Hash (Join-Path $PSScriptRoot 'leaf.ps1')) -or $taskBuild.leaf_units -ne 2 -or $taskBuild.origin_units -ne 18){throw 'Ordinary image mismatch'}
        if(-not (Raw "final-gnu$taskCompiler/origin-regressions.log").Contains('test result: ok. 18 passed; 0 failed;')){throw 'Missing ordinary origin tests'}
    }
    foreach($taskKind in @('baseline','candidate')){
        $taskDir="$taskKind-gnu194"
        $taskReceipt=Json "$taskDir/receipt.json"
        if($taskReceipt.kind -cne $taskKind -or $taskReceipt.uid -cne '1000' -or $taskReceipt.retirement_repeats -ne 3 -or $taskReceipt.frame_cases -ne 8 -or $taskReceipt.clock_before -cne 'active' -or $taskReceipt.clock_after -cne 'active' -or (Raw "$taskDir/clock-restored.log").Trim() -cne 'active'){throw 'Retirement or restoration mismatch'}
        foreach($taskClosed in @('original_constructor','server','supervisor','product')){if($taskReceipt.$taskClosed -cne $false){throw 'Retirement exceeded scope'}}
        if($taskKind -ceq 'candidate' -and $taskReceipt.image -cne (Json 'final-gnu194/build.json').image){throw 'Not the unprofiled final image'}
        foreach($taskRepeat in 1..3){
            $taskRaw=Raw "$taskDir/retirement-$taskRepeat.log"
            if(-not $taskRaw.Contains('test result: ok. 1 passed; 0 failed;') -or -not $taskRaw.Contains('HC074-ROOT-RETIREMENT frame=true auth=68 channel_checks=4 retired_retry_auth=0 retired_retry_channel=0 original_bounds=true kernel=simulated original_constructor=false send=false')){throw 'Incomplete retirement callback'}
        }
        $taskRaw=Raw "$taskDir/frame-regression.log"
        if(-not $taskRaw.Contains('test result: ok. 1 passed; 0 failed;') -or ([regex]::Matches($taskRaw,'HC074-ORIGINAL-ROOT-FRAME case=')).Count -ne 8){throw 'Incomplete frame regression'}
    }
    if((Json 'aa-v3-analysis.json').accepted_both -cne $false){throw 'Negative timing result relabeled'}
    if((Raw 'fixture-before.json') -cne (Raw 'fixture-after.json') -or @(Json 'fixture-before.json').Count -ne 9){throw 'Signed fixture data drift'}
    if(-not (Raw 'source-full-confirmed.log').Contains('test result: ok. 297 passed; 0 failed;')){throw 'Missing source contracts'}
}
$taskMeta=Json 'metadata.json'
$taskInventory=if($Seal){
    if(Test-Path -LiteralPath (Join-Path $PSScriptRoot 'inventory.json')){throw 'Already sealed'}
    @(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Sort-Object FullName | ForEach-Object {[pscustomobject]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=Hash $_.FullName}})
}else{@(Json 'inventory.json')}
Check $taskMeta $taskInventory
if($SelfTest){
    foreach($taskFault in @('source','server','product','wall','scope','gate','manifest','count','hash','missing')){
        $taskCopy=$taskMeta | ConvertTo-Json -Depth 10 | ConvertFrom-Json
        $taskEntries=@($taskInventory | ConvertTo-Json -Depth 10 | ConvertFrom-Json)
        switch($taskFault){
            source {$taskCopy.candidate_source='00000000'}
            server {$taskCopy.server=$true}
            product {$taskCopy.product=$true}
            wall {$taskCopy.wall_speedup_claimed=$true}
            scope {$taskCopy.retirement_scope='production'}
            gate {$taskCopy.local_retirement_gate_passed=$false}
            manifest {$taskCopy.manifest_sha256='0'*64}
            count {$taskCopy.source_contracts=0}
            hash {$taskEntries[0].sha256='0'*64}
            missing {$taskEntries=@($taskEntries | Select-Object -Skip 1)}
        }
        $taskRejected=$false
        try{Check $taskCopy $taskEntries}catch{$taskRejected=$true}
        if(-not $taskRejected){throw "Mutation accepted: $taskFault"}
    }
}
if($Seal){ConvertTo-Json -InputObject $taskInventory -Depth 10 | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'inventory.json') -Encoding utf8NoBOM}
"Fixed leaf packet verified; files=$($taskInventory.Count); retirement=local-simulated-root; server=false; mutation_controls=10"
