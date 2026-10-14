"""Guard against changing INSTDIR after Tauri has selected OUTDIR."""
from pathlib import Path
import re
root=Path(__file__).resolve().parents[2]
hooks=(root/'desktop/installer-hooks.nsh').read_text(encoding='utf-8')
preinstall=hooks.split('!macro NSIS_HOOK_PREINSTALL',1)[1].split('!macroend',1)[0]
assert 'StrCpy $INSTDIR $R0' in preinstall
sync=re.search(r'^\s*SetOutPath\s+"\$INSTDIR"\s*$',preinstall,re.M)
assert sync, 'Migrated installation must synchronize OUTDIR'
assert sync.start()>preinstall.rfind('StrCpy $INSTDIR'), 'OUTDIR must be synchronized after directory selection'
print('PASS: NSIS synchronizes resource extraction after legacy directory migration.')
