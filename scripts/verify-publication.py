"""Read-only publication checks. No Git writes; never prints matching secret values."""
from pathlib import Path
import re
import sys
import subprocess

ROOT = Path(__file__).resolve().parents[1]
DIRECTORIES = {'.github', 'src', 'web', 'docs', 'scripts', 'tests', 'deploy',
               'languages', 'messages', 'assets', 'licenses'}
FILES = {'.gitignore', '.gitattributes', '.dockerignore', 'README.md', 'LICENSE',
         'TRADEMARKS.md', 'CONTRIBUTING.md', 'SECURITY.md', 'Cargo.toml', 'Cargo.lock',
         'Dockerfile', 'docker-compose.yml', 'ultimatepr-logo.png',
         'ultimatepr-station-icon.ico', 'ultimatepr-station-icon.png'}
IGNORED = {'.git', 'target', 'release', '__pycache__', 'node_modules', '.venv'}
PRIVATE = {'data', 'backups', '.local-notes', 'github-ready', 'linux-transfer', 'prealpha'}
PATTERNS = {
    'private key': re.compile(r'-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----'),
    'GitHub token': re.compile(r'\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b'),
    'AWS access key': re.compile(r'\b(?:AKIA|ASIA)[A-Z0-9]{16}\b'),
    'Telegram token': re.compile(r'\b\d{8,12}:[A-Za-z0-9_-]{30,}\b'),
    'local user path': re.compile(r'(?i)[A-Z]:[\\/]+Users[\\/]+mikol\b'),
}

def inspect(root=ROOT):
    issues = []
    count = 0
    # Inspect the publication inventory, not adjacent ignored operator data.
    # Tracked private files remain visible here even when .gitignore excludes them.
    names = subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'], cwd=root).decode('utf-8').split('\0')
    for name in sorted(set(filter(None,names))):
        path = root / name
        relative = Path(name)
        if any(part in PRIVATE or part in IGNORED or part.startswith('target-') for part in relative.parts):
            issues.append(f'{relative}: private/generated publication path'); continue
        if not path.exists() and not path.is_symlink():
            continue  # tracked source deletion in the working tree
        if path.is_symlink():
            issues.append(f'{relative}: symbolic link'); continue
        if path.is_dir():
            if path.name in PRIVATE:
                issues.append(f'{relative}: private/generated directory')
            continue
        if relative.parts[0] not in DIRECTORIES and str(relative) not in FILES:
            issues.append(f'{relative}: unreviewed root file')
        if path.suffix.lower() in {'.zip', '.exe', '.msi', '.deb', '.log', '.bak', '.pyc',
                                  '.token', '.password', '.pem', '.key', '.p12', '.pfx', '.conf'}:
            issues.append(f'{relative}: generated/private file type')
        if path.name == '.env' or path.name.startswith('.env.'):
            issues.append(f'{relative}: environment file')
        count += 1
        if path.suffix.lower() in {'.png', '.ico'}:
            continue
        try:
            text = path.read_text(encoding='utf-8-sig')
        except UnicodeDecodeError:
            issues.append(f'{relative}: unreviewed binary file'); continue
        for number, line in enumerate(text.splitlines(), 1):
            for label, pattern in PATTERNS.items():
                if pattern.search(line):
                    issues.append(f'{relative}:{number}: possible {label}')
    return count, issues

if __name__ == '__main__':
    count, issues = inspect()
    for issue in issues:
        print(issue)
    print(f'Checked {count} publication files; {len(issues)} findings. This is not a complete secret or license audit.')
    sys.exit(bool(issues))
