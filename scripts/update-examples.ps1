$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    docker compose up -d --build dev
    if ($LASTEXITCODE -ne 0) { throw 'development container failed to start' }
    docker compose exec -T dev sh scripts/update-examples.sh
    if ($LASTEXITCODE -ne 0) { throw 'example regeneration failed' }
} finally { Pop-Location }
