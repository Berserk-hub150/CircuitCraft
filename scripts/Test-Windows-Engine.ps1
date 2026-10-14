$ErrorActionPreference = "Stop"
$ProjectDir = Split-Path $PSScriptRoot -Parent
$CliExe = Join-Path $ProjectDir "windows/circuitcraft-cli.exe"
$ReportPath = Join-Path $ProjectDir "verification/windows-engine-report.txt"
& $CliExe check-engine $ReportPath
if ($LASTEXITCODE -ne 0) { throw "Engine verification failed." }
$ProjectFile = Join-Path $ProjectDir "examples/rc-lowpass.cforge"
$SweepReport = Join-Path $ProjectDir "verification/windows-sweep.csv"
& $CliExe sweep $ProjectFile ac $SweepReport --component R1 --values "500,1k,2k"
if ($LASTEXITCODE -ne 0) { throw "Parameter sweep failed." }
$NoiseReport = Join-Path $ProjectDir "verification/windows-noise.csv"
& $CliExe simulate $ProjectFile noise $NoiseReport --output VOUT --source V1
if ($LASTEXITCODE -ne 0) { throw "Noise analysis failed." }
Write-Host "PASS. Report: $ReportPath"
