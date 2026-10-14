param([Parameter(Mandatory=$true)][string]$Installer)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
# This test writes disposable HKCU installation entries and shortcuts. Never run on a user's PC.
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows' -or -not $env:RUNNER_TEMP) {
    throw 'Installer integration tests may run only on an ephemeral GitHub Windows runner.'
}
$Installer=(Resolve-Path -LiteralPath $Installer).Path
$repo=Split-Path -Parent $PSScriptRoot
$default=Join-Path $env:LOCALAPPDATA 'UPR Terminal'
$legacy=Join-Path $env:LOCALAPPDATA 'UltimatePR Station'
$data=Join-Path $env:APPDATA 'UltimatePR Station'
$custom=Join-Path $env:RUNNER_TEMP ('upr-install-test-'+[guid]::NewGuid().ToString()+'\Custom Folder')
$newKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\UPR Terminal'
$oldKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\UltimatePR Station'
$desktop=[Environment]::GetFolderPath('DesktopDirectory')
$programs=[Environment]::GetFolderPath('Programs')
$shell=New-Object -ComObject WScript.Shell
$links=@((Join-Path $desktop 'UPR Terminal.lnk'),(Join-Path $programs 'UPR Terminal.lnk'),(Join-Path $desktop 'UltimatePR Station.lnk'),(Join-Path $programs 'UltimatePR Station.lnk'))
foreach($path in @($default,$legacy,$data,$custom,$newKey,$oldKey)+$links){
    if(Test-Path -LiteralPath $path){throw "Nonempty runner installation state: $path"}
}
New-Item -ItemType Directory -Path $data | Out-Null
$sentinel=Join-Path $data 'station.conf'
[IO.File]::WriteAllText($sentinel,"# installer preservation fixture`ncallsign=N0CALL`n",[Text.UTF8Encoding]::new($false))
$sentinelHash=(Get-FileHash -LiteralPath $sentinel).Hash
# Tauri bundles a patched copy, then restores the build output (tauri-bundler bundle.rs).
# Compare against precisely that one documented NSIS marker replacement, not arbitrary differences.
$expectedBytes=[IO.File]::ReadAllBytes((Join-Path $repo 'desktop\target\release\ultimatepr-station-desktop.exe'))
$marker='__TAURI_BUNDLE_TYPE_VAR_UNK'
$markerIndex=[Text.Encoding]::Latin1.GetString($expectedBytes).IndexOf($marker,[StringComparison]::Ordinal)
if($markerIndex -lt 0){throw 'Expected Tauri bundle marker is missing; review bundler compatibility'}
[Array]::Copy([Text.Encoding]::ASCII.GetBytes('__TAURI_BUNDLE_TYPE_VAR_NSS'),0,$expectedBytes,$markerIndex,$marker.Length)
$expectedBinaryHash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($expectedBytes))
function Assert-Data {
    if((Get-FileHash -LiteralPath $sentinel).Hash -ne $sentinelHash){throw 'Operator configuration changed'}
}
function Run-Setup([string]$Target) {
    # NSIS requires /D as the last argument, unquoted even when the path contains spaces.
    $process=Start-Process -FilePath $Installer -ArgumentList "/S /D=$Target" -WindowStyle Hidden -PassThru
    if(-not $process.WaitForExit(180000)){
        Stop-Process -Id $process.Id -Force
        throw 'Installer timed out'
    }
    if($process.ExitCode -ne 0){throw "Installer failed: $($process.ExitCode)"}
}
function Assert-Install([string]$Target) {
    foreach($relative in @('ultimatepr-station-desktop.exe','LICENSE','TRADEMARKS.md','docs\ARCHITECTURE.md','licenses\crates\THIRD-PARTY.md','licenses\assets\Apache-2.0.txt')){
        if(-not(Test-Path -LiteralPath (Join-Path $Target $relative) -PathType Leaf)){throw "Missing installed file: $relative in $Target"}
    }
    $expected=(Get-FileHash -LiteralPath (Join-Path $repo 'docs\ARCHITECTURE.md')).Hash
    if((Get-FileHash -LiteralPath (Join-Path $Target 'docs\ARCHITECTURE.md')).Hash -ne $expected){throw 'Installed documentation differs from package source'}
    if((Get-FileHash -LiteralPath (Join-Path $Target 'ultimatepr-station-desktop.exe')).Hash -ne $expectedBinaryHash){throw 'Wrong installed executable'}
    $location=(Get-ItemProperty -LiteralPath $newKey).InstallLocation.Trim('"')
    if($location -ne $Target){throw "Uninstall location differs: $location"}
    foreach($link in $links[0..1]){
        if(-not(Test-Path -LiteralPath $link)){throw "Missing shortcut: $link"}
        if($shell.CreateShortcut($link).TargetPath -ne (Join-Path $Target 'ultimatepr-station-desktop.exe')){throw "Wrong shortcut target: $link"}
    }
    Assert-Data
}
function Uninstall-Test([string]$Target) {
    $uninstaller=Join-Path $Target 'uninstall.exe'
    if(-not(Test-Path -LiteralPath $uninstaller)){throw 'Missing uninstaller'}
    $process=Start-Process -FilePath $uninstaller -ArgumentList "/S _?=$Target" -WindowStyle Hidden -PassThru
    if(-not $process.WaitForExit(180000)){Stop-Process -Id $process.Id -Force;throw 'Uninstaller timed out'}
    if($process.ExitCode -ne 0){throw "Uninstaller failed: $($process.ExitCode)"}
    if(Test-Path -LiteralPath $newKey){throw 'Uninstall entry was not removed'}
    Assert-Data
}
Run-Setup $default
Assert-Install $default
Write-Output 'PASS: fresh default installation, resources, shortcuts and unchanged configuration.'
# Minimal legacy fixture: original executable, quoted install location and old shortcuts.
New-Item -ItemType Directory -Path $legacy | Out-Null
Copy-Item -LiteralPath (Join-Path $default 'ultimatepr-station-desktop.exe') -Destination $legacy
New-Item -Path $oldKey -Force | Out-Null
New-ItemProperty -LiteralPath $oldKey -Name InstallLocation -Value ('"'+$legacy+'"') -PropertyType String | Out-Null
foreach($link in $links[2..3]){$shortcut=$shell.CreateShortcut($link);$shortcut.TargetPath=Join-Path $legacy 'ultimatepr-station-desktop.exe';$shortcut.Save()}
Uninstall-Test $default
Run-Setup $custom
Assert-Install $custom
if(-not(Test-Path -LiteralPath $oldKey)){throw 'Custom installation removed unrelated legacy registration'}
foreach($link in $links[2..3]){if(-not(Test-Path -LiteralPath $link)){throw 'Custom installation removed unrelated legacy shortcut'}}
Write-Output 'PASS: custom directory with spaces is not redirected to legacy installation.'
Uninstall-Test $custom
# Reproduce the user's interrupted setup: files remain in the new name's directory.
New-Item -ItemType Directory -Path $default -Force | Out-Null
$partialBinary=Join-Path $default 'ultimatepr-station-desktop.exe'
[IO.File]::WriteAllText($partialBinary,'interrupted setup fixture')
$partialHash=(Get-FileHash -LiteralPath $partialBinary).Hash
Run-Setup $default
Assert-Install $legacy
if(Test-Path -LiteralPath $oldKey){throw 'Legacy uninstall registration remained after migration'}
foreach($link in $links[2..3]){if(Test-Path -LiteralPath $link){throw 'Legacy shortcut remained after migration'}}
if((Get-FileHash -LiteralPath $partialBinary).Hash -ne $partialHash){throw 'Migration still extracted executable into the interrupted installation directory'}
if(Test-Path -LiteralPath (Join-Path $default 'docs\ARCHITECTURE.md')){throw 'Migration split documentation between old and new directories'}
Write-Output 'PASS: legacy migration and interrupted setup recovery keep active files and shortcuts together; existing leftovers are not deleted.'
Run-Setup $legacy
Assert-Install $legacy
Write-Output 'PASS: repeat installation in the migrated directory; configuration preserved.'
Uninstall-Test $legacy
Write-Output 'PASS: uninstallation preserves operator configuration.'
# No recursive cleanup outside the checkout: the entire hosted runner is disposable.
