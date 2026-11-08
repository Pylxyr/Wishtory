; Inno Setup script for the Windows installer. Built by .github/workflows/release.yml:
;   ISCC.exe /DAppVersion=0.3.0 /DSourceDir=<full path of the Flutter Release folder> /DOutputDir=<folder> wishtory.iss
; The installer is per user: it needs no administrator rights and installs under
; %LOCALAPPDATA%\Programs. Uninstalling removes the program only; saved history stays in
; %APPDATA%\app.wishtory.desktop.

#ifndef AppVersion
  #error Pass the version: /DAppVersion=x.y.z
#endif
#ifndef SourceDir
  #error Pass the folder to install: /DSourceDir=<full path of build\windows\x64\runner\Release>
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

[Setup]
; Never change this GUID: it is how Windows recognises one app across versions.
AppId={{0C0E10C8-A743-4385-892C-EF5184C19CBA}
AppName=Wishtory
AppVersion={#AppVersion}
AppPublisher=Wishtory contributors
AppPublisherURL=https://github.com/Pylxyr/Wishtory
AppSupportURL=https://github.com/Pylxyr/Wishtory/issues
DefaultDirName={autopf}\Wishtory
DefaultGroupName=Wishtory
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=Wishtory_{#AppVersion}_x64-setup
SetupIconFile=..\..\app\windows\runner\resources\app_icon.ico
UninstallDisplayIcon={app}\wishtory.exe
LicenseFile=..\..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{autoprograms}\Wishtory"; Filename: "{app}\wishtory.exe"
Name: "{autodesktop}\Wishtory"; Filename: "{app}\wishtory.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\wishtory.exe"; Description: "Start Wishtory"; Flags: nowait postinstall skipifsilent
