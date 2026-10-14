"""Offline release-layout checks for the portable Station package."""
from pathlib import Path
import sys


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).parents[1]
    package = root / "release" / "ultimatepr-station-win-x64"
    required = [
        package / "ultimatepr-station.exe",
        package / "Dockerfile",
        package / "docker-compose.yml",
        package / "README.md",
        package / "web.conf.example",
        package / "deploy" / "ultimatepr-station.service",
        package / "web" / "index.html",
        package / "web" / "app.js",
        package / "web" / "app.css",
        package / "web" / "lang" / "en.json",
        package / "web" / "lang" / "pl.json",
    ]
    missing = [str(path.relative_to(package)) for path in required if not path.is_file()]
    if missing:
        raise SystemExit("missing release files: " + ", ".join(missing))
    dockerfile = (package / "Dockerfile").read_text(encoding="utf-8")
    for fragment in ("FROM rust:", "FROM alpine:", "--web-config", "HEALTHCHECK"):
        if fragment not in dockerfile:
            raise SystemExit(f"Dockerfile missing required fragment: {fragment}")
    service = (package / "deploy" / "ultimatepr-station.service").read_text(encoding="utf-8")
    for fragment in ("ExecStart=", "--config", "--web-config", "Restart=on-failure"):
        if fragment not in service:
            raise SystemExit(f"systemd unit missing required fragment: {fragment}")
    compose = (package / "docker-compose.yml").read_text(encoding="utf-8")
    for fragment in ("build: .", "STATION_TOKEN", "ultimatepr-data", "8080:8080"):
        if fragment not in compose:
            raise SystemExit(f"docker-compose.yml missing required fragment: {fragment}")
    print(f"PASS: release layout ({len(required)} required files), Dockerfile and systemd unit")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
