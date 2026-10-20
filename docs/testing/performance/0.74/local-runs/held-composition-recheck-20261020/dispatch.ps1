# Recover only the recorded path-conversion refusal before first dispatch.
$ErrorActionPreference='Stop'
Push-Location (Resolve-Path "$PSScriptRoot/../../../../../..")
try {
    foreach($taskName in 'session.log','preflight.log','postflight.log','matrix-194.log','matrix-188.log','exits.txt','post.json','dispatch.json'){
        if(Test-Path "$PSScriptRoot/$taskName"){throw 'Already dispatched; never replay'}
    }
    $taskBinding=Get-Content -Raw "$PSScriptRoot/bindings.json"|ConvertFrom-Json
    $taskSelected=@('tools/long-run-supervisor-074','crates/xtask','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
    if((& git diff $taskBinding.source HEAD -- @taskSelected) -or (& git diff $taskBinding.image_source HEAD -- @taskSelected) -or (& git status --porcelain -- @taskSelected)){throw 'Source changed'}
    foreach($taskRecipe in @(@('run.ps1',$taskBinding.run_sha256),@('matrix.sh',$taskBinding.matrix_sha256))){
        if((Get-FileHash "$PSScriptRoot/$($taskRecipe[0])" -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskRecipe[1]){throw 'Original recipe changed'}
    }
    $taskPrior='docs/testing/performance/0.74/local-runs/pid-path-cost-20261020'
    if((Get-FileHash "$taskPrior/inventory.json" -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskBinding.prior_inventory_sha256){throw 'Prior inventory changed'}
    foreach($taskFile in (Get-Content -Raw "$taskPrior/inventory.json"|ConvertFrom-Json).files){
        if((Get-FileHash "$taskPrior/$($taskFile.path)" -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskFile.sha256){throw 'Prior member changed'}
    }
    function Bundle {
        $taskBundle=(Resolve-Path 'target/diagnostic-start-material-bundle-20261017').Path
        @(Get-ChildItem $taskBundle -File -Recurse|Sort-Object FullName|ForEach-Object{
            [ordered]@{path=$_.FullName.Substring($taskBundle.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
        })
    }
    if((ConvertTo-Json -Depth 6 -Compress -InputObject @(Bundle)) -cne (ConvertTo-Json -Depth 6 -Compress -InputObject @($taskBinding.bundle_before))){throw 'Bundle changed before dispatch'}
    [ordered]@{recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();continued_pre_dispatch_only=$true;runtime_changed=$false;server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/dispatch.json"
    & wsl.exe -d Ubuntu -u root -- /bin/bash docs/testing/performance/0.74/local-runs/held-composition-recheck-20261020/matrix.sh $taskBinding.images[0].path $taskBinding.images[0].sha256 $taskBinding.images[1].path $taskBinding.images[1].sha256 2>&1|Tee-Object "$PSScriptRoot/session.log"
    $taskExit=$LASTEXITCODE
    $taskAfter=Bundle
    [ordered]@{exit=$taskExit;bundle_after=$taskAfter;server=$false;product=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/post.json"
    if($taskExit -ne 0 -or (ConvertTo-Json -Depth 6 -Compress -InputObject $taskAfter) -cne (ConvertTo-Json -Depth 6 -Compress -InputObject @($taskBinding.bundle_before))){throw 'Postflight failed; retain all files, do not replay'}
} finally {Pop-Location}
