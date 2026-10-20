$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path "$PSScriptRoot/../../../../../..").Path
Push-Location $taskRepo
try {
    $taskSource='3ffa6e91b0ab0b03b82206a2dc3760c752679092'
    $taskImageSource='b5f1b8fce6b3046239087873cb85ff1bfc557f0f'
    $taskSelected=@('tools/long-run-supervisor-074','crates/xtask','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
    if((& git diff $taskSource HEAD -- @taskSelected) -or (& git diff $taskImageSource HEAD -- @taskSelected) -or (& git status --porcelain -- @taskSelected)){throw 'Frozen source changed'}
    if(Test-Path "$PSScriptRoot/bindings.json"){throw 'Never replay this cohort'}
    $taskPrior='docs/testing/performance/0.74/local-runs/pid-path-cost-20261020'
    $taskSeal='982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05'
    function Prior {
        if((Get-FileHash "$taskPrior/inventory.json" -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskSeal){throw 'Prior inventory changed'}
        $taskInventory=Get-Content -Raw "$taskPrior/inventory.json"|ConvertFrom-Json
        foreach($taskFile in $taskInventory.files){
            if((Get-FileHash "$taskPrior/$($taskFile.path)" -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskFile.sha256){throw 'Prior member changed'}
        }
    }
    function Bundle {
        $taskBundle='target/diagnostic-start-material-bundle-20261017'
        @(Get-ChildItem $taskBundle -File -Recurse|Sort-Object FullName|ForEach-Object{
            [ordered]@{path=$_.FullName.Substring((Resolve-Path $taskBundle).Path.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
        })
    }
    Prior
    $taskImages=@('194','188')|ForEach-Object {
        $taskImage=Get-Content -Raw "$taskPrior/Candidate-$_-plain-image.json"|ConvertFrom-Json
        if($taskImage.source -cne $taskImageSource -or $taskImage.flavor -cne 'plain'){throw 'Wrong image source/flavor'}
        $taskImage
    }
    $taskBefore=Bundle
    if($taskBefore.Count -eq 0){throw 'Missing bundle'}
    [ordered]@{source=$taskSource;image_source=$taskImageSource;images=$taskImages;bundle_before=$taskBefore;prior_inventory_sha256=$taskSeal;run_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();matrix_sha256=(Get-FileHash "$PSScriptRoot/matrix.sh" -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/bindings.json"
    $taskLinux=(& wsl.exe -d Ubuntu -- wslpath -a $PSScriptRoot).Trim()
    & wsl.exe -d Ubuntu -u root -- /bin/bash "$taskLinux/matrix.sh" $taskImages[0].path $taskImages[0].sha256 $taskImages[1].path $taskImages[1].sha256 2>&1|Tee-Object "$PSScriptRoot/session.log"
    $taskExit=$LASTEXITCODE
    $taskAfter=Bundle
    [ordered]@{exit=$taskExit;bundle_after=$taskAfter;server=$false;product=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/post.json"
    Prior
    if($taskExit -ne 0 -or (ConvertTo-Json -Depth 6 -Compress -InputObject $taskBefore) -cne (ConvertTo-Json -Depth 6 -Compress -InputObject $taskAfter)){throw 'Postflight failed; retain all files, do not replay'}
} finally {Pop-Location}
