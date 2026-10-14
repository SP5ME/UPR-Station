param([string]$Exe = 'desktop\target\debug\ultimatepr-station-desktop.exe')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $root
if (Get-Process -Name 'ultimatepr-station-desktop' -ErrorAction SilentlyContinue) { throw 'Close the desktop app before running its isolated smoke test.' }
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class DesktopSmokeNative {
 public delegate bool EnumProc(IntPtr window, IntPtr parameter);
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr parameter);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr window, StringBuilder text, int size);
 public static IntPtr FindMain(uint pid) {
   IntPtr found=IntPtr.Zero;
   EnumWindows((window, parameter) => {
     uint owner; GetWindowThreadProcessId(window,out owner);
     if(owner==pid) {
       var title=new StringBuilder(512);GetWindowTextW(window,title,512);
       if(title.ToString().Contains("UltimatePR Station") && GetMenu(window)!=IntPtr.Zero) found=window;
     }
     return true;
   },IntPtr.Zero);
   return found;
 }
 [DllImport("user32.dll")] public static extern IntPtr GetMenu(IntPtr window);
 [DllImport("user32.dll")] public static extern IntPtr GetSubMenu(IntPtr menu, int index);
 [DllImport("user32.dll")] public static extern int GetMenuItemCount(IntPtr menu);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetMenuStringW(IntPtr menu, uint item, StringBuilder text, int size, uint flags);
 [DllImport("user32.dll")] public static extern uint GetMenuItemID(IntPtr menu, int index);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
 [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
}
'@
$previousLocal = $env:LOCALAPPDATA
$previousArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$app = $null
try {
    $env:LOCALAPPDATA = Join-Path $root ('target\desktop-smoke-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=19222'
    $data = Join-Path $env:LOCALAPPDATA 'UltimatePR Station\demo'
    $app = Start-Process -FilePath (Join-Path $root $Exe) -WorkingDirectory $root -ArgumentList '--demo' -WindowStyle Hidden -PassThru
    $pages = $null
    for ($i=0; $i -lt 80; $i++) {
        Start-Sleep -Milliseconds 250
        $app.Refresh()
        if ($app.HasExited) { throw 'Desktop exited before ready.' }
        try { $pages = Invoke-RestMethod http://127.0.0.1:19222/json/list -TimeoutSec 1 } catch { continue }
        if ($pages | Where-Object { $_.url -like 'http://127.0.0.1:*' }) { break }
    }
    node tests/manual/desktop-smoke.mjs 19222
    if ($LASTEXITCODE -ne 0) { throw 'WebView smoke test failed.' }
    $app.Refresh()
    $window = [DesktopSmokeNative]::FindMain($app.Id)
    if ($window -eq 0) { throw 'Main window not found.' }
    $menu = [DesktopSmokeNative]::GetSubMenu([DesktopSmokeNative]::GetMenu($window), 0)
    $quit = $null
    $files = $null
    for ($i=0; $i -lt [DesktopSmokeNative]::GetMenuItemCount($menu); $i++) {
        $label = [Text.StringBuilder]::new(256)
        [void][DesktopSmokeNative]::GetMenuStringW($menu, $i, $label, 256, 0x400)
        Write-Output ('Menu: '+$i+' '+$label.ToString())
        # The menu is constructed in a fixed order in setup(); using indexes
        # keeps this smoke test independent of the console code page.
        if ($i -eq 8) { $quit = [DesktopSmokeNative]::GetMenuItemID($menu, $i) }
        if ($i -eq 4) { $files = [DesktopSmokeNative]::GetMenuItemID($menu, $i) }
    }
    if ($null -eq $quit -or $null -eq $files) { throw 'Native menu entries not found.' }
    [void][DesktopSmokeNative]::PostMessageW($window, 0x111, [IntPtr]$files, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 500
    if ((Get-Content -LiteralPath (Join-Path $data 'desktop.conf')) -notcontains 'files=false') { throw 'Preference toggle not persisted.' }
    [void][DesktopSmokeNative]::PostMessageW($window, 0x111, [IntPtr]$files, [IntPtr]::Zero)
    [void][DesktopSmokeNative]::PostMessageW($window, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 500
    if ([DesktopSmokeNative]::IsWindowVisible($window)) { throw 'Close did not hide window to tray.' }
    $url = ($pages | Where-Object { $_.url -like 'http://127.0.0.1:*' } | Select-Object -First 1).url
    if ((Invoke-WebRequest ($url.TrimEnd('/') + '/healthz') -TimeoutSec 3).StatusCode -ne 200) { throw 'Backend stopped with hidden window.' }
    $second = Start-Process -FilePath (Join-Path $root $Exe) -WorkingDirectory $root -ArgumentList '--demo' -WindowStyle Hidden -PassThru
    if (-not $second.WaitForExit(5000)) { throw 'Second instance did not exit.' }
    Start-Sleep -Milliseconds 500
    if (-not [DesktopSmokeNative]::IsWindowVisible($window)) { throw 'Second launch did not restore first window.' }
    [void][DesktopSmokeNative]::PostMessageW($window, 0x111, [IntPtr]$quit, [IntPtr]::Zero)
    if (-not $app.WaitForExit(20000)) { throw 'Graceful exit did not finish.' }
    $alive = $false
    try { $alive = (Invoke-WebRequest ($url.TrimEnd('/') + '/healthz') -TimeoutSec 2).StatusCode -eq 200 } catch {}
    if ($alive) { throw 'Backend remained alive after exit.' }
    Write-Output 'PASS: native preferences, close-to-tray, live hidden backend, single instance restore, graceful exit'
    Write-Output "Isolated test data: $data"
} finally {
    $env:LOCALAPPDATA = $previousLocal
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousArguments
    if ($app -and -not $app.HasExited) {
        # Only this harness-created demo process can be terminated on test failure.
        $app.Kill()
        $app.WaitForExit()
    }
}
