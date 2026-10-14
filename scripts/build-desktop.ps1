param([switch]$Offline, [string]$CliScript)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    if ($Offline) { $env:CARGO_NET_OFFLINE = 'true' }
    cargo fetch --locked --manifest-path desktop/Cargo.toml --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Nie udało się przygotować zależności desktopu.' }
    python scripts/desktop-licenses.py
    if ($LASTEXITCODE -ne 0) { throw 'Nie udało się przygotować licencji.' }
    if (Test-Path -LiteralPath desktop/target/legal/MISSING-LICENSE-TEXTS.txt) { throw 'Brak tekstów licencji; zobacz desktop/target/legal/MISSING-LICENSE-TEXTS.txt.' }
    if ($CliScript) { $CliScript = (Resolve-Path -LiteralPath $CliScript).Path }
    $buildStarted = Get-Date
    $rootManifest = Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'Cargo.toml')
    $desktopManifest = Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'desktop\Cargo.toml')
    $tauriConfig = Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'desktop\tauri.conf.json') | ConvertFrom-Json
    $rootVersion = [regex]::Match($rootManifest, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
    $desktopVersion = [regex]::Match($desktopManifest, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
    if (-not $rootVersion -or $rootVersion -ne $desktopVersion -or $rootVersion -ne $tauriConfig.version) {
        throw "Niespójne wersje projektu: root=$rootVersion desktop=$desktopVersion tauri=$($tauriConfig.version)"
    }
    Push-Location desktop
    try {
        if ($CliScript) { node $CliScript build --bundles nsis --no-sign -- --locked }
        else { cargo tauri build --bundles nsis --no-sign -- --locked }
        if ($LASTEXITCODE -ne 0) { throw 'Budowanie instalatora nie powiodło się.' }
    } finally { Pop-Location }
    $installer = Get-ChildItem -LiteralPath desktop/target/release/bundle/nsis -Filter '*-setup.exe' |
        Where-Object { $_.LastWriteTime -ge $buildStarted } |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $installer) { throw 'Brak wygenerowanego instalatora.' }
    # A fresh directory prevents stale files or operator data entering a new package.
    $output = Join-Path $projectRoot ('release\desktop\windows-x64-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
    New-Item -ItemType Directory -Path $output | Out-Null
    Copy-Item -LiteralPath $installer.FullName -Destination $output
    Copy-Item -LiteralPath docs,LICENSE,TRADEMARKS.md -Destination $output -Recurse
    $sourceRoots = @('Cargo.toml','Cargo.lock','desktop\Cargo.toml','desktop\Cargo.lock','desktop\tauri.conf.json','src','web','desktop\src')
    $sourceFiles = foreach ($sourceRoot in $sourceRoots) {
        $resolved = Join-Path $projectRoot $sourceRoot
        if ((Get-Item -LiteralPath $resolved).PSIsContainer) { Get-ChildItem -LiteralPath $resolved -Recurse -File }
        else { Get-Item -LiteralPath $resolved }
    }
    $sourceHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $sourceFiles.FullName | Sort-Object Path | ForEach-Object Hash | Out-String).Replace("`r",'').Replace("`n",'')
    Set-Content -LiteralPath (Join-Path $output 'BUILD-INFO.txt') -Value @("version=$rootVersion", "installer=$($installer.Name)", "source_sha256=$sourceHash") -Encoding utf8
    $hashes = Get-ChildItem -LiteralPath $output -Recurse -File | ForEach-Object {
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        "$hash  $($_.FullName.Substring($output.Length + 1))"
    }
    Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') -Value $hashes -Encoding utf8
    Write-Output "Instalator i dokumentacja: $output"
} finally { Pop-Location }
