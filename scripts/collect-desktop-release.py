from pathlib import Path
import shutil
import sys
system,tag,destination=sys.argv[1:]
root=Path(__file__).resolve().parents[1]
out=Path(destination);out.mkdir(parents=True,exist_ok=True)
pattern='nsis/*-setup.exe' if system=='Windows' else 'deb/*_amd64.deb'
files=list((root/'desktop/target/release/bundle').glob(pattern))
if len(files)!=1: raise SystemExit('Expected exactly one installer from a fresh build')
name=f'UPR-Terminal-{tag}-windows-x86_64-setup.exe' if system=='Windows' else f'UPR-Terminal-{tag}-linux-x86_64.deb'
shutil.copy2(files[0],out/name)
