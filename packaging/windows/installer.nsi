; CyberLexi Windows installer (NSIS / MUI2).
; Compiled in CI: makensis /INPUTCHARSET UTF8 /DVERSION=1.0.0 /DVERSION_NUMERIC=1.0.0.0 \
;                 /DSTAGE=<abs stage dir> /DICON=<abs .ico> /DOUTFILE=<abs setup.exe> \
;                 packaging\windows\installer.nsi
; (Script is UTF-8: the .onInit MessageBox carries Chinese text.)
; The stage dir must contain cyberlexi.exe and assets\ (dict + fonts).
; Per-user install (no UAC). User data (data\) is NOT removed on uninstall.

Unicode true

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef VERSION_NUMERIC
  !define VERSION_NUMERIC "0.0.0.0"
!endif
!ifndef STAGE
  !define STAGE "stage"
!endif
!ifndef ICON
  !define ICON "..\icon\cyberlexi.ico"
!endif
!ifndef OUTFILE
  !define OUTFILE "cyberlexi-${VERSION}-windows-x64-setup.exe"
!endif

!define PRODUCT_NAME "CyberLexi"
!define PRODUCT_EXE "cyberlexi.exe"
!define PRODUCT_PUBLISHER "CyberLexi"
!define PRODUCT_WEB_SITE "https://github.com/Lin-H/CyberLexi"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

Name "${PRODUCT_NAME} ${VERSION}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\${PRODUCT_NAME}"
InstallDirRegKey HKCU "Software\${PRODUCT_NAME}" "InstallDir"
RequestExecutionLevel user
ShowInstDetails show
ShowUnInstDetails show

VIProductVersion "${VERSION_NUMERIC}"
VIAddVersionKey "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey "FileDescription" "${PRODUCT_NAME} Installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "MIT License"

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${PRODUCT_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${PRODUCT_NAME}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

; Auto-update flow: the app launches this installer and then exits. A running
; exe cannot be opened for write (sharing violation), so probe the file lock
; and wait up to 20s for the old process to release it before overwriting.
Function .onInit
  IfFileExists "$INSTDIR\${PRODUCT_EXE}" 0 done
  StrCpy $R0 0
  wait:
    ClearErrors
    FileOpen $R1 "$INSTDIR\${PRODUCT_EXE}" a
    IfErrors still_running
    FileClose $R1
    Goto done
  still_running:
    IntOp $R0 $R0 + 1
    IntCmp $R0 40 ask continue continue
  continue:
    Sleep 500
    Goto wait
  ask:
    MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "CyberLexi is still running. Please close it, then click Retry.$\n检测到 CyberLexi 仍在运行，请先关闭再点击重试。" IDRETRY reset
    Quit
  reset:
    StrCpy $R0 0
    Goto wait
  done:
FunctionEnd

Section "${PRODUCT_NAME} (required)" SEC_MAIN
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "${STAGE}\${PRODUCT_EXE}"
  File /r "${STAGE}\assets"

  WriteRegStr HKCU "Software\${PRODUCT_NAME}" "InstallDir" "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Add/Remove Programs (per-user)
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKCU "${UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${PRODUCT_EXE}"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" "$\"$INSTDIR\uninstall.exe$\" /S"
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" "$0"

  ; Start Menu shortcuts
  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}" "" "$INSTDIR\${PRODUCT_EXE}" 0
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall ${PRODUCT_NAME}.lnk" "$INSTDIR\uninstall.exe"
SectionEnd

Section /o "Desktop shortcut" SEC_DESKTOP
  CreateShortCut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}" "" "$INSTDIR\${PRODUCT_EXE}" 0
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_MAIN} "Install ${PRODUCT_NAME} (executable + offline dictionaries + fonts)."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_DESKTOP} "Create a shortcut on the desktop."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section "Uninstall"
  ; Remove program files only. The portable data\ dir (wordbook / config /
  ; audio cache) is deliberately preserved; RMDir without /r leaves it intact.
  RMDir /r "$INSTDIR\assets"
  Delete "$INSTDIR\${PRODUCT_EXE}"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk"
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall ${PRODUCT_NAME}.lnk"
  RMDir "$SMPROGRAMS\${PRODUCT_NAME}"
  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"

  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "Software\${PRODUCT_NAME}"
SectionEnd