$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskTokens=$null;$taskErrors=$null
$taskAst=[System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'launch.ps1'),[ref]$taskTokens,[ref]$taskErrors)
if($taskErrors.Count){throw 'Launcher parse failed'}
$taskAssignments=@($taskAst.FindAll({param($Node) $Node -is [System.Management.Automation.Language.AssignmentStatementAst] -and $Node.Left.Extent.Text -ceq '$taskPins'},$true))
if($taskAssignments.Count -ne 1){throw 'One pins assignment required'}
$Source='a'*40
$taskPreparation=@{'194'=@{sha256=('b'*64)};'188'=@{sha256=('c'*64)}}
$taskPrior='docs/testing/performance/0.74/local-runs/validation-round-residual-20261021'
$taskWslRepo='/synthetic-local-control'
# Exercise the actual source expression with synthetic values; no WSL launch.
$taskPins=& ([scriptblock]::Create($taskAssignments[0].Right.Extent.Text))
$taskEncoded=ConvertTo-Json -InputObject $taskPins -Depth 8
$taskDecoded=$taskEncoded|ConvertFrom-Json
if($taskDecoded.images.'194' -cne ('b'*64) -or $taskDecoded.images.'188' -cne ('c'*64) -or $taskDecoded.source -cne $Source -or $taskDecoded.bundle.Count -ne 9){throw 'Pins serialization changed'}
$taskRefused=$false
try{ConvertTo-Json -InputObject @{images=@{194='invalid-numeric-key'}} -ErrorAction Stop|Out-Null}catch{$taskRefused=$true}
if(-not $taskRefused){throw 'Original serialization canary was not refused'}
Write-Output 'launcher-controls: 2 passed (actual pins expression; original numeric-key refusal)'
