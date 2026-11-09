; CANdata installer script
; Build it with scripts\build-installer.ps1, which packages CANdata first and passes
; /DMyAppVersion and /DSourceDir (the packaged folder) to the Inno Setup 6 compiler.
;
; CANdata is portable and writes its settings and libraries next to candata.exe,
; so it installs per user (under %LOCALAPPDATA%\Programs) where that folder is writable.

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir SourcePath + "..\release-package\candata-v" + MyAppVersion
#endif

#define MyAppName "CANdata"
#define MyAppPublisher "Chris Brune"
#define MyAppURL "https://github.com/ChrisBruneEngr/CANdata"
#define MyAppExeName "candata.exe"

[Setup]
; AppId must stay the same across CANdata releases so upgrades work;
; it is deliberately different from the original can-viewer installer's AppId.
AppId={{E5F51502-5C94-4004-9DF9-4C943D37CB04}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile={#SourceDir}\LICENSE.txt
OutputDir={#SourcePath}..\installer-output
OutputBaseFilename=candata-Setup-v{#MyAppVersion}
SetupIconFile={#SourcePath}..\assets\ico\candata.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "{#SourceDir}\USAGE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\samples\*"; DestDir: "{app}\samples"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

; Settings, signal sets and libraries the user created next to candata.exe are
; left in place on uninstall, so reinstalling keeps the user's setup.
