param([string]$ProcessName = 'todotxt', [int]$ProcessId = 0, [string]$OutputPath = 'docs/screenshots/upstream.png')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class WindowCapture {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
}
'@
$process = if ($ProcessId) { Get-Process -Id $ProcessId } else { Get-Process -Name $ProcessName | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1 }
if (!$process) { throw "No visible window for $ProcessName" }
$rect = New-Object WindowCapture+RECT
[void][WindowCapture]::GetWindowRect($process.MainWindowHandle, [ref]$rect)
$bitmap = New-Object System.Drawing.Bitmap ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$dc = $graphics.GetHdc()
try { $captured = [WindowCapture]::PrintWindow($process.MainWindowHandle, $dc, 2) }
finally { $graphics.ReleaseHdc($dc) }
if (!$captured) { throw 'PrintWindow failed' }
$absolute = [IO.Path]::GetFullPath($OutputPath)
[void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($absolute))
$bitmap.Save($absolute, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
Write-Output $absolute
