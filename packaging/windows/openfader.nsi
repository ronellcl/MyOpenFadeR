; MyOpenFadeR for Windows: the program with its plugins, themes, mappings, ffmpeg and a Python for the Python plugins.
;
; Compiled by build-installer.ps1, which stages the payload and passes:
;   -DPAYLOAD=<folder>   what ends up next to openfader.exe
;   -DVERSION=<x.y.z>
;   -DOUTFILE=<setup.exe>
; and, for a no-UAC test build, -DPERUSER (installs under %LOCALAPPDATA%, used by the self-test of the build script).
;
; Layout it produces (the folders src/paths.rs looks in, next to the executable):
;   openfader.exe  *.dll  tools\ffmpeg.exe  tools\ffprobe.exe  python\  extension\plugin\  extension\skin\  mappings\
; `tools\` and `python\` go first in the PATH of the program and of the processes it starts (src/main.rs), so the video
; decoder and the Python plugins find ffmpeg and python without the installer touching the system PATH.

Unicode true
SetCompressor /SOLID lzma
SetCompressorDictSize 64

!define APP_NAME "MyOpenFadeR"
!define APP_EXE "openfader.exe"
!define APP_PUBLISHER "OpenRsG"
!define APP_URL "https://github.com/ronellcl/MyOpenFadeR"

!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!ifndef PAYLOAD
  !error "PAYLOAD is not set: run build-installer.ps1 (it stages the payload and calls makensis)."
!endif
!ifndef OUTFILE
  !define OUTFILE "MyOpenFadeR-${VERSION}-windows-x86_64-setup.exe"
!endif

Name "${APP_NAME} ${VERSION}"
OutFile "${OUTFILE}"
BrandingText "${APP_NAME} ${VERSION}"
ShowInstDetails show
ShowUninstDetails show

!ifdef PERUSER
  RequestExecutionLevel user
  InstallDir "$LOCALAPPDATA\Programs\OpenFadeR"
  !define REGROOT "HKCU"
  !define SHELLCTX "current"
!else
  RequestExecutionLevel admin
  InstallDir "$PROGRAMFILES64\OpenFadeR"
  !define REGROOT "HKLM"
  !define SHELLCTX "all"
!endif

InstallDirRegKey ${REGROOT} "Software\OpenFadeR" "InstallDir"

!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenFadeR"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "FileDescription" "${APP_NAME} installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "CompanyName" "${APP_PUBLISHER}"
VIAddVersionKey "LegalCopyright" "GPL-3.0"

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!define MUI_ICON "..\..\assets\openfader.ico"
!define MUI_UNICON "..\..\assets\openfader.ico"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE_3LINES
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "$(RUN_TEXT)"
!define MUI_FINISHPAGE_NOREBOOTSUPPORT

; Every text the wizard shows, in both languages (1034 Spanish, 1033 English: the ids those language files use).
LangString RUN_TEXT 1034 "Abrir MyOpenFadeR"
LangString RUN_TEXT 1033 "Run MyOpenFadeR"
LangString SEC_APP 1034 "MyOpenFadeR (programa, plugins, temas, mapeos, ffmpeg y Python)"
LangString SEC_APP 1033 "MyOpenFadeR (program, plugins, themes, mappings, ffmpeg and Python)"
LangString SEC_APP_DESC 1034 "El programa y todo lo que necesita: plugins (yt-dlp, importadores de rekordbox/Serato/Traktor/VirtualDJ/iTunes/Engine, Beat FX), temas, mapeos de controladoras, ffmpeg y Python para los plugins."
LangString SEC_APP_DESC 1033 "The program and everything it needs: plugins (yt-dlp, rekordbox/Serato/Traktor/VirtualDJ/iTunes/Engine importers, Beat FX), themes, controller mappings, ffmpeg and Python for the Python plugins."
LangString SEC_DESKTOP 1034 "Acceso directo en el escritorio"
LangString SEC_DESKTOP 1033 "Desktop shortcut"
LangString SEC_DESKTOP_DESC 1034 "Un icono de MyOpenFadeR en el escritorio."
LangString SEC_DESKTOP_DESC 1033 "A MyOpenFadeR icon on the desktop."
LangString UNINST_SHORTCUT 1034 "Desinstalar MyOpenFadeR"
LangString UNINST_SHORTCUT 1033 "Uninstall MyOpenFadeR"
LangString IN_USE 1034 "MyOpenFadeR está abierto y no se puede cerrar. Ciérralo y pulsa Reintentar."
LangString IN_USE 1033 "MyOpenFadeR is running and could not be closed. Close it and press Retry."
LangString UNINST_DATA 1034 "¿Borrar también tu configuración, biblioteca y add-ons descargados? Están en tu carpeta de usuario, no en la del programa.$\r$\n$\r$\nSí = borrar todo · No = conservar tu música y tus ajustes (recomendado)."
LangString UNINST_DATA 1033 "Also delete your settings, library and downloaded add-ons? They live in your user folder, not in the program folder.$\r$\n$\r$\nYes = delete everything · No = keep your music and settings (recommended)."

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\..\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; The languages come after every page: that is when the MUI collects the texts of each one.
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "English"

; --- the program and everything it carries ------------------------------------------------------------------------

Section "$(SEC_APP)" SEC_APP
  SectionIn RO
  SetShellVarContext ${SHELLCTX}

  ; A running copy holds openfader.exe and its plugins open: end it before anything is written. taskkill answers 128
  ; when there was nothing to close.
  close_app:
  nsExec::Exec 'taskkill /IM ${APP_EXE}'
  Pop $0
  ${If} $0 != 128
  ${AndIf} $0 != 0
    MessageBox MB_RETRYCANCEL|MB_ICONSTOP "$(IN_USE)" /SD IDCANCEL IDRETRY close_app
    Abort
  ${EndIf}

  SetOutPath "$INSTDIR"
  File /r "${PAYLOAD}\*.*"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${APP_EXE}" 0
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\$(UNINST_SHORTCUT).lnk" "$INSTDIR\uninstall.exe"

  WriteRegStr ${REGROOT} "Software\OpenFadeR" "InstallDir" "$INSTDIR"
  WriteRegStr ${REGROOT} "${UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr ${REGROOT} "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr ${REGROOT} "${UNINST_KEY}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr ${REGROOT} "${UNINST_KEY}" "URLInfoAbout" "${APP_URL}"
  WriteRegStr ${REGROOT} "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr ${REGROOT} "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr ${REGROOT} "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr ${REGROOT} "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD ${REGROOT} "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD ${REGROOT} "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $1 $2 $3
  IntFmt $1 "0x%08X" $1
  WriteRegDWORD ${REGROOT} "${UNINST_KEY}" "EstimatedSize" "$1"
SectionEnd

Section "$(SEC_DESKTOP)" SEC_DESKTOP
  SetShellVarContext ${SHELLCTX}
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${APP_EXE}" 0
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_APP} "$(SEC_APP_DESC)"
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_DESKTOP} "$(SEC_DESKTOP_DESC)"
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; --- removing it --------------------------------------------------------------------------------------------------

Section "Uninstall"
  SetShellVarContext ${SHELLCTX}
  nsExec::Exec 'taskkill /IM ${APP_EXE}'

  Delete "$SMPROGRAMS\${APP_NAME}\*.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
  Delete "$DESKTOP\${APP_NAME}.lnk"

  ; The program folder goes whole; the user's own folders are asked about below.
  RMDir /r "$INSTDIR"

  DeleteRegKey ${REGROOT} "${UNINST_KEY}"
  DeleteRegKey ${REGROOT} "Software\OpenFadeR"

  MessageBox MB_YESNO|MB_ICONQUESTION "$(UNINST_DATA)" /SD IDNO IDNO keep_data
  RMDir /r "$APPDATA\openfader"
  RMDir /r "$LOCALAPPDATA\openfader"
  keep_data:
SectionEnd
