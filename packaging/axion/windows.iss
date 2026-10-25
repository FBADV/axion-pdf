; Axion Windows installer (Inno Setup 6). Defines come from the workflow:
;   AppTitle, AppSlug, AppExe, AppId, AppVersion, Icon, Staging (exe [+ models\]), Out
#ifndef AppTitle
  #define AppTitle "Axion"
#endif

[Setup]
AppId={#AppId}
AppName={#AppTitle}
AppVersion={#AppVersion}
AppVerName={#AppTitle} {#AppVersion}
AppPublisher=Bernardo Advogados
AppPublisherURL=https://bernardoadvogados.netlify.app
DefaultDirName={autopf}\{#AppTitle}
DefaultGroupName={#AppTitle}
DisableProgramGroupPage=yes
OutputDir={#Out}
OutputBaseFilename={#AppSlug}-{#AppVersion}-Windows-Setup
SetupIconFile={#Icon}
UninstallDisplayIcon={app}\{#AppExe}.exe
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ShowLanguageDialog=no

[Languages]
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#Staging}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppTitle}"; Filename: "{app}\{#AppExe}.exe"
Name: "{autodesktop}\{#AppTitle}"; Filename: "{app}\{#AppExe}.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}.exe"; Description: "{cm:LaunchProgram,{#AppTitle}}"; Flags: nowait postinstall skipifsilent
