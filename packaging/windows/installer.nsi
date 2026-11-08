; Installeur de Bobine Deploy : par utilisateur, sans droits d'administrateur.
; makensis /DVERSION=<x.y.z> /DBINARY=<chemin de bobine-deploy.exe> /DOUTFILE=<installeur.exe> installer.nsi

Unicode true
SetCompressor /SOLID lzma
RequestExecutionLevel user

!define NAME "Bobine Deploy"
!define REGKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\BobineDeploy"

Name "${NAME}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\${NAME}"
Icon "..\..\crates\bobine-deploy\assets\icons\icon.ico"
UninstallIcon "..\..\crates\bobine-deploy\assets\icons\icon.ico"
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${NAME}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${NAME}"
VIAddVersionKey "LegalCopyright" "FantasmaGlad, AGPL-3.0-or-later"

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Install"
  SetOutPath "$INSTDIR"
  File "/oname=bobine-deploy.exe" "${BINARY}"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortCut "$SMPROGRAMS\${NAME}.lnk" "$INSTDIR\bobine-deploy.exe"
  WriteRegStr HKCU "${REGKEY}" "DisplayName" "${NAME}"
  WriteRegStr HKCU "${REGKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${REGKEY}" "Publisher" "FantasmaGlad"
  WriteRegStr HKCU "${REGKEY}" "DisplayIcon" "$INSTDIR\bobine-deploy.exe"
  WriteRegStr HKCU "${REGKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${REGKEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${REGKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${REGKEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\bobine-deploy.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${NAME}.lnk"
  DeleteRegKey HKCU "${REGKEY}"
SectionEnd
