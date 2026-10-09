# Windows PowerShell treats native stderr as an error when output is redirected.
# Native command failures are checked explicitly below.
$ErrorActionPreference = 'Continue'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    cargo build --locked
    if ($LASTEXITCODE -ne 0) { throw 'build failed' }
    $binary = Join-Path $PWD 'target/debug/planorama'
    if ($IsWindows -or $env:OS -eq 'Windows_NT') { $binary += '.exe' }
    foreach ($name in @('association','aws-relationships','checks','components','containment','dense-architecture','multi-container','network-scope','rds-subnet-group','routes','security-groups','spanning-dense','terraform')) {
        & $binary "tests/fixtures/$name-plan.json" -o "examples/$name.svg"
        if ($LASTEXITCODE -ne 0) { throw "render failed: $name" }
    }
    foreach ($pair in @(@('examples/plan.json','examples/diagram.svg'),@('examples/data-plan.json','examples/data.svg'),@('examples/bundling-plan.json','examples/bundling.svg'),@('examples/terraform-large/plan.json','examples/terraform-large/diagram.svg'))) {
        & $binary $pair[0] -o $pair[1]
        if ($LASTEXITCODE -ne 0) { throw "render failed: $($pair[0])" }
    }
    & $binary --state network=tests/fixtures/multi/network.json --state application=tests/fixtures/multi/application.json -o examples/multi-plan.svg
    if ($LASTEXITCODE -ne 0) { throw 'multi-plan render failed' }
    & $binary --state producer=tests/fixtures/multi/producer.json --state consumer=tests/fixtures/multi/consumer.json --remote-state consumer:data.terraform_remote_state.network=producer -o examples/cross-state.svg
    if ($LASTEXITCODE -ne 0) { throw 'cross-state render failed' }
} finally { Pop-Location }
