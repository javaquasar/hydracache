#Requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidatePattern('\A[a-z0-9][a-z0-9-]{0,47}\z')]
    [string]$Task,

    [Parameter(Mandatory)]
    [ValidatePattern('\A[a-z0-9][a-z0-9-]{0,47}\z')]
    [string]$Operation,

    [Parameter(Mandatory)]
    [string]$Content,

    [switch]$Replace
)

$ErrorActionPreference = 'Stop'
$taskLimit = 3
$workspaceLimit = 20
$reservedName = '\A(?:con|prn|aux|nul|com[1-9]|lpt[1-9])\z'
if ($Task -match $reservedName -or $Operation -match $reservedName) {
    throw 'Task and operation names cannot be Windows device names.'
}

function Assert-OrdinaryPath([string]$Path, [bool]$Directory) {
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    if ($item -and (
        $item.PSIsContainer -ne $Directory -or
        ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
    )) {
        throw "Unexpected file type or link: $Path"
    }
}

$workspace = Split-Path -Parent $PSScriptRoot
$scratch = Join-Path $workspace '.tmp'
$storage = Join-Path $scratch 'generated-shell'
Assert-OrdinaryPath $scratch $true
Assert-OrdinaryPath $storage $true
[void][IO.Directory]::CreateDirectory($storage)

$lockPath = Join-Path $storage '.write.lock'
Assert-OrdinaryPath $lockPath $false
try {
    $lock = [IO.File]::Open($lockPath, 'OpenOrCreate', 'ReadWrite', 'None')
} catch [IO.IOException] {
    throw 'The temporary script directory is locked or unavailable. Retry after the current write.'
}

try {
    $taskDirectory = Join-Path $storage $Task.ToLowerInvariant()
    $scriptPath = Join-Path $taskDirectory ($Operation.ToLowerInvariant() + '.sh')
    Assert-OrdinaryPath $taskDirectory $true
    Assert-OrdinaryPath $scriptPath $false

    $body = $Content -replace '\r\n?', "`n"
    if (Test-Path -LiteralPath $scriptPath) {
        if ([IO.File]::ReadAllText($scriptPath) -ceq $body) {
            return $scriptPath
        }
        if (!$Replace) {
            throw "The operation already has a script: $scriptPath. Reuse its name with -Replace."
        }
    } else {
        if ((Test-Path -LiteralPath $taskDirectory) -and
            @(Get-ChildItem -LiteralPath $taskDirectory -Force -File -Filter '*.sh').Count -ge $taskLimit) {
            throw "Task '$Task' already has $taskLimit temporary scripts. Reuse an operation name."
        }
        if (@(Get-ChildItem -LiteralPath $storage -Force -Recurse -File -Filter '*.sh').Count -ge $workspaceLimit) {
            throw "The workspace already has $workspaceLimit temporary scripts. Reuse an operation name."
        }
    }

    [void][IO.Directory]::CreateDirectory($taskDirectory)
    $pending = Join-Path $taskDirectory ([IO.Path]::GetRandomFileName() + '.partial')
    try {
        [IO.File]::WriteAllText($pending, $body, [Text.UTF8Encoding]::new($false))
        [IO.File]::Move($pending, $scriptPath, $Replace.IsPresent)
    } finally {
        if ([IO.File]::Exists($pending)) {
            [IO.File]::Delete($pending)
        }
    }
    $scriptPath
} finally {
    $lock.Dispose()
}
