# CANdata packaging script
#
# Builds the release executable and creates the portable package:
#
#   release-package\candata-v<Version>\       staging folder (also used by the installer)
#   release-package\candata-v<Version>.zip    portable ZIP
#
# CANdata is portable: it keeps its configuration, signal sets and library
# files next to candata.exe and creates them on first use, so the package
# only needs the executable, the docs and the samples.

param(
    [string]$Version = "1.0.0",
    [string]$OutputDir = ".\release-package"
)

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

Write-Host "CANdata packaging script v$Version" -ForegroundColor Cyan
Write-Host ""

# 1. Build the release executable
Write-Host "Step 1: Building candata.exe (release)..." -ForegroundColor Green
cargo build --release -p candata
if ($LASTEXITCODE -ne 0) { Write-Host "Build failed." -ForegroundColor Red; exit 1 }

# 2. Create a clean staging folder
Write-Host "Step 2: Creating the staging folder..." -ForegroundColor Green
$PackageName = "candata-v$Version"
$PackageDir = Join-Path $OutputDir $PackageName
if (Test-Path $PackageDir) { Remove-Item -Path $PackageDir -Recurse -Force }
New-Item -ItemType Directory -Path "$PackageDir\samples" -Force | Out-Null

# 3. Copy the executable and docs
Write-Host "Step 3: Copying the executable and docs..." -ForegroundColor Green
Copy-Item ".\target\release\candata.exe" "$PackageDir\candata.exe" -ErrorAction Stop
Copy-Item ".\LICENSE.txt" "$PackageDir\LICENSE.txt" -ErrorAction Stop
Copy-Item ".\docs\USAGE.md" "$PackageDir\USAGE.md" -ErrorAction Stop

# 4. Samples: the demo database plus a freshly generated matching BLF recording
Write-Host "Step 4: Creating the samples..." -ForegroundColor Green
Copy-Item ".\demo.dbc" "$PackageDir\samples\demo.dbc" -ErrorAction Stop
cargo run --release -q -p blf --bin gen_demo_blf -- "$PackageDir\samples\demo.blf"
if ($LASTEXITCODE -ne 0) { Write-Host "Generating demo.blf failed." -ForegroundColor Red; exit 1 }

# 5. Package readme
Write-Host "Step 5: Writing README.txt..." -ForegroundColor Green
$readme = @"
CANdata v$Version
=================

CANdata is an open-source CAN/LIN bus data analysis tool for Windows 10 and 11 (x64).
It reads Vector BLF log files, decodes signals with DBC/LDF databases and plots them.

Getting started
---------------
1. Run candata.exe.
2. Drag samples\demo.blf onto the window (or use File > Open BLF...).
3. Open Library, add a library with a version, and add channel 1 using samples\demo.dbc.
4. Switch to Plot to see the decoded signals.

The full user guide is in USAGE.md.

Where CANdata keeps its files
-----------------------------
CANdata is portable. Its settings (multi_channel_config.json), signal sets
(signal_sets.json) and library files (libraries\) are created next to candata.exe,
so keep this folder somewhere you can write to, and copy the whole folder to move
your setup. CANdata makes no network connections.

License
-------
MIT License; see LICENSE.txt. CANdata is based on can-viewer
(https://github.com/cantool/can-viewer).

Project page and issues: https://github.com/ChrisBruneEngr/CANdata
"@
$readme | Out-File -FilePath "$PackageDir\README.txt" -Encoding ASCII

# 6. Create the ZIP
Write-Host "Step 6: Creating the ZIP..." -ForegroundColor Green
$ZipPath = Join-Path $OutputDir "$PackageName.zip"
if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
Compress-Archive -Path $PackageDir -DestinationPath $ZipPath -CompressionLevel Optimal -ErrorAction Stop

Write-Host ""
Write-Host "Done." -ForegroundColor Green
Write-Host "  Folder: $PackageDir"
Write-Host "  ZIP:    $ZipPath"
