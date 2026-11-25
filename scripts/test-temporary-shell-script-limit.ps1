#Requires -Version 7.0

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$fixtures = Join-Path $repo 'target'
$scratch = Join-Path $fixtures ('hydracache-shell-limit-' + [guid]::NewGuid().ToString('N'))
$tools = Join-Path $scratch 'tools'
$helper = Join-Path $tools 'Save-TemporaryShellScript.ps1'

function Assert-Rejected([scriptblock]$Action, [string]$ExpectedPattern) {
    try {
        & $Action | Out-Null
    } catch {
        if ($_.Exception.Message -notmatch $ExpectedPattern) { throw }
        return
    }
    throw "Expected rejection matching: $ExpectedPattern"
}

[void][IO.Directory]::CreateDirectory($tools)
try {
    Copy-Item -LiteralPath (Join-Path $repo 'tools/Save-TemporaryShellScript.ps1') -Destination $helper
    $storage = Join-Path $scratch '.tmp/generated-shell'
    $expectedPath = Join-Path $storage 'qualification/run.sh'
    $body = "printf 'hello'`r`n"
    $path = & $helper -Task qualification -Operation run -Content $body
    $actualBytes = [Convert]::ToHexString([IO.File]::ReadAllBytes($path))
    $expectedBytes = [Convert]::ToHexString([Text.Encoding]::UTF8.GetBytes("printf 'hello'`n"))
    if ($path -cne $expectedPath -or $actualBytes -cne $expectedBytes) {
        throw 'Incorrect path or script encoding'
    }
    if ((& $helper -Task qualification -Operation run -Content $body) -cne $path) {
        throw 'Repeating the same operation must reuse the script'
    }
    Assert-Rejected { & $helper -Task qualification -Operation run -Content 'true' } 'already has a script'

    foreach ($operation in @('prepare', 'freeze')) {
        & $helper -Task qualification -Operation $operation -Content 'true' | Out-Null
    }
    Assert-Rejected { & $helper -Task qualification -Operation extra -Content 'true' } 'already has 3 temporary scripts'
    for ($i = 1; $i -le 17; $i++) {
        & $helper -Task "test-$i" -Operation run -Content 'true' | Out-Null
    }
    Assert-Rejected { & $helper -Task extra -Operation run -Content 'true' } 'already has 20 temporary scripts'
    & $helper -Task qualification -Operation run -Content 'true' -Replace | Out-Null
    if ([IO.File]::ReadAllText($path) -cne 'true') { throw 'Explicit replacement failed at the limits' }

    Assert-Rejected { & $helper -Task '../escape' -Operation run -Content 'true' } '\.\./escape'
    Assert-Rejected { & $helper -Task con -Operation run -Content 'true' } 'Windows device names'
    $lock = [IO.File]::Open((Join-Path $storage '.write.lock'), 'Open', 'ReadWrite', 'None')
    try {
        Assert-Rejected { & $helper -Task qualification -Operation run -Content 'false' -Replace } 'locked or unavailable'
    } finally {
        $lock.Dispose()
    }
    if (@(Get-ChildItem -LiteralPath $storage -Recurse -File -Filter '*.sh').Count -ne 20 -or
        @(Get-ChildItem -LiteralPath $scratch -Force -File -Filter '*.sh').Count -ne 0) {
        throw 'Rejected operations or replacement changed the expected script count'
    }
    Write-Output 'Temporary shell script limit checks passed.'
} finally {
    # Remove only this test's isolated fixture.
    $resolved = (Resolve-Path -LiteralPath $scratch).ProviderPath
    if ($resolved -cne [IO.Path]::GetFullPath($scratch) -or
        (Split-Path -Parent $resolved) -cne $fixtures) {
        throw 'Unexpected temporary test directory'
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
