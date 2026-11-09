# Build the CANdata installer
#
# Packages CANdata with package.ps1 (unless the staging folder already exists)
# and compiles scripts\installer.iss with Inno Setup 6. Output:
#
#   installer-output\candata-Setup-v<Version>.exe

param(
    [string]$Version = "1.0.0",
    [string]$InnoSetupPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
)

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

Write-Host "CANdata installer build script v$Version" -ForegroundColor Cyan
Write-Host ""

# 1. Check for Inno Setup
Write-Host "Step 1: Checking for Inno Setup..." -ForegroundColor Green
if (-not (Test-Path $InnoSetupPath)) {
    Write-Host "Inno Setup not found at $InnoSetupPath" -ForegroundColor Red
    Write-Host "Install it from https://jrsoftware.org/isdl.php or pass -InnoSetupPath." -ForegroundColor Yellow
    exit 1
}

# 2. Make sure the packaged files exist
$StagingDir = Join-Path $RepoRoot "release-package\candata-v$Version"
if (-not (Test-Path (Join-Path $StagingDir "candata.exe"))) {
    Write-Host "Step 2: Packaging (package.ps1)..." -ForegroundColor Green
    & (Join-Path $PSScriptRoot "package.ps1") -Version $Version
    if ($LASTEXITCODE -ne 0) { exit 1 }
} else {
    Write-Host "Step 2: Using the existing package in $StagingDir" -ForegroundColor Green
}

# 3. Compile the installer
Write-Host "Step 3: Compiling the installer..." -ForegroundColor Green
& $InnoSetupPath "/DMyAppVersion=$Version" "/DSourceDir=$StagingDir" (Join-Path $PSScriptRoot "installer.iss")
if ($LASTEXITCODE -ne 0) { Write-Host "Installer build failed." -ForegroundColor Red; exit 1 }

$Installer = Join-Path $RepoRoot "installer-output\candata-Setup-v$Version.exe"
Write-Host ""
Write-Host "Done: $Installer" -ForegroundColor Green
if (Test-Path $Installer) {
    $sizeMB = [math]::Round((Get-Item $Installer).Length / 1MB, 2)
    Write-Host "  Size: $sizeMB MB"
}
