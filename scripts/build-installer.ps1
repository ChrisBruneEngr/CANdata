# Build the CANdata installer
# Compiles the program and builds the installer with Inno Setup

param(
    [string]$Version = "1.0.0",
    [string]$InnoSetupPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "CANdata installer build script v$Version" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Check that Inno Setup is installed
Write-Host "📋 Step 1: Checking for Inno Setup..." -ForegroundColor Green
if (-not (Test-Path $InnoSetupPath)) {
    Write-Host "❌ Inno Setup not found!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Download and install Inno Setup:" -ForegroundColor Yellow
    Write-Host "  https://jrsoftware.org/isdl.php" -ForegroundColor White
    Write-Host ""
    Write-Host "Or pass the Inno Setup path:" -ForegroundColor Yellow
    Write-Host "  .\build-installer.ps1 -InnoSetupPath 'C:\Path\To\ISCC.exe'" -ForegroundColor White
    Write-Host ""
    exit 1
}
Write-Host "✅ Inno Setup found: $InnoSetupPath" -ForegroundColor Green
Write-Host ""

# 2. Build the release version
Write-Host "📦 Step 2: Building the release version..." -ForegroundColor Green
cargo build --release -p candata
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Build succeeded!" -ForegroundColor Green
Write-Host ""

# 3. Create the required folders
Write-Host "📁 Step 3: Preparing files..." -ForegroundColor Green

# Make sure the config folder exists
if (-not (Test-Path "config")) {
    New-Item -ItemType Directory -Path "config" -Force | Out-Null
}
if (-not (Test-Path "config\signal_library")) {
    New-Item -ItemType Directory -Path "config\signal_library" -Force | Out-Null
}

# Create the default config file
$defaultConfig = @"
{
  "libraries": [],
  "mappings": [],
  "active_library_id": null,
  "active_version_name": null
}
"@
$defaultConfig | Out-File -FilePath "config\default_config.json" -Encoding UTF8 -Force

Write-Host "✅ Files prepared!" -ForegroundColor Green
Write-Host ""

# 4. Build the installer
Write-Host "🔨 Step 4: Building the installer..." -ForegroundColor Green

# Update the version number
$issSource = Join-Path $PSScriptRoot "installer.iss"
$issContent = Get-Content $issSource -Raw
$issContent = $issContent -replace '#define MyAppVersion ".*"', "#define MyAppVersion `"$Version`""
$issContent | Out-File -FilePath "installer.iss" -Encoding UTF8 -Force

# Run the Inno Setup compiler
& $InnoSetupPath "installer.iss"
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Installer build failed!" -ForegroundColor Red
    exit 1
}

Write-Host "✅ Installer built!" -ForegroundColor Green
Write-Host ""

# 5. Done
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "✅ Build complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Installer location:" -ForegroundColor Yellow
Write-Host "  installer-output\candata-Setup-v$Version.exe" -ForegroundColor White
Write-Host ""
Write-Host "You can distribute this installer to users." -ForegroundColor Cyan
Write-Host ""

# Show file info
if (Test-Path "installer-output\candata-Setup-v$Version.exe") {
    $fileInfo = Get-Item "installer-output\candata-Setup-v$Version.exe"
    $fileSizeMB = [math]::Round($fileInfo.Length / 1MB, 2)
    Write-Host "File size: $fileSizeMB MB" -ForegroundColor White
    Write-Host "Created: $($fileInfo.CreationTime)" -ForegroundColor White
}
