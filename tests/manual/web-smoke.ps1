param([string]$Chrome = 'C:\Program Files\Google\Chrome\Application\chrome.exe')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $root
if (-not (Test-Path -LiteralPath $Chrome)) { throw 'Chrome required for the isolated headless browser test.' }
$data = Join-Path $root ('target\web-smoke-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $data | Out-Null
Set-Content -LiteralPath (Join-Path $data 'station.conf') -Encoding utf8 -Value "callsign=TEST-1`nlanguage=en`nbeacon_enabled=false`ndigipeat_enabled=false`ninterface_TEST=127.0.0.1:1,0,0,false,Isolated test"
Set-Content -LiteralPath (Join-Path $data 'web.conf') -Encoding utf8 -Value "bind=127.0.0.1:19334`nauth_enabled=false"
$server = $null
$browser = $null
try {
    $arguments = '--bind 127.0.0.1:19334 --config "{0}" --web-config "{1}"' -f (Join-Path $data 'station.conf'), (Join-Path $data 'web.conf')
    $server = Start-Process -FilePath (Join-Path $root 'target\debug\ultimatepr-station.exe') -WorkingDirectory $root -ArgumentList $arguments -WindowStyle Hidden -PassThru
    $healthy = $false
    for ($i=0; $i -lt 40; $i++) {
        try { $healthy = (Invoke-WebRequest http://127.0.0.1:19334/healthz -TimeoutSec 1).StatusCode -eq 200 } catch {}
        if ($healthy) { break }
        Start-Sleep -Milliseconds 250
    }
    if (-not $healthy) { throw 'Test Station did not start.' }
    $browserArguments = '--headless=new --disable-gpu --no-first-run --no-default-browser-check --remote-debugging-port=19333 --user-data-dir="{0}" http://127.0.0.1:19334/' -f (Join-Path $data 'browser')
    $browser = Start-Process -FilePath $Chrome -ArgumentList $browserArguments -WindowStyle Hidden -PassThru
    for ($i=0; $i -lt 40; $i++) {
        try { $pages = Invoke-RestMethod http://127.0.0.1:19333/json/list -TimeoutSec 1; if ($pages) { break } } catch {}
        Start-Sleep -Milliseconds 250
    }
    node tests/browser/panel-smoke.mjs 19333
    if ($LASTEXITCODE -ne 0) { throw 'Station browser smoke failed.' }
    Write-Output "Isolated test data: $data"
} finally {
    # Only these two harness-created processes are terminated, never a user session.
    foreach ($process in @($browser,$server)) {
        if ($process -and -not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
    }
}
