param([string]$Binary = 'target/debug/todotxt-rs.exe')
$ErrorActionPreference = 'Stop'
# Exercise real Win32 controls against an isolated copy of the sample.
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class TodoPreview {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
    public static int Width(IntPtr h) { Rect r; GetClientRect(h, out r); return r.Right; }
    public static int OuterWidth(IntPtr h) { Rect r; GetWindowRect(h, out r); return r.Right - r.Left; }
    public static int Height(IntPtr h) { Rect r; GetClientRect(h, out r); return r.Bottom; }
    public delegate bool EnumWindow(IntPtr h, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindow callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint id);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h, int id);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint command);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder text, int count);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, string text);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, System.Text.StringBuilder text);
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder text, int count);
    public static IntPtr FindRoot(int process) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((h, p) => {
            uint id; GetWindowThreadProcessId(h, out id);
            var name = new System.Text.StringBuilder(100); GetClassName(h, name, 100);
            if (id == process && name.ToString() == "TodoTxtRustNative") { found = h; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@
function Control-Class([IntPtr]$handle) {
    $text = [Text.StringBuilder]::new(100)
    [void][TodoPreview]::GetClassName($handle, $text, $text.Capacity)
    $text.ToString()
}
function Await-Condition([scriptblock]$condition, [string]$message) {
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    while (![bool](& $condition)) {
        if ([DateTime]::UtcNow -gt $deadline) { throw $message }
        Start-Sleep -Milliseconds 50
    }
}
function Total-RowHeight([IntPtr]$list) {
    $count = [TodoPreview]::SendMessage($list, 0x18B, [IntPtr]::Zero, [IntPtr]::Zero).ToInt32()
    $height = 0
    for ($n = 0; $n -lt $count; $n++) { $height += [TodoPreview]::SendMessage($list, 0x1A1, [IntPtr]$n, [IntPtr]::Zero).ToInt32() }
    $height
}
$previewProfileDirectory = Join-Path $PWD ('.local/preview-' + [Guid]::NewGuid().ToString('N'))
$process = Start-Process -FilePath ([IO.Path]::GetFullPath($Binary)) -ArgumentList '--demo', '--config-dir', ('"' + $previewProfileDirectory + '"') -WindowStyle Hidden -PassThru
try {
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do { Start-Sleep -Milliseconds 100; $root = [TodoPreview]::FindRoot($process.Id) } while ($root -eq 0 -and [DateTime]::UtcNow -lt $deadline)
    if ($root -eq 0) { throw 'Application window did not appear.' }
    # Show the interactive application preview so the native renderer can repaint.
    [void][TodoPreview]::ShowWindow($root, 5)
    Start-Sleep -Seconds 2
    & "$PSScriptRoot/capture-window.ps1" -ProcessId $process.Id -OutputPath 'docs/screenshots/rust.png'
    $editor = [TodoPreview]::GetDlgItem($root, 10)
    $list = [TodoPreview]::GetDlgItem($root, 11)
    $status = [TodoPreview]::GetDlgItem($root, 13)
    if ((Control-Class $editor) -ne 'Edit' -or (Control-Class $list) -ne 'ListBox' -or (Control-Class $status) -ne 'msctls_statusbar32') {
        throw 'Expected native Windows Edit, ListBox, and status controls.'
    }
    # Real window changes must resize child controls and reflow wrapped task rows.
    [void][TodoPreview]::SetWindowPos($root, [IntPtr]::Zero, 0, 0, 850, 700, 0x16)
    Await-Condition { [TodoPreview]::OuterWidth($editor) -eq [TodoPreview]::Width($root) } 'Editor did not resize with the window.'
    $wideHeight = Total-RowHeight $list
    $tallViewport = [TodoPreview]::Height($list)
    [void][TodoPreview]::SetWindowPos($root, [IntPtr]::Zero, 0, 0, 350, 700, 0x16)
    Await-Condition { (Total-RowHeight $list) -gt $wideHeight } 'Task text did not reflow at a narrower width.'
    [void][TodoPreview]::SetWindowPos($root, [IntPtr]::Zero, 0, 0, 350, 300, 0x16)
    Await-Condition { [TodoPreview]::Height($list) -lt $tallViewport } 'Task viewport did not resize with window height.'
    [void][TodoPreview]::SendMessage($root, 0x112, [IntPtr]0xF030, [IntPtr]::Zero)
    Await-Condition { [TodoPreview]::IsZoomed($root) -and [TodoPreview]::OuterWidth($editor) -eq [TodoPreview]::Width($root) } 'Maximize did not relayout controls.'
    [void][TodoPreview]::SendMessage($root, 0x112, [IntPtr]0xF020, [IntPtr]::Zero)
    Await-Condition { [TodoPreview]::IsIconic($root) } 'Taskbar minimization failed.'
    [void][TodoPreview]::SendMessage($root, 0x112, [IntPtr]0xF120, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsIconic($root) -and [TodoPreview]::IsZoomed($root) -and [TodoPreview]::OuterWidth($editor) -eq [TodoPreview]::Width($root) } 'Minimized maximized window did not restore to its previous state.'
    [void][TodoPreview]::SendMessage($root, 0x112, [IntPtr]0xF120, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsIconic($root) -and ![TodoPreview]::IsZoomed($root) } 'Window did not restore.'
    [void][TodoPreview]::SetWindowPos($root, [IntPtr]::Zero, 0, 0, 521, 1047, 0x16)
    # N/Enter runs through the application message loop; no simulated physical input.
    [void][TodoPreview]::SendMessage($root, 0x111, [IntPtr]102, [IntPtr]::Zero)
    [void][TodoPreview]::SendMessage($editor, 0xC, [IntPtr]::Zero, 'Native smoke task +smoke')
    [void][TodoPreview]::PostMessage($editor, 0x100, [IntPtr]13, [IntPtr]::Zero)
    $todo = Join-Path $previewProfileDirectory 'demo/todo.txt'
    Await-Condition { [IO.File]::ReadAllText($todo).Contains('Native smoke task +smoke') } 'Native editor Enter did not save.'
    [void][TodoPreview]::PostMessage($root, 0x111, [IntPtr]107, [IntPtr]::Zero)
    Await-Condition { [IO.File]::ReadAllLines($todo) | Where-Object { $_ -match '^x \d{4}-\d{2}-\d{2} .*Native smoke task' } } 'Native completion did not save.'
    [void][TodoPreview]::PostMessage($root, 0x111, [IntPtr]105, [IntPtr]::Zero)
    Await-Condition { [TodoPreview]::GetDlgItem([TodoPreview]::GetWindow($root, 6), 101) -ne [IntPtr]::Zero } 'Append dialog did not appear.'
    $input = [TodoPreview]::GetDlgItem([TodoPreview]::GetWindow($root, 6), 101)
    [void][TodoPreview]::SendMessage($input, 0xC, [IntPtr]::Zero, 'native-dialog-enter')
    [void][TodoPreview]::PostMessage($input, 0x100, [IntPtr]13, [IntPtr]::Zero)
    Await-Condition { [IO.File]::ReadAllText($todo).Contains('native-dialog-enter') } 'Enter did not accept the native task dialog.'
    # Open the real owned Options window and check its edit/checkbox controls.
    [void][TodoPreview]::PostMessage($root, 0x111, [IntPtr]110, [IntPtr]::Zero)
    Await-Condition { [TodoPreview]::GetDlgItem([TodoPreview]::GetWindow($root, 6), 200) -ne [IntPtr]::Zero } 'Options controls did not appear.'
    $dialog = [TodoPreview]::GetWindow($root, 6)
    if ((Control-Class ([TodoPreview]::GetDlgItem($dialog, 101))) -ne 'Edit' -or (Control-Class ([TodoPreview]::GetDlgItem($dialog, 200))) -ne 'Button') {
        throw 'Options must use native Edit and Button controls.'
    }
    [void][TodoPreview]::PostMessage($dialog, 0x111, [IntPtr]2, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsWindow($dialog) } 'Options did not close.'
    # External edits must trigger a visible native error and preserve the draft.
    [void][TodoPreview]::SendMessage($root, 0x111, [IntPtr]102, [IntPtr]::Zero)
    [void][TodoPreview]::SendMessage($editor, 0xC, [IntPtr]::Zero, 'Retained native draft')
    [IO.File]::AppendAllText($todo, "Externally added task`r`n")
    $externalBytes = [IO.File]::ReadAllText($todo)
    [void][TodoPreview]::PostMessage($editor, 0x100, [IntPtr]13, [IntPtr]::Zero)
    Await-Condition { (Control-Class ([TodoPreview]::GetWindow($root, 6))) -eq '#32770' } 'External-change error dialog did not appear.'
    if ([IO.File]::ReadAllText($todo) -ne $externalBytes) { throw 'External edit was overwritten.' }
    $draft = [Text.StringBuilder]::new(100)
    [void][TodoPreview]::SendMessage($editor, 0xD, [IntPtr]$draft.Capacity, $draft)
    if ($draft.ToString() -ne 'Retained native draft') { throw 'Draft was lost after a refused write.' }
    $errorDialog = [TodoPreview]::GetWindow($root, 6)
    [void][TodoPreview]::PostMessage($errorDialog, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsWindow($errorDialog) } 'Error dialog did not close.'
    if ($process.HasExited) { throw 'Application exited unexpectedly.' }
    Write-Output 'PASS: native controls, width/height reflow, maximize/minimize/restore, Enter/save, completion, dialog Enter, Options, external-change refusal/draft preservation, and screenshot.'
}
finally {
    $process.Refresh()
    if (!$process.HasExited) {
        $root = [TodoPreview]::FindRoot($process.Id)
        $popup = [TodoPreview]::GetWindow($root, 6)
        if ($popup -ne $root -and $popup -ne [IntPtr]::Zero) {
            [void][TodoPreview]::PostMessage($popup, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
            Start-Sleep -Milliseconds 100
        }
        [void][TodoPreview]::PostMessage([TodoPreview]::FindRoot($process.Id), 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
        if (!$process.WaitForExit(5000)) { throw 'Preview did not exit after WM_CLOSE.' }
    }
}
