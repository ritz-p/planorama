$ErrorActionPreference = 'Stop'
$plan = Get-Content (Join-Path $PSScriptRoot 'raw.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$plan.PSObject.Properties.Remove('timestamp')
$text = ($plan | ConvertTo-Json -Depth 100) -replace "`r`n", "`n"
if ($text.Contains('fixture-only')) { throw 'Dummy credentials unexpectedly present in capture' }
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'plan.json'), $text + "`n", (New-Object Text.UTF8Encoding($false)))
