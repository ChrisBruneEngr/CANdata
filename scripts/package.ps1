# CANdata packaging script
# Builds a complete release package with the executable, config files and docs

param(
    [string]$Version = "1.0.0",
    [string]$OutputDir = ".\release-package"
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "CANdata packaging script v$Version" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Build the release version
Write-Host "📦 Step 1: Building the release version..." -ForegroundColor Green
cargo build --release -p candata
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Build succeeded!" -ForegroundColor Green
Write-Host ""

# 2. Create the release folder structure
Write-Host "📁 Step 2: Creating the release folder..." -ForegroundColor Green
$PackageName = "candata-v$Version"
$PackageDir = Join-Path $OutputDir $PackageName

# Remove the old release folder
if (Test-Path $PackageDir) {
    Remove-Item -Path $PackageDir -Recurse -Force
}

# Create the folder structure
New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null
New-Item -ItemType Directory -Path "$PackageDir\bin" -Force | Out-Null
New-Item -ItemType Directory -Path "$PackageDir\config" -Force | Out-Null
New-Item -ItemType Directory -Path "$PackageDir\config\signal_library" -Force | Out-Null
New-Item -ItemType Directory -Path "$PackageDir\docs" -Force | Out-Null
New-Item -ItemType Directory -Path "$PackageDir\samples" -Force | Out-Null
New-Item -ItemType Directory -Path "$PackageDir\assets" -Force | Out-Null

Write-Host "✅ Folder structure created!" -ForegroundColor Green
Write-Host ""

# 3. Copy the executable
Write-Host "📋 Step 3: Copying the executable..." -ForegroundColor Green
Copy-Item -Path ".\target\release\candata.exe" -Destination "$PackageDir\bin\candata.exe"
Write-Host "✅ Executable copied to bin\candata.exe" -ForegroundColor Green
Write-Host ""

# 4. Copy the config files
Write-Host "📋 Step 4: Copying config files..." -ForegroundColor Green

# Create the default config file
$defaultConfig = @"
{
  "libraries": [],
  "mappings": [],
  "active_library_id": null,
  "active_version_name": null
}
"@
$defaultConfig | Out-File -FilePath "$PackageDir\config\default_config.json" -Encoding UTF8

# If a user config exists, copy it as an example too
if (Test-Path ".\multi_channel_config.json") {
    Copy-Item -Path ".\multi_channel_config.json" -Destination "$PackageDir\config\example_config.json"
}

# Create the signal library storage readme
$signalLibraryReadme = @"
# Signal library local storage folder

This folder stores the signal libraries' database files.

## Folder structure

```
signal_library/
└── {library}/
    └── {version}/
        └── database.{dbc|ldf}
```

## Example

```
signal_library/
├── BMW_PTCAN/
│   ├── v1.0/
│   │   └── database.dbc
│   └── v2.0/
│       └── database.dbc
└── Ford_LIN/
    └── v1.5/
        └── database.ldf
```

## Notes

- When you add a signal library and version in the program, the database files are copied here automatically
- The config file stores paths inside this folder, which keeps the program portable
- You can also place database files into this folder structure by hand

## Caution

- Don't delete files in this folder unless you're sure they're no longer needed
- When backing up the program, back up this folder too

---
Updated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
"@
$signalLibraryReadme | Out-File -FilePath "$PackageDir\config\signal_library\README.txt" -Encoding UTF8

Write-Host "✅ Config files created" -ForegroundColor Green
Write-Host ""

# 5. Copy the sample files
Write-Host "📋 Step 5: Copying sample files..." -ForegroundColor Green
if (Test-Path ".\sample.dbc") {
    Copy-Item -Path ".\sample.dbc" -Destination "$PackageDir\samples\sample.dbc"
}
if (Test-Path ".\sample.blf") {
    Copy-Item -Path ".\sample.blf" -Destination "$PackageDir\samples\sample.blf"
}
Write-Host "✅ Sample files copied" -ForegroundColor Green
Write-Host ""

# 6. Copy the resource files
Write-Host "📋 Step 6: Copying resource files..." -ForegroundColor Green
if (Test-Path ".\assets") {
    Copy-Item -Path ".\assets\*" -Destination "$PackageDir\assets\" -Recurse -Force
}
Write-Host "✅ Resource files copied" -ForegroundColor Green
Write-Host ""

# 7. Copy the docs
Write-Host "📋 Step 7: Copying docs..." -ForegroundColor Green
Copy-Item -Path ".\README.md" -Destination "$PackageDir\docs\README.md" -ErrorAction SilentlyContinue
Copy-Item -Path ".\BUILD.md" -Destination "$PackageDir\docs\BUILD.md" -ErrorAction SilentlyContinue
Copy-Item -Path ".\ADD_CHANNEL_CRASH_FIX.md" -Destination "$PackageDir\docs\ADD_CHANNEL_CRASH_FIX.md" -ErrorAction SilentlyContinue

Write-Host "✅ Docs copied" -ForegroundColor Green
Write-Host ""

# 8. Create the launch script
Write-Host "📋 Step 8: Creating the launch script..." -ForegroundColor Green

$launchScript = @"
@echo off
REM CANdata launch script
echo Starting CANdata...
cd /d "%~dp0"
bin\candata.exe
pause
"@
$launchScript | Out-File -FilePath "$PackageDir\start.bat" -Encoding ASCII

Write-Host "✅ Launch script created" -ForegroundColor Green
Write-Host ""

# 9. Create the README
Write-Host "📋 Step 9: Creating release notes..." -ForegroundColor Green

$releaseReadme = @"
# CANdata v$Version

## Folder structure

```
candata-v$Version/
├── bin/              # executable
│   └── candata.exe   # main program
├── config/           # config folder
│   ├── signal_library/        # signal library local storage
│   │   └── README.txt         # storage notes
│   ├── default_config.json    # default config
│   └── example_config.json    # example config
├── samples/          # sample files
│   ├── sample.dbc    # DBC sample
│   └── sample.blf    # BLF sample
├── assets/           # resource files (icons, etc.)
├── docs/             # documentation
│   ├── README.md     # usage notes
│   └── BUILD.md      # build notes
├── start.bat         # launch script
└── README.txt        # this file
```

## Quick start

1. Double-click `start.bat` to start the program
2. Or run `bin\candata.exe` directly

## Config file

The program looks for its config file in:
1. `multi_channel_config.json` in the current folder
2. `config\default_config.json`

To customize the config, copy `config\example_config.json` to the program folder and rename it `multi_channel_config.json`.

## Signal library storage

The program stores signal libraries locally:

- **Location**: `config\signal_library\`
- **Managed automatically**: when you add a signal library, its database files are copied here
- **Folder structure**: `{library}\{version}\database.{dbc|ldf}`
- **Portable**: the whole program folder can be copied elsewhere and used as is

### How to use

1. Switch to the "Library" view
2. Click "+ Add Library" to create a library
3. Click "+ Add Version" to add a version
4. Click "+ Add Channel" to configure a channel and choose its database file
5. The file is copied to `config\signal_library\{library}\{version}\` automatically

## Features

- BLF file parsing and viewing
- DBC/LDF database support
- Multi-channel configuration
- Signal decoding and display
- Signal plotting

## System requirements

- Windows 10 or later
- 64-bit operating system

## Support

For help, see the docs in the `docs` folder or open an issue at https://github.com/ChrisBruneEngr/CANdata/issues.

---
Built: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Version: $Version
"@
$releaseReadme | Out-File -FilePath "$PackageDir\README.txt" -Encoding UTF8

Write-Host "✅ Release notes created" -ForegroundColor Green
Write-Host ""

# 10. Create the ZIP archive
Write-Host "📦 Step 10: Creating the ZIP archive..." -ForegroundColor Green
$ZipPath = Join-Path $OutputDir "$PackageName.zip"
if (Test-Path $ZipPath) {
    Remove-Item -Path $ZipPath -Force
}

Compress-Archive -Path $PackageDir -DestinationPath $ZipPath -CompressionLevel Optimal
Write-Host "✅ ZIP archive created: $ZipPath" -ForegroundColor Green
Write-Host ""

# Done
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "✅ Packaging complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Release package location:" -ForegroundColor Yellow
Write-Host "  Folder: $PackageDir" -ForegroundColor White
Write-Host "  ZIP: $ZipPath" -ForegroundColor White
Write-Host ""
Write-Host "Distribute the ZIP to users; they unzip it and run the program." -ForegroundColor Cyan
