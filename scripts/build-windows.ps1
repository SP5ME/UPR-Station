param(
    [switch]$SkipZip
)

$ErrorActionPreference = 'Stop'
$OutputEncoding = [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$projectRoot = Split-Path -Parent $PSScriptRoot
$packageDir = Join-Path $projectRoot 'release\ultimatepr-station-win-x64'
$builtExe = Join-Path $projectRoot 'target\release\ultimatepr-station.exe'
$packageExe = Join-Path $packageDir 'ultimatepr-station.exe'
$zipPath = Join-Path $projectRoot 'release\ultimatepr-station-win-x64.zip'

Set-Location $projectRoot

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Nie znaleziono cargo. Zainstaluj Rust oraz cargo i uruchom skrypt ponownie.'
}

Write-Host 'Budowanie UltimatePR Station (release)...' -ForegroundColor Cyan
cargo build --release

if (-not (Test-Path -LiteralPath $builtExe)) {
    throw "Nie znaleziono zbudowanego pliku: $builtExe"
}

New-Item -ItemType Directory -Force -Path $packageDir | Out-Null
try {
    Copy-Item -LiteralPath $builtExe -Destination $packageExe -Force
}
catch {
    Write-Host 'Nie można podmienić EXE, ponieważ jest używany przez uruchomioną aplikację.' -ForegroundColor Yellow
    Write-Host 'Zamknij UltimatePR Station i uruchom build-windows.cmd ponownie.' -ForegroundColor Yellow
    exit 2
}

$hash = (Get-FileHash -LiteralPath $packageExe -Algorithm SHA256).Hash
Set-Content -LiteralPath (Join-Path $packageDir 'ultimatepr-station.exe.sha256') -Value "$hash  ultimatepr-station.exe" -Encoding ascii

if (-not $SkipZip) {
    Write-Host 'Tworzenie paczki ZIP...' -ForegroundColor Cyan
    Compress-Archive -Path (Join-Path $packageDir '*') -DestinationPath $zipPath -Force
}

Write-Host ''
Write-Host 'Gotowe.' -ForegroundColor Green
Write-Host "EXE:  $packageExe"
Write-Host "SHA:  $hash"
if (-not $SkipZip) {
    Write-Host "ZIP:  $zipPath"
}
