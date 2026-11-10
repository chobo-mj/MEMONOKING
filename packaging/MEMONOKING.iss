; Per-user, offline Windows x64 UI preview installer.
; Input is a strictly staged directory, never the source tree or user data.
#ifndef AppVersion
  #define AppVersion "0.2.0"
#endif
#ifndef ReleaseLabel
  #define ReleaseLabel "0.2.0-ui-preview.1"
#endif
#ifndef StageDir
  #define StageDir "..\dist\portable\MEMONOKING"
#endif
#ifndef PackageOutputDir
  #define PackageOutputDir "..\dist"
#endif

[Setup]
AppId={{36497726-D8EF-4514-9BF2-51367779F831}
AppName=MEMONOKING UI Preview
AppVersion={#AppVersion}
AppVerName=MEMONOKING {#ReleaseLabel}
AppPublisher=MEMONOKING
DefaultDirName={localappdata}\Programs\MEMONOKING
DefaultGroupName=MEMONOKING
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#PackageOutputDir}
OutputBaseFilename=MEMONOKING-{#ReleaseLabel}-Windows-x64-Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\MEMONOKING.exe
UninstallDisplayName=MEMONOKING UI Preview
VersionInfoVersion={#AppVersion}
InfoBeforeFile={#StageDir}\README.txt
CloseApplications=yes
RestartApplications=no
SetupMutex=MEMONOKINGPreviewInstaller

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "바탕화면 바로가기 만들기"; GroupDescription: "바로가기:"; Flags: unchecked

[Files]
Source: "{#StageDir}\MEMONOKING.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\SHA256SUMS.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\MEMONOKING\MEMONOKING"; Filename: "{app}\MEMONOKING.exe"; WorkingDir: "{app}"
Name: "{userdesktop}\MEMONOKING"; Filename: "{app}\MEMONOKING.exe"; WorkingDir: "{app}"; Tasks: desktopicon

; No [Run], [Registry], download, startup, firewall, or security-policy actions.
; Inno Setup manages the current user's uninstall registration automatically.
