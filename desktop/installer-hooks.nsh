; Keep the old installation directory when the default new directory was selected.
; No user data or installed files are deleted by this migration.
!macro NSIS_HOOK_PREINSTALL
  Push $R0
  Push $R1
  ${If} $INSTDIR == "$LOCALAPPDATA\UPR Terminal"
    ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\UltimatePR Station" "InstallLocation"
    StrCpy $R1 $R0 1
    ${If} $R1 == '$\"'
      StrCpy $R0 $R0 -1 1
    ${EndIf}
    ${If} $R0 != ""
      IfFileExists "$R0\${MAINBINARYNAME}.exe" 0 +2
        StrCpy $INSTDIR $R0
    ${EndIf}
  ${EndIf}
  ; Tauri sets OUTDIR before this hook. Keep extraction in the selected/migrated directory.
  SetOutPath "$INSTDIR"
  Pop $R1
  Pop $R0
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Push $R0
  Push $R1
  ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\UltimatePR Station" "InstallLocation"
  StrCpy $R1 $R0 1
  ${If} $R1 == '$\"'
    StrCpy $R0 $R0 -1 1
  ${EndIf}
  ${If} $R0 == $INSTDIR
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\UltimatePR Station"
  ${EndIf}
  !insertmacro IsShortcutTarget "$DESKTOP\UltimatePR Station.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  Pop $R0
  ${If} $R0 == 1
    Delete "$DESKTOP\UltimatePR Station.lnk"
  ${EndIf}
  !insertmacro IsShortcutTarget "$SMPROGRAMS\UltimatePR Station.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  Pop $R0
  ${If} $R0 == 1
    Delete "$SMPROGRAMS\UltimatePR Station.lnk"
  ${EndIf}
  Pop $R1
  Pop $R0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "UltimatePR Station"
!macroend
