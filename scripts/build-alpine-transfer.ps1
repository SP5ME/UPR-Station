param(
    [string]$OutputRoot = ""
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'Cargo.toml')
$version = [regex]::Match($manifest, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if (-not $version) { throw 'Nie można odczytać wersji Cargo.toml.' }

if (-not $OutputRoot) { $OutputRoot = Join-Path $projectRoot 'release\alpine-test' }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$packageName = "ultimatepr-alpine-server-$version-$stamp"
$packageDir = Join-Path $OutputRoot $packageName
$zipPath = Join-Path $OutputRoot "$packageName.zip"
New-Item -ItemType Directory -Path $packageDir -Force | Out-Null

$sourceFiles = @(
    'Cargo.toml', 'Cargo.lock', 'LICENSE', 'TRADEMARKS.md', 'README.md'
)
$sourceDirs = @('src', 'web', 'docs', 'deploy\alpine', 'messages', 'languages', 'assets\branding')

foreach ($file in $sourceFiles) {
    $source = Join-Path $projectRoot $file
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Brak pliku: $file" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $packageDir $file) -Force
}
foreach ($dir in $sourceDirs) {
    $source = Join-Path $projectRoot $dir
    if (-not (Test-Path -LiteralPath $source -PathType Container)) { throw "Brak katalogu: $dir" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $packageDir $dir) -Recurse -Force
}

# Browser assets carry their own third-party license notices.
$assetLicenses = Join-Path $packageDir 'licenses'
New-Item -ItemType Directory -Path $assetLicenses -Force | Out-Null
foreach ($name in @('Apache-2.0.txt', 'MDI-LICENSE.txt')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot "licenses/$name") -Destination $assetLicenses
}

if (Test-Path -LiteralPath (Join-Path $packageDir 'data')) { throw 'Private data/ directory would be included.' }
if (Test-Path -LiteralPath (Join-Path $packageDir 'target')) { throw 'target/ directory would be included.' }

$manifestLines = @(
    "name=$packageName",
    "version=$version",
    'variant=linux-headless-server-alpine-musl',
    'source=data-targets-excluded',
    "created=$stamp"
)
Set-Content -LiteralPath (Join-Path $packageDir 'PACKAGE-MANIFEST.txt') -Value $manifestLines -Encoding utf8

$hashes = Get-ChildItem -LiteralPath $packageDir -Recurse -File |
    Where-Object { $_.Name -ne 'SHA256SUMS.txt' } |
    ForEach-Object {
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        "$hash  $($_.FullName.Substring($packageDir.Length + 1))"
    }
Set-Content -LiteralPath (Join-Path $packageDir 'SHA256SUMS.txt') -Value $hashes -Encoding utf8
Compress-Archive -Path (Join-Path $packageDir '*') -DestinationPath $zipPath -Force

# Compress-Archive zapisuje nazwy wpisów z separatorami Windows. Alpine
# oczekuje standardowych separatorów ZIP `/`, dlatego normalizujemy archiwum
# przed przekazaniem go do panelu aktualizacji.
Add-Type -AssemblyName System.IO.Compression.FileSystem
$normalizedZip = "$zipPath.normalized"
$sourceArchive = [IO.Compression.ZipFile]::OpenRead($zipPath)
$stream = [IO.File]::Open($normalizedZip, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
$destinationArchive = [IO.Compression.ZipArchive]::new($stream, [IO.Compression.ZipArchiveMode]::Create, $false)
$seenEntries = @{}
try {
    foreach ($entry in $sourceArchive.Entries) {
        $entryName = $entry.FullName.Replace('\', '/')
        if ($seenEntries.ContainsKey($entryName)) { throw "Zduplikowana ścieżka po normalizacji: $entryName" }
        $seenEntries[$entryName] = $true
        $newEntry = $destinationArchive.CreateEntry($entryName, [IO.Compression.CompressionLevel]::Optimal)
        $input = $entry.Open()
        $output = $newEntry.Open()
        try { $input.CopyTo($output) }
        finally { $output.Dispose(); $input.Dispose() }
    }
}
finally {
    $destinationArchive.Dispose()
    $sourceArchive.Dispose()
    $stream.Dispose()
}
Move-Item -LiteralPath $normalizedZip -Destination $zipPath -Force

# Ostateczna kontrola artefaktu, a nie tylko katalogu roboczego. Dzięki temu
# błąd normalizacji ZIP albo przypadkowo brakujący plik jest wykrywany przed
# przekazaniem paczki do testów na Alpine.
Add-Type -AssemblyName System.IO.Compression.FileSystem
$requiredEntries = @(
    'PACKAGE-MANIFEST.txt',
    'Cargo.toml',
    'Cargo.lock',
    'web/index.html',
    'web/app.js',
    'deploy/alpine/build-server-linux.sh',
    'deploy/alpine/install-server-linux.sh',
    'deploy/alpine/ultimatepr-station-run',
    'deploy/alpine/ultimatepr-station.initd',
    'deploy/alpine/verify-service.sh'
)
$archive = [IO.Compression.ZipFile]::OpenRead($zipPath)
try {
    $entryNames = @($archive.Entries | ForEach-Object { $_.FullName })
    if ($entryNames | Where-Object { $_ -like '*\*' }) {
        throw 'ZIP contains Windows separators; normalization failed.'
    }
    if ($entryNames | Where-Object { $_ -match '^(target|data)(/|$)' }) {
        throw 'ZIP contains forbidden target/ or data/ directory.'
    }
    foreach ($required in $requiredEntries) {
        if ($entryNames -notcontains $required) { throw "ZIP nie zawiera wymaganego pliku: $required" }
    }
}
finally {
    $archive.Dispose()
}

$zipHash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
$latestInfo = @(
    "package=$([IO.Path]::GetFileName($zipPath))",
    "sha256=$zipHash",
    'variant=linux-headless-server-alpine-musl',
    'verification=zip-structure-and-required-files-ok'
)
Set-Content -LiteralPath (Join-Path $OutputRoot 'LATEST-PACKAGE.txt') -Value $latestInfo -Encoding utf8

Write-Host "Paczka Alpine: $zipPath" -ForegroundColor Green
Write-Host "Rozpakowany katalog: $packageDir"
Write-Host "SHA-256 ZIP: $zipHash"
Write-Host "Wskaznik najnowszej paczki: $(Join-Path $OutputRoot 'LATEST-PACKAGE.txt')"
