$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Exe = Join-Path $Root 'release\ultimatepr-station-win-x64\ultimatepr-station.exe'
# Relative paths avoid argument splitting when the project directory contains
# spaces (for example "Ultimate Packet Radio"). The process working directory
# is set to $Root below.
$Config = 'data\station.conf'
$WebConfig = 'data\web.conf'

function Get-StationProcesses {
    Get-Process -Name 'ultimatepr-station' -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and $_.Path -ieq $Exe }
}

function Show-Status {
    $processes = @(Get-StationProcesses)
    if ($processes.Count -eq 0) {
        Write-Host 'UltimatePR Station: zatrzymana' -ForegroundColor Yellow
        return
    }
    foreach ($process in $processes) {
        Write-Host "UltimatePR Station: uruchomiona (PID $($process.Id))" -ForegroundColor Green
    }
}

function Start-Station {
    if (-not (Test-Path -LiteralPath $Exe)) {
        throw "Nie znaleziono aktualnego EXE: $Exe"
    }
    if (@(Get-StationProcesses).Count -gt 0) {
        Write-Host 'Aplikacja jest już uruchomiona.' -ForegroundColor Yellow
        return
    }
    Start-Process -FilePath $Exe -WorkingDirectory $Root -ArgumentList @(
        '--config', $Config,
        '--web-config', $WebConfig
    )
    Write-Host 'Aplikacja została uruchomiona.' -ForegroundColor Green
}

function Stop-Station {
    $processes = @(Get-StationProcesses)
    if ($processes.Count -eq 0) {
        Write-Host 'Aplikacja nie jest uruchomiona.' -ForegroundColor Yellow
        return
    }
    foreach ($process in $processes) {
        Stop-Process -Id $process.Id -Force
        Write-Host "Zatrzymano proces PID $($process.Id)." -ForegroundColor Green
    }
}

if (-not (Test-Path -LiteralPath $Exe)) {
    Write-Host "Brak pliku EXE: $Exe" -ForegroundColor Red
    Read-Host 'Naciśnij Enter, aby zakończyć'
    exit 1
}

$lastRefresh = [DateTime]::MinValue
$running = $true
[Console]::CursorVisible = $false
try {
    while ($running) {
        if (((Get-Date) - $lastRefresh).TotalSeconds -ge 1) {
            Clear-Host
            Write-Host '=== UltimatePR Station ===' -ForegroundColor Cyan
            Show-Status
            Write-Host ''
            Write-Host '1. Włącz aplikację'
            Write-Host '2. Wyłącz aplikację'
            Write-Host '3. Restart aplikacji'
            Write-Host '4. Odśwież status teraz'
            Write-Host '5. Zakończ'
            Write-Host ''
            Write-Host 'Wybierz klawisz 1-5:' -NoNewline
            $lastRefresh = Get-Date
        }

        if ([Console]::KeyAvailable) {
            $choice = [Console]::ReadKey($true).KeyChar
            if ($choice -eq '1') {
                Start-Station
                Start-Sleep -Milliseconds 300
                $lastRefresh = [DateTime]::MinValue
            } elseif ($choice -eq '2') {
                Stop-Station
                Start-Sleep -Milliseconds 300
                $lastRefresh = [DateTime]::MinValue
            } elseif ($choice -eq '3') {
                Stop-Station
                Start-Sleep -Milliseconds 500
                Start-Station
                Start-Sleep -Milliseconds 300
                $lastRefresh = [DateTime]::MinValue
            } elseif ($choice -eq '4') {
                $lastRefresh = [DateTime]::MinValue
            } elseif ($choice -eq '5') {
                $running = $false
            }
        }
        Start-Sleep -Milliseconds 100
    }
} finally {
    [Console]::CursorVisible = $true
}
