; MinPoint NSIS installer hooks (included by Tauri's installer template).
;
; Before installing, upgrading or uninstalling, force-close any running MinPoint.
; MinPoint keeps running in the system tray, and Tauri's built-in "is the app running?"
; check can fail to close it (for example a frozen 0.1.0 process, or security software
; blocking the gentle close). taskkill /F /T ends minpoint.exe and its WebView2 children.

!macro MINPOINT_FORCE_CLOSE
  DetailPrint "Closing MinPoint if it is running..."
  nsExec::Exec '"$SYSDIR\taskkill.exe" /F /T /IM ${MAINBINARYNAME}.exe'
  Pop $0
  ${If} $0 == 0
    Sleep 1000 ; give Windows a moment to release file locks
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro MINPOINT_FORCE_CLOSE
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro MINPOINT_FORCE_CLOSE
!macroend
