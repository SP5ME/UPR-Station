param(
    [switch]$Offline,
    [string]$CliScript
)

$ErrorActionPreference = 'Stop'
$projectRoot = $PSScriptRoot
$buildScript = Join-Path $projectRoot 'scripts\build-desktop.ps1'

if (-not (Test-Path -LiteralPath $buildScript)) {
    throw "Brak skryptu budowania: $buildScript"
}

# Prefer the bundled Tauri CLI used by this project when it is available.
if (-not $CliScript) {
    $bundledCli = Join-Path $projectRoot 'target\desktop-tools\cli\package\tauri.js'
    if (Test-Path -LiteralPath $bundledCli) {
        $CliScript = $bundledCli
    }
}

$buildArgs = @{}
if ($Offline) { $buildArgs.Offline = $true }
if ($CliScript) { $buildArgs.CliScript = $CliScript }

Write-Host 'Budowanie instalatora UPR Terminal...' -ForegroundColor Cyan
& $buildScript @buildArgs
if ($LASTEXITCODE -ne 0) {
    throw 'Installer build failed.'
}

$installer = Get-ChildItem -LiteralPath (Join-Path $projectRoot 'desktop\target\release\bundle\nsis') `
    -Filter '*-setup.exe' -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

if (-not $installer) {
    throw 'Generated installer was not found.'
}

$downloads = Join-Path ([Environment]::GetFolderPath('UserProfile')) 'Downloads'
if (-not (Test-Path -LiteralPath $downloads)) {
    $downloads = Join-Path $env:USERPROFILE 'Pobrane'
}
if (-not (Test-Path -LiteralPath $downloads)) {
    New-Item -ItemType Directory -Path $downloads -Force | Out-Null
}

$destination = Join-Path $downloads $installer.Name
try {
    Copy-Item -LiteralPath $installer.FullName -Destination $destination -Force -ErrorAction Stop
} catch [System.UnauthorizedAccessException] {
    $base = [System.IO.Path]::GetFileNameWithoutExtension($installer.Name)
    $destination = Join-Path $downloads ("{0}-{1}{2}" -f $base, (Get-Date -Format 'yyyyMMdd-HHmmss'), $installer.Extension)
    try {
        Copy-Item -LiteralPath $installer.FullName -Destination $destination -Force -ErrorAction Stop
    } catch [System.UnauthorizedAccessException] {
        Write-Warning "Nie można skopiować instalatora do katalogu Downloads; artefakt pozostaje w release/desktop."
        $destination = $null
    }
}

if ($destination) {
    $hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
    $hashFile = "$destination.sha256"
    Set-Content -LiteralPath $hashFile -Value "$hash  $($installer.Name)" -Encoding utf8
}

Write-Host ''
Write-Host "Gotowy instalator: $($installer.FullName)" -ForegroundColor Green
if ($destination) {
    Write-Host "Kopia użytkownika: $destination"
    Write-Host "SHA-256: $hash"
    Write-Host "Suma zapisana: $hashFile"
}
