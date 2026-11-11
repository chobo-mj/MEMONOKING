; Per-user, offline Windows x64 UI preview installer.
; Input is a strictly staged directory, never the source tree or user data.
#ifndef AppVersion
  #define AppVersion "0.3.0"
#endif
#ifndef ReleaseLabel
  #define ReleaseLabel "0.3.0-ui-preview.1"
#endif
#ifndef StageDir
  #define StageDir "..\dist\portable\OmniMemoJang"
#endif
#ifndef PackageOutputDir
  #define PackageOutputDir "..\dist"
#endif

[Setup]
AppId={{36497726-D8EF-4514-9BF2-51367779F831}
AppName=OmniMemoJang UI Preview
AppVersion={#AppVersion}
AppVerName=OmniMemoJang {#ReleaseLabel}
AppPublisher=OmniMemoJang
DefaultDirName={localappdata}\Programs\OmniMemoJang
DefaultGroupName=OmniMemoJang
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#PackageOutputDir}
OutputBaseFilename=OmniMemoJang-{#ReleaseLabel}-Windows-x64-Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\OmniMemoJang.exe
UninstallDisplayName=OmniMemoJang UI Preview
VersionInfoVersion={#AppVersion}
InfoBeforeFile={#StageDir}\README.txt
CloseApplications=yes
RestartApplications=no
SetupMutex=OmniMemoJangPreviewInstaller

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "바탕화면 바로가기 만들기"; GroupDescription: "바로가기:"; Flags: unchecked

[Files]
Source: "{#StageDir}\OmniMemoJang.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\SHA256SUMS.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\OmniMemoJang\OmniMemoJang"; Filename: "{app}\OmniMemoJang.exe"; WorkingDir: "{app}"
Name: "{userdesktop}\OmniMemoJang"; Filename: "{app}\OmniMemoJang.exe"; WorkingDir: "{app}"; Tasks: desktopicon

; No [Run], [Registry], download, startup, firewall, or security-policy actions.
; Inno Setup manages the current user's uninstall registration automatically.
