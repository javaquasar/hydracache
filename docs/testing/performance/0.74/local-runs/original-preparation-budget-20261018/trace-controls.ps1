$ErrorActionPreference='Stop'
$images=Get-Content (Join-Path $PSScriptRoot 'Candidate-images.json') -Raw | ConvertFrom-Json
foreach($image in $images){
    $hash=(& wsl -d Ubuntu -- sha256sum -- $image.image) -join "`n"
    if($LASTEXITCODE -ne 0 -or $hash -cne "$($image.sha256)  $($image.image)"){throw 'trace image drift'}
    & wsl -d Ubuntu -- $image.image validation_trace_tests:: --test-threads=1 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot "trace-controls-$($image.compiler).log")
    if($LASTEXITCODE -ne 0){throw 'trace controls failed'}
    $raw=Get-Content (Join-Path $PSScriptRoot "trace-controls-$($image.compiler).log") -Raw
    if(!$raw.Contains('test result: ok. 6 passed; 0 failed;')){throw 'trace control count'}
}
