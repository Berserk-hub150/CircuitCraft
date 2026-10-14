$ErrorActionPreference = "Stop"
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    cargo build --workspace --profile dist
    if ($LASTEXITCODE -ne 0) { throw "Build failed." }
    New-Item -ItemType Directory -Force windows | Out-Null
    Copy-Item target/dist/circuitcraft.exe windows/
    Copy-Item target/dist/circuitcraft-cli.exe windows/
} finally { Pop-Location }
