"""Collect the resolved target's crate notices into an isolated bundle resource directory."""
import json
from pathlib import Path
import shutil
import subprocess

root = Path(__file__).resolve().parents[1]
host = next(line.split(": ", 1)[1] for line in subprocess.check_output(["rustc", "-vV"], text=True).splitlines() if line.startswith("host: "))
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1", "--locked", "--offline",
    "--manifest-path", str(root / "desktop/Cargo.toml"), "--filter-platform", host,
], text=True, encoding="utf-8"))
destination = root / "desktop/target/legal"
destination.mkdir(parents=True, exist_ok=True)
lines = ["# Dependency notices", "", f"Target: {host}", "", "Generated from desktop/Cargo.lock. See each crate directory for its supplied license texts.", ""]
missing = []
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    key = f'{package["name"]}-{package["version"]}'
    source = Path(package["manifest_path"]).parent
    files = [p for p in source.iterdir() if p.is_file() and p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT"))]
    if package.get("license_file"):
        explicit = source / package["license_file"]
        if explicit.is_file() and explicit not in files:
            files.append(explicit)
    for folder in (source / "LICENSES", source / "licenses"):
        if folder.is_dir():
            files.extend(p for p in folder.rglob("*") if p.is_file())
    extra = root / "licenses/desktop" / key
    if not files and extra.is_dir():
        files = [p for p in extra.iterdir() if p.is_file()]
    if not files and package.get("source") is None:
        files = [root / "LICENSE"]
    out = destination / key
    out.mkdir(exist_ok=True)
    for file in files:
        relative = file.relative_to(source) if file.is_relative_to(source) else Path(file.name)
        target = out / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(file, target)
    license_id = package.get("license") or "SEE SUPPLIED LICENSE"
    lines.append(f'- {key}: {license_id}; {package.get("repository") or package.get("homepage") or "local project"}')
    if not files:
        missing.append(key)
(destination / "THIRD-PARTY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
if missing:
    print("License texts absent from distributed crate archives: " + ", ".join(missing))
    print("Resolve missing license texts before public distribution.")
    (destination / "MISSING-LICENSE-TEXTS.txt").write_text("\n".join(missing) + "\n", encoding="utf-8")
else:
    (destination / "MISSING-LICENSE-TEXTS.txt").unlink(missing_ok=True)
print(f"Collected notices for {len(metadata['packages'])} packages ({host}).")
