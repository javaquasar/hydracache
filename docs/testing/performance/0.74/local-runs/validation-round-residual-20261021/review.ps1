param([ValidateSet('Facts','WriteSummary','Seal','Review','Audit')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 70 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 70 -Compress)))}
function Rows([string]$File,[string]$Prefix){
    foreach($taskLine in Get-Content -LiteralPath $File){$taskAt=$taskLine.IndexOf($Prefix,[StringComparison]::Ordinal);if($taskAt -ge 0){$taskLine.Substring($taskAt+$Prefix.Length)|ConvertFrom-Json}}
}
function Inactive($Row,[string[]]$Flags){foreach($taskFlag in $Flags){Require ($Row.$taskFlag -is [bool] -and -not $Row.$taskFlag) "Scope changed: $taskFlag"}}
function Range($Values){@{minimum=($Values|Measure-Object -Minimum).Minimum;maximum=($Values|Measure-Object -Maximum).Maximum}}
function Validate-Phases($Phases,$Old,[string]$Mutation=''){
    Require ($Phases.Count -eq 10 -and (Same @($Phases.phase) @($Old.phase))) 'Ten ordered phases required'
    if($Mutation -ceq 'parent'){$Phases[0].authentication_calls++}
    if($Mutation -ceq 'missing'){$Phases[2].round_steps=@($Phases[2].round_steps|Select-Object -First 7)}
    if($Mutation -ceq 'edge'){$Phases[2].round_steps[1].kernel[5].calls++}
    if($Mutation -ceq 'scope'){$Phases[0].guard_clock_added=$true}
    $taskOutputs=$null
    for($taskIndex=0;$taskIndex -lt 10;$taskIndex++){
        $taskRow=$Phases[$taskIndex];$taskPrior=$Old[$taskIndex]
        Require ($taskRow.returned -eq $true -and $taskRow.instrumented -eq $true -and $taskRow.overlapping_spans -eq $true -and $taskRow.wall_ns -gt 0 -and $null -ne $taskRow.process_cpu_ns -and $taskRow.process_cpu_ns -ge 0) 'Phase incomplete'
        Inactive $taskRow @('includes_child_cpu','guard_clock_added','deadline_changed','healthy_gate','server','product')
        Require ((Same @($taskRow.kernel.calls) @($taskPrior.kernel.calls)) -and (Same @($taskRow.live.calls) @($taskPrior.live.calls)) -and $taskRow.authentication_calls -eq $taskPrior.authentication_calls -and $taskRow.signature_calls -eq $taskPrior.signature_calls) 'Parent call frequency changed'
        Require ($taskRow.round_steps.Count -eq 8) 'Missing round step'
        for($taskStep=0;$taskStep -lt 8;$taskStep++){
            $taskPart=$taskRow.round_steps[$taskStep];$taskOldStep=$taskPrior.round_steps[$taskStep]
            Require ($taskPart.step -eq $taskStep -and $taskPart.calls -eq $taskPart.completed -and $taskPart.calls -eq $taskOldStep.calls -and $taskPart.overlapping_spans -eq $true) 'Step order/completeness changed'
            Inactive $taskPart @('guard_clock_added')
            Require ($taskPart.kernel.Count -eq 10 -and $taskPart.roots.Count -eq 10 -and $taskPart.live.Count -eq 6) 'Graph shape changed'
            Require ((Same @($taskPart.kernel.calls) @($taskOldStep.kernel.calls)) -and (Same @($taskPart.live.calls) @($taskOldStep.live.calls)) -and (Same @($taskPart.roots) @($taskOldStep.roots)) -and $taskPart.authentication_calls -eq $taskOldStep.authentication_calls -and $taskPart.signature_calls -eq $taskOldStep.signature_calls) 'Per-step calls changed'
            Require (@($taskPart.edges|Where-Object {$_.parent -lt 0 -or $_.parent -ge 10 -or $_.child -lt 0 -or $_.child -ge 10 -or $_.calls -le 0}).Count -eq 0 -and @($taskPart.edges|Group-Object parent,child|Where-Object Count -ne 1).Count -eq 0) 'Invalid/duplicate edge'
            Require (Same @($taskPart.edges|ForEach-Object {@{parent=$_.parent;child=$_.child;calls=$_.calls}}) @($taskOldStep.edges|ForEach-Object {@{parent=$_.parent;child=$_.child;calls=$_.calls}})) 'Edge call frequency changed'
            foreach($taskKind in 0..9){
                $taskNode=$taskPart.kernel[$taskKind]
                $taskIncoming=($taskPart.edges|Where-Object child -eq $taskKind|Measure-Object calls -Sum).Sum
                $taskChildren=($taskPart.edges|Where-Object parent -eq $taskKind|Measure-Object inclusive_ns -Sum).Sum
                Require ($taskNode.calls -eq $taskPart.roots[$taskKind]+$taskIncoming -and $taskNode.inclusive_ns -eq $taskNode.outside_children_ns+$taskChildren) 'Graph count/time not conserved'
            }
            $taskOutside=($taskPart.kernel.outside_children_ns|Measure-Object -Sum).Sum
            Require ($taskPart.wall_ns -ge $taskOutside -and $taskOutside -ge 0) 'Kernel wall exceeds step'
        }
        if($taskRow.phase -ceq 'outputs'){
            Require ($taskRow.authentication_calls -eq 782 -and @($taskRow.round_steps|Where-Object calls -ne 46).Count -eq 0) '46 original output rounds required'
            foreach($taskKind in 0..9){Require (($taskRow.round_steps|ForEach-Object {$_.kernel[$taskKind].calls}|Measure-Object -Sum).Sum -eq $taskRow.kernel[$taskKind].calls) 'Output kernel calls not reconciled'}
            $taskOutputs=$taskRow
        }
    }
    Require ($null -ne $taskOutputs) 'Output phase missing'
    $taskOutputs
}
function No-Later([bool]$Stopped,[bool]$Exists){Require (-not($Stopped -and $Exists)) 'Continuation after first refusal'}
function Binding([string]$Source,[string]$Expected,[string]$Before,[string]$After){Require ($Source -cmatch '^[0-9a-f]{40}$' -and $Source -ceq $Expected -and $Before -cmatch '^[0-9a-f]{64}$' -and $Before -ceq $After) 'Source/image binding differs'}
function Bundle-Equal($A,$B){Require ($A.Count -eq 9 -and (Same $A $B)) 'Original bundle changed'}
function Facts{
    $taskAuthoring=Json 'authoring-refusal.json'
    Require ($taskAuthoring.measured_invocations_started -eq $false -and $taskAuthoring.refused_recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'review-refused.ps1'))) 'Original authoring refusal missing'
    $taskAudit=Json 'review-audit.json'
    Require (($taskAudit.mutations.mutation -join ',') -ceq 'parent,missing,edge,scope,continue-after-refusal,source,image,bundle' -and @($taskAudit.mutations|Where-Object refused -ne $true).Count -eq 0) 'Falsifying controls incomplete'
    $taskPrior=Join-Path (Split-Path $PSScriptRoot) 'authority-open-name-20261021'
    Require ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq '7d5a88964d1df7498aced1d5ce81fbd8b4ad16fe693645bb5044503717a67ce8') 'Prior seal changed'
    & (Join-Path $taskPrior 'final-review.ps1') -Mode Review|Out-Null
    $taskInitial=Json 'context-Before.json';$taskCleanup=Json 'root-toolchain-cleanup.json'
    Require ($taskInitial.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'context.ps1')) -and @($taskInitial.commands|Where-Object {$_.output.Contains('has been auto-installed')}).Count -eq 2) 'Initial context mutation must remain visible'
    Require ($taskCleanup.exit -eq 0 -and $taskCleanup.after.Contains('no installed toolchains') -and $taskCleanup.preflight_auto_installed -eq $true -and $taskCleanup.read_only_query_was_false -eq $true -and $taskCleanup.measured_invocations_started -eq $false -and $taskCleanup.artur_toolchains_touched -eq $false) 'Root toolchain side effect/cleanup not accounted'
    $taskBefore=Json 'context-Verified-Before.json';$taskAfter=Json 'context-Verified-After.json';$taskSource=$taskBefore.source
    Require ($taskAfter.source -ceq $taskSource -and $taskSource -cmatch '^[0-9a-f]{40}$') 'Context source changed'
    & git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d $taskSource -- tools/long-run-supervisor-074 crates/xtask Cargo.lock
    Require ($LASTEXITCODE -eq 0) 'Runtime/observer tree changed'
    $taskOldInputs=Get-Content (Join-Path $PSScriptRoot '../validation-round-cost-20261020/inputs-before.json') -Raw|ConvertFrom-Json
    foreach($taskContext in @($taskBefore,$taskAfter)){
        Require ($taskContext.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'context-repair.ps1')) -and ($taskContext.commands.name -join ',') -ceq 'boot,config,kernel,fixtures,compiler194,compiler188' -and @($taskContext.commands|Where-Object {$_.exit -ne 0 -or $_.output.Contains('auto-installed')}).Count -eq 0) 'Context receipt changed'
        Require (-not ($taskContext.commands|Where-Object name -ceq 'fixtures').output.Trim()) 'Owned fixture residue'
        Bundle-Equal $taskContext.bundle $taskOldInputs.bundle
        Require ($taskContext.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
        Inactive $taskContext @('clocks_changed','services_changed','server','product')
    }
    $taskBoot=($taskBefore.commands|Where-Object name -ceq 'boot').output.Trim()
    Require ($taskBoot -cmatch '^[0-9a-f-]{36}$' -and $taskBoot -ceq ($taskAfter.commands|Where-Object name -ceq 'boot').output.Trim() -and (Same $taskBefore.windows_config $taskAfter.windows_config) -and ($taskBefore.commands|Where-Object name -ceq 'config').output -ceq ($taskAfter.commands|Where-Object name -ceq 'config').output) 'Boot/configuration mismatch; no reset/retry'
    $taskOld=@(Rows (Join-Path $PSScriptRoot '../validation-round-cost-20261020/194-Profile-0.log') 'HC074-HELD-PHASE ')
    $taskLanes=@()
    foreach($taskLane in @('194','188')){
        $taskCheck=Json "check-$taskLane-exit.json";$taskCohort=Json "$taskLane-cohort.json"
        Require ($taskCheck.source -ceq $taskSource -and $taskCheck.lane -ceq $taskLane -and -not $taskCheck.failure -and $taskCheck.steps.Count -eq 11 -and @($taskCheck.steps|Where-Object exit -ne 0).Count -eq 0 -and $taskCheck.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check.ps1'))) 'Focused gate incomplete'
        foreach($taskStep in $taskCheck.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Check log changed'}
        $taskImages=@(Get-Content (Join-Path $PSScriptRoot "check-$taskLane-build.log")|Where-Object {$_.StartsWith('{')}|ForEach-Object {$_|ConvertFrom-Json}|Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
        Require ($taskImages.Count -eq 1 -and $taskImages[0].features.Count -eq 0) 'Default image required'
        Require ($taskCohort.source -ceq $taskSource -and $taskCohort.lane -ceq $taskLane -and $taskCohort.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'profile.ps1'))) 'Cohort identity changed'
        $taskStopped=$false;$taskProfiles=@();$taskRefusals=@();$taskStepRows=@()
        foreach($taskSample in 0..2){
            $taskPrefix="$taskLane-Profile-$taskSample";$taskExists=Test-Path (Join-Path $PSScriptRoot "$taskPrefix-exit.json")
            No-Later $taskStopped ($taskExists -or (Test-Path (Join-Path $PSScriptRoot "$taskPrefix.log")))
            if($taskStopped){continue}
            Require $taskExists 'Scheduled receipt missing'
            $taskReceipt=Json "$taskPrefix-exit.json"
            Binding $taskReceipt.source $taskSource $taskReceipt.image_sha256 $taskReceipt.image_after_sha256
            Require ($taskReceipt.source -ceq $taskSource -and $taskReceipt.lane -ceq $taskLane -and $taskReceipt.sample -eq $taskSample -and $taskReceipt.exit -ge 0 -and $taskReceipt.exit -le 255 -and $taskReceipt.image -ceq $taskImages[0].executable -and $taskReceipt.image_sha256 -cmatch '^[0-9a-f]{64}$' -and $taskReceipt.image_sha256 -ceq $taskReceipt.image_after_sha256 -and $taskReceipt.image_hash_exit -eq 0) 'Profile source/image changed'
            Require ($taskReceipt.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'profile.ps1')) -and $taskReceipt.log_sha256 -ceq (Hash (Join-Path $PSScriptRoot "$taskPrefix.log"))) 'Profile recipe/log changed'
            Inactive $taskReceipt @('allocator_probe','full_matrix','original_send','server','product','native_gate_open')
            $taskLog=Join-Path $PSScriptRoot "$taskPrefix.log"
            $taskOriginal=@(Rows $taskLog 'HC074-HELD-LIVE-ORIGINAL ');$taskPhases=@(Rows $taskLog 'HC074-HELD-PHASE ')
            if($taskReceipt.exit -ne 0){
                Require (@($taskOriginal|Where-Object returned -eq $true).Count -eq 0) 'Failed profile cannot claim healthy original'
                $taskRefusals+=@{sample=$taskSample;exit=$taskReceipt.exit;original=$taskOriginal;partial_phases=$taskPhases;complete_cost_sample=$false}
                $taskStopped=$true;continue
            }
            Require ($taskOriginal.Count -eq 1 -and $taskOriginal[0].returned -eq $true -and $null -eq $taskOriginal[0].kernel -and $null -eq $taskOriginal[0].causal.first_refusal -and @($taskOriginal[0].age.sites|Where-Object {$null -ne $_.first_refusal}).Count -eq 0) 'Original live refusal cannot become healthy'
            Require ($taskOriginal[0].causal.first.issued_boot -ceq $taskBoot) 'Profile boot mismatch'
            $taskText=Get-Content $taskLog -Raw
            Require ($taskText.Contains('case=healthy seed=0x074c051e actual_fixed=true actual_consuming=true original_bounds=true eof=true fence_retained=true custodians_terminal=true synthetic_images=true original_send=false completion=false server=false product=false')) 'Original terminal invariants missing'
            $taskOutputs=Validate-Phases $taskPhases $taskOld
            $taskSum=($taskOutputs.round_steps.wall_ns|Measure-Object -Sum).Sum
            foreach($taskPart in $taskOutputs.round_steps){
                $taskOutside=($taskPart.kernel.outside_children_ns|Measure-Object -Sum).Sum
                $taskStepRows+=@{sample=$taskSample;step=$taskPart.step;calls=$taskPart.calls;wall_ns=$taskPart.wall_ns;kernel_outside_children_ns=$taskOutside;outside_instrumented_kernel_ns=$taskPart.wall_ns-$taskOutside;share_percent=100*$taskPart.wall_ns/$taskSum;kernel=$taskPart.kernel;live=$taskPart.live;authentication_calls=$taskPart.authentication_calls;signature_calls=$taskPart.signature_calls}
            }
            $taskProfiles+=@{sample=$taskSample;original=$taskOriginal[0];phases=$taskPhases}
        }
        Require ($taskCohort.stopped_on_refusal -eq $taskStopped) 'Cohort stopping differs'
        $taskComplete=$taskProfiles.Count -eq 3 -and -not $taskStopped
        $taskRanges=if($taskComplete){@(foreach($taskStep in 0..7){$taskParts=@($taskStepRows|Where-Object step -eq $taskStep);@{step=$taskStep;samples=3;wall_ns=(Range $taskParts.wall_ns);share_percent=(Range $taskParts.share_percent);outside_instrumented_kernel_ns=(Range $taskParts.outside_instrumented_kernel_ns)}})}else{@()}
        $taskLanes+=@{lane=$taskLane;complete_profile_cohort=$taskComplete;profiles=$taskProfiles;refusals=$taskRefusals;complete_sample_steps=$taskStepRows;full_cohort_ranges=$taskRanges}
    }
    @{schema='validation-round-residual-074-v1';source=$taskSource;runtime_baseline='b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d';boot=$taskBoot;lanes=$taskLanes;complete_profile_cohort=@($taskLanes|Where-Object complete_profile_cohort -ne $true).Count -eq 0;preflight_toolchain_side_effect=$true;root_toolchain_cleanup_verified=$true;runtime_changed_this_step=$false;new_full_matrix_run=$false;product_gain_claimed=$false;historical_refusals_retained=$true;native_gate_open=$false;server=$false;product=$false}
}
function Inventory{@{schema='sha256-local-file-inventory-v1';files=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
if($Mode -ceq 'Audit'){
    $taskOldFile=Join-Path $PSScriptRoot '../validation-round-cost-20261020/194-Profile-0.log'
    $taskAudits=@()
    foreach($taskMutation in @('parent','missing','edge','scope')){
        $taskRows=@(Rows $taskOldFile 'HC074-HELD-PHASE ');$taskReference=@(Rows $taskOldFile 'HC074-HELD-PHASE ')
        Validate-Phases @($taskReference) @($taskReference)|Out-Null
        $taskRefused=$false;try{Validate-Phases $taskRows $taskReference $taskMutation|Out-Null}catch{$taskRefused=$true}
        Require $taskRefused "Accepted mutation: $taskMutation";$taskAudits+=@{mutation=$taskMutation;refused=$true}
    }
    $taskRefused=$false;try{No-Later $true $true}catch{$taskRefused=$true};Require $taskRefused 'Accepted continuation'
    $taskAudits+=@{mutation='continue-after-refusal';refused=$true}
    foreach($taskMutation in @('source','image','bundle')){
        Binding ('a'*40) ('a'*40) ('b'*64) ('b'*64)
        $taskRefused=$false
        try{
            if($taskMutation -ceq 'source'){Binding ('c'*40) ('a'*40) ('b'*64) ('b'*64)}
            elseif($taskMutation -ceq 'image'){Binding ('a'*40) ('a'*40) ('b'*64) ('d'*64)}
            else{
                $taskBundle=(Get-Content (Join-Path $PSScriptRoot '../validation-round-cost-20261020/inputs-before.json') -Raw|ConvertFrom-Json).bundle
                Bundle-Equal $taskBundle $taskBundle
                $taskChanged=($taskBundle|ConvertTo-Json -Depth 8)|ConvertFrom-Json
                $taskChanged[0].sha256='e'*64
                Bundle-Equal $taskChanged $taskBundle
            }
        }catch{$taskRefused=$true}
        Require $taskRefused "Accepted binding mutation: $taskMutation";$taskAudits+=@{mutation=$taskMutation;refused=$true}
    }
    @{mutations=$taskAudits;server=$false;product=$false}|ConvertTo-Json -Depth 8
    exit
}
$taskFacts=Facts
if($Mode -ceq 'Facts'){$taskFacts|ConvertTo-Json -Depth 70;exit}
if($Mode -ceq 'WriteSummary'){
    Require (-not(Test-Path (Join-Path $PSScriptRoot 'summary.json')) -and -not(Test-Path (Join-Path $PSScriptRoot 'inventory.json'))) 'Never replace summary'
    $taskFacts|ConvertTo-Json -Depth 70|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'summary.json')
}else{Require (Same $taskFacts (Json 'summary.json')) 'Summary changed'}
if($Mode -ceq 'Seal'){
    Require (-not(Test-Path (Join-Path $PSScriptRoot 'inventory.json'))) 'Never reseal'
    $taskFinal=Json 'final-check-exit.json'
    Require (-not $taskFinal.failure -and $taskFinal.steps.Count -eq 5 -and @($taskFinal.steps|Where-Object exit -ne 0).Count -eq 0) 'Final checks incomplete'
    foreach($taskStep in $taskFinal.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Final log changed'}
    Inventory|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'inventory.json')
}
if($Mode -ceq 'Review'){
    Require (Same (Inventory) (Json 'inventory.json')) 'Packet member changed'
    $taskFinal=Json 'final-check-exit.json'
    Require (-not $taskFinal.failure -and $taskFinal.steps.Count -eq 5 -and @($taskFinal.steps|Where-Object exit -ne 0).Count -eq 0 -and $taskFinal.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'final-check.ps1'))) 'Final checks incomplete'
    foreach($taskStep in $taskFinal.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Final log changed'}
}
@{mode=$Mode;integrity_verified=$true;complete_profile_cohort=$taskFacts.complete_profile_cohort;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
