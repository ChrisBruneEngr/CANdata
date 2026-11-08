; CANdata installer script
; Compile with Inno Setup 6 to build the installer.

#define MyAppName "CANdata"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Chris Brune"
#define MyAppURL "https://github.com/ChrisBruneEngr/CANdata"
#define MyAppExeName "candata.exe"

[Setup]
; Basic information. AppId must stay the same across CANdata releases so upgrades work;
; it is deliberately different from the original can-viewer installer's AppId.
AppId={{E5F51502-5C94-4004-9DF9-4C943D37CB04}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=LICENSE.txt
OutputDir=installer-output
OutputBaseFilename=candata-Setup-v{#MyAppVersion}
SetupIconFile=assets\ico\candata.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

; Uninstall information
UninstallDisplayIcon={app}\bin\{#MyAppExeName}
UninstallDisplayName={#MyAppName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "quicklaunchicon"; Description: "{cm:CreateQuickLaunchIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked; OnlyBelowVersion: 6.1; Check: not IsAdminInstallMode

[Files]
; Main program
Source: "target\release\candata.exe"; DestDir: "{app}\bin"; DestName: "{#MyAppExeName}"; Flags: ignoreversion

; Configuration folder
Source: "config\*"; DestDir: "{app}\config"; Flags: ignoreversion recursesubdirs createallsubdirs

; Sample files
Source: "sample.dbc"; DestDir: "{app}\samples"; Flags: ignoreversion; Check: FileExists('sample.dbc')
; sample.blf is not included in the repository

; Documentation
Source: "README.md"; DestDir: "{app}\docs"; Flags: ignoreversion isreadme; Check: FileExists('README.md')


; Resource files
Source: "assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs; Check: DirExists('assets')

; NOTE: Don't use "Flags: ignoreversion" on any shared system files

[Dirs]
; Create required folders
Name: "{app}\config\signal_library"; Permissions: users-modify
Name: "{app}\logs"; Permissions: users-modify
Name: "{userappdata}\{#MyAppName}"; Permissions: users-modify

[Icons]
; Start menu shortcut
Name: "{group}\{#MyAppName}"; Filename: "{app}\bin\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

; Desktop shortcut
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\bin\{#MyAppExeName}"; Tasks: desktopicon

; Quick Launch shortcut
Name: "{userappdata}\Microsoft\Internet Explorer\Quick Launch\{#MyAppName}"; Filename: "{app}\bin\{#MyAppExeName}"; Tasks: quicklaunchicon

[Run]
; Offer to start the program when setup finishes
Filename: "{app}\bin\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
// Custom setup logic

// Find the uninstaller of an existing installation, if any
function GetUninstallString(): String;
var
  sUnInstPath: String;
  sUnInstallString: String;
begin
  sUnInstPath := ExpandConstant('Software\Microsoft\Windows\CurrentVersion\Uninstall\{#emit SetupSetting("AppId")}_is1');
  sUnInstallString := '';
  if not RegQueryStringValue(HKLM, sUnInstPath, 'UninstallString', sUnInstallString) then
    RegQueryStringValue(HKCU, sUnInstPath, 'UninstallString', sUnInstallString);
  Result := sUnInstallString;
end;

// Uninstall the previous version
function IsUpgrade(): Boolean;
begin
  Result := (GetUninstallString() <> '');
end;

function UnInstallOldVersion(): Integer;
var
  sUnInstallString: String;
  iResultCode: Integer;
begin
  Result := 0;
  sUnInstallString := GetUninstallString();
  if sUnInstallString <> '' then begin
    sUnInstallString := RemoveQuotes(sUnInstallString);
    if Exec(sUnInstallString, '/SILENT /NORESTART /SUPPRESSMSGBOXES','', SW_HIDE, ewWaitUntilTerminated, iResultCode) then
      Result := 3
    else
      Result := 2;
  end else
    Result := 1;
end;

// Upgrade handling and default configuration
procedure CurStepChanged(CurStep: TSetupStep);
var
  ConfigFile: String;
  DefaultConfig: String;
begin
  // 1. Before installing, remove an existing version
  if (CurStep = ssInstall) then
  begin
    if (IsUpgrade()) then
    begin
      UnInstallOldVersion();
    end;
  end;

  // 2. After installing, create a default config file
  if CurStep = ssPostInstall then
  begin
    ConfigFile := ExpandConstant('{app}\config\default_config.json');
    if not FileExists(ConfigFile) then
    begin
      DefaultConfig := '{' + #13#10 +
        '  "libraries": [],' + #13#10 +
        '  "mappings": [],' + #13#10 +
        '  "active_library_id": null,' + #13#10 +
        '  "active_version_name": null' + #13#10 +
        '}';
      SaveStringToFile(ConfigFile, DefaultConfig, False);
    end;
  end;
end;
