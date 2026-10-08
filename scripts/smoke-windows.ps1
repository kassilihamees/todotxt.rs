param([string]$Binary = 'target/debug/todotxt-rs.exe', [switch]$PhysicalHotkey)
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
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
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
    [DllImport("user32.dll")] public static extern uint SendInput(uint count, Input[] inputs, int size);
    [StructLayout(LayoutKind.Sequential)] public struct KeyboardInput { public ushort Key, Scan; public uint Flags, Time; public UIntPtr Extra; }
    [StructLayout(LayoutKind.Sequential)] public struct MouseInput { public int X, Y; public uint Data, Flags, Time; public UIntPtr Extra; }
    [StructLayout(LayoutKind.Explicit)] public struct InputData { [FieldOffset(0)] public KeyboardInput Keyboard; [FieldOffset(0)] public MouseInput Mouse; }
    [StructLayout(LayoutKind.Sequential)] public struct Input { public uint Type; public InputData Data; }
    public static bool CtrlAltM() {
        var inputs = new Input[6];
        ushort[] keys = { 0x11, 0x12, 0x4d, 0x4d, 0x12, 0x11 };
        for (int n = 0; n < inputs.Length; n++) { inputs[n].Type = 1; inputs[n].Data.Keyboard.Key = keys[n]; inputs[n].Data.Keyboard.Flags = n >= 3 ? 2u : 0u; }
        return SendInput(6, inputs, Marshal.SizeOf(typeof(Input))) == 6;
    }
    [DllImport("user32.dll")] public static extern int GetMenuItemCount(IntPtr menu);
    [DllImport("user32.dll")] public static extern uint GetMenuItemID(IntPtr menu, int position);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetMenuString(IntPtr menu, uint position, System.Text.StringBuilder text, int count, uint flags);
    public static IntPtr FindMenu(int process) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((h, p) => {
            uint id; GetWindowThreadProcessId(h, out id);
            var name = new System.Text.StringBuilder(100); GetClassName(h, name, 100);
            if (id == process && IsWindowVisible(h) && name.ToString() == "#32768") { found = h; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static IntPtr FindPopup(int process) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((h, p) => {
            uint id; GetWindowThreadProcessId(h, out id);
            var name = new System.Text.StringBuilder(100); GetClassName(h, name, 100);
            if (id == process && IsWindowVisible(h) && name.ToString() != "TodoTxtRustNative") { found = h; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
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
    Write-Output "PASS: original native smoke flows. Starting filter form checks."
    # Filter fields and suggestions operate on the actual native controls.
    [void][TodoPreview]::PostMessage($root, 0x111, [IntPtr]111, [IntPtr]::Zero)
    Await-Condition { [TodoPreview]::GetDlgItem([TodoPreview]::GetWindow($root, 6), 102) -ne [IntPtr]::Zero } 'Filter form did not appear.'
    $dialog = [TodoPreview]::GetWindow($root, 6)
    $panel = [TodoPreview]::GetDlgItem($dialog, 102)
    $filter = [TodoPreview]::GetDlgItem($panel, 600)
    if ([TodoPreview]::GetDlgItem($panel, 609) -eq [IntPtr]::Zero) { throw 'Filter preset 9 is missing.' }
    [void][TodoPreview]::SendMessage($filter, 0xC, [IntPtr]::Zero, '+de')
    [void][TodoPreview]::SendMessage($filter, 0xB1, [IntPtr]3, [IntPtr]3)
    # Explicit change notification after caret placement (WM_SETTEXT can notify before updating it).
    [void][TodoPreview]::SendMessage($dialog, 0x111, [IntPtr](600 + (0x300 -shl 16)), $filter)
    $suggestions = [TodoPreview]::GetDlgItem($dialog, 109)
    Await-Condition { [TodoPreview]::IsWindowVisible($suggestions) } 'Filter suggestions did not appear.'
    [void][TodoPreview]::PostMessage($filter, 0x100, [IntPtr]32, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsWindowVisible($suggestions) } 'Space did not accept the filter suggestion.'
    $filterText = [Text.StringBuilder]::new(100)
    [void][TodoPreview]::SendMessage($filter, 0xD, [IntPtr]$filterText.Capacity, $filterText)
    if ($filterText.ToString() -ne '+demo') { throw 'Filter completion inserted the wrong tag.' }
    [void][TodoPreview]::SendMessage($panel, 0x115, [IntPtr]7, [IntPtr]::Zero)
    [void][TodoPreview]::SendMessage([TodoPreview]::GetDlgItem($panel, 609), 0xC, [IntPtr]::Zero, '-DONE')
    [void][TodoPreview]::PostMessage($dialog, 0x111, [IntPtr]1, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsWindow($dialog) } 'Filter OK did not close.'
    Await-Condition { $filters = Get-Content (Join-Path $previewProfileDirectory 'settings.json') -Raw | ConvertFrom-Json; $filters.filter -eq '+demo' -and $filters.presets[8] -eq '-DONE' } 'Active filter and preset 9 did not save.'
    [void][TodoPreview]::SendMessage($root, 0x111, [IntPtr]400, [IntPtr]::Zero)
    Write-Output "PASS: filter suggestions, scrolling and saved preset 9. Starting printer checks."
    # A printer dialog can be opened/cancelled without submitting a print job.
    [void][TodoPreview]::PostMessage($root, 0x111, [IntPtr]113, [IntPtr]::Zero)
    Await-Condition { (Control-Class ([TodoPreview]::GetWindow($root, 6))) -eq '#32770' } 'Native print dialog did not appear.'
    $printDialog = [TodoPreview]::GetWindow($root, 6)
    [void][TodoPreview]::PostMessage($printDialog, 0x111, [IntPtr]2, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsWindow($printDialog) } 'Print dialog Cancel did not close.'
    Write-Output "PASS: printer cancellation. Starting font and tray checks."
    # Font chooser, settings persistence, tray minimize/restore, close-to-tray and forced Exit.
    [void][TodoPreview]::PostMessage($root, 0x111, [IntPtr]110, [IntPtr]::Zero)
    Await-Condition { [TodoPreview]::GetDlgItem([TodoPreview]::GetWindow($root, 6), 108) -ne [IntPtr]::Zero } 'Font chooser button is missing.'
    $dialog = [TodoPreview]::GetWindow($root, 6)
    [void][TodoPreview]::PostMessage($dialog, 0x111, [IntPtr]108, [IntPtr]::Zero)
    Await-Condition { (Control-Class ([TodoPreview]::GetWindow($dialog, 6))) -eq '#32770' } 'Native font dialog did not appear.'
    $fontDialog = [TodoPreview]::GetWindow($dialog, 6)
    [void][TodoPreview]::PostMessage($fontDialog, 0x111, [IntPtr]2, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsWindow($fontDialog) } 'Font dialog Cancel did not close.'
    [void][TodoPreview]::SendMessage([TodoPreview]::GetDlgItem($dialog, 107), 0xC, [IntPtr]::Zero, '16')
    foreach ($id in 212, 213, 214) { [void][TodoPreview]::SendMessage([TodoPreview]::GetDlgItem($dialog, $id), 0xF1, [IntPtr]1, [IntPtr]::Zero) }
    [void][TodoPreview]::PostMessage($dialog, 0x111, [IntPtr]1, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsWindow($dialog) } 'Options OK did not close.'
    Await-Condition {
        $saved = Get-Content (Join-Path $previewProfileDirectory 'settings.json') -Raw | ConvertFrom-Json
        $saved.minimize_to_tray -and $saved.minimize_on_close -and $saved.debug_logging -and $saved.font_size -eq 16
    } 'Native preferences did not persist.'
    Start-Sleep -Milliseconds 250
    # The running original may own Ctrl+Alt+M; this must surface an error without disabling the tray.
    $popup = [TodoPreview]::FindPopup($process.Id)
    $hotkeyAvailable = (Control-Class $popup) -ne '#32770'
    if ((Control-Class $popup) -eq '#32770') { [void][TodoPreview]::PostMessage($popup, 0x10, [IntPtr]::Zero, [IntPtr]::Zero); Await-Condition { ![TodoPreview]::IsWindow($popup) } 'Hotkey conflict error did not close.' }
    [void][TodoPreview]::SendMessage($root, 0x112, [IntPtr]0xF020, [IntPtr]::Zero)
    Await-Condition { [TodoPreview]::IsIconic($root) -and ![TodoPreview]::IsWindowVisible($root) } 'Tray minimization did not hide the taskbar window.'
    [void][TodoPreview]::PostMessage($root, 0x8002, [IntPtr]1, [IntPtr]0x203)
    Await-Condition { [TodoPreview]::IsWindowVisible($root) -and ![TodoPreview]::IsIconic($root) } 'Tray double-click did not restore the window.'
    [void][TodoPreview]::PostMessage($root, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsWindowVisible($root) -and !$process.HasExited } 'Close-to-tray did not retain the application.'
    [void][TodoPreview]::PostMessage($root, 0x312, [IntPtr]1, [IntPtr]::Zero)
    Await-Condition { [TodoPreview]::IsWindowVisible($root) } 'Global hotkey message did not restore the window.'
    if ($PhysicalHotkey -and $hotkeyAvailable) {
        if (![TodoPreview]::CtrlAltM()) { throw 'Windows refused Ctrl+Alt+M input injection.' }
        Await-Condition { ![TodoPreview]::IsWindowVisible($root) } 'Actual Ctrl+Alt+M did not hide the window.'
        if (![TodoPreview]::CtrlAltM()) { throw 'Windows refused Ctrl+Alt+M input injection.' }
        Await-Condition { [TodoPreview]::IsWindowVisible($root) } 'Actual Ctrl+Alt+M did not restore the window.'
        Write-Output 'PASS: actual Ctrl+Alt+M input hid and restored the isolated application.'
    } elseif ($PhysicalHotkey) { Write-Output 'SKIP: actual Ctrl+Alt+M input; another application owns the shortcut.' }
    [void][TodoPreview]::PostMessage($root, 0x111, [IntPtr]134, [IntPtr]::Zero)
    $title = [Text.StringBuilder]::new(300)
    Await-Condition { [void][TodoPreview]::GetWindowText($root, $title, $title.Capacity); $title.ToString().Contains('Calendar:') } 'Calendar did not update the title.'
    [void][TodoPreview]::PostMessage($root, 0x111, [IntPtr]134, [IntPtr]::Zero)
    Await-Condition { [void][TodoPreview]::GetWindowText($root, $title, $title.Capacity); $title.ToString() -eq 'todotxt.rs' } 'Calendar did not toggle off.'
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
    $log = [IO.File]::ReadAllText((Join-Path $previewProfileDirectory 'error.log'))
    if (!$log.Contains('DEBUG command 134') -or $log.Contains('Retained native draft')) { throw 'Debug events are missing or include draft text.' }
    [void][TodoPreview]::PostMessage($root, 0x111, [IntPtr]133, [IntPtr]::Zero)
    Await-Condition { $process.HasExited } 'File Exit did not quit with minimize-on-close enabled.'
    # Restart the isolated application to test tray Exit independently of File Exit.
    $process = Start-Process -FilePath ([IO.Path]::GetFullPath($Binary)) -ArgumentList '--demo', '--config-dir', ('"' + $previewProfileDirectory + '"') -WindowStyle Hidden -PassThru
    Await-Condition { $root = [TodoPreview]::FindRoot($process.Id); $root -ne [IntPtr]::Zero } 'Tray Exit test window did not appear.'
    $root = [TodoPreview]::FindRoot($process.Id)
    [void][TodoPreview]::ShowWindow($root, 5)
    # Right-click must offer a real Exit entry even while the owner is hidden.
    [void][TodoPreview]::SendMessage($root, 0x112, [IntPtr]0xF020, [IntPtr]::Zero)
    Await-Condition { ![TodoPreview]::IsWindowVisible($root) } 'Window did not hide before the tray menu check.'
    # Modern Shell callback: icon ID in the high word, WM_CONTEXTMENU in the low word.
    [void][TodoPreview]::PostMessage($root, 0x8002, [IntPtr]::Zero, [IntPtr](0x10000 + 0x7B))
    Await-Condition { [TodoPreview]::FindMenu($process.Id) -ne [IntPtr]::Zero } 'Tray right-click menu did not appear.'
    $menuWindow = [TodoPreview]::FindMenu($process.Id)
    $menu = [TodoPreview]::SendMessage($menuWindow, 0x1E1, [IntPtr]::Zero, [IntPtr]::Zero)
    $menuLabel = [Text.StringBuilder]::new(100)
    [void][TodoPreview]::GetMenuString($menu, 0, $menuLabel, $menuLabel.Capacity, 0x400)
    if ([TodoPreview]::GetMenuItemID($menu, 0) -ne 133 -or $menuLabel.ToString().Replace('&', '') -ne 'Exit') { throw 'Tray menu is missing Exit.' }
    [void][TodoPreview]::PostMessage($root, 0x100, [IntPtr]40, [IntPtr]::Zero)
    [void][TodoPreview]::PostMessage($root, 0x100, [IntPtr]13, [IntPtr]::Zero)
    Await-Condition { $process.HasExited } 'Tray Exit did not quit with minimize-on-close enabled.'
    Write-Output 'PASS: native controls, width/height reflow, maximize/minimize/restore, Enter/save, completion, dialog Enter, Options/font dialog, filter suggestions, printer cancellation, tray minimize/restore/close, hotkey routing, tray right-click/Exit, calendar, debug logging, forced Exit, external-change refusal/draft preservation, and screenshot.'
}
catch { Write-Output ("SMOKE FAILED: " + $_.Exception.Message); throw }
finally {
    $process.Refresh()
    if (!$process.HasExited) {
        $root = [TodoPreview]::FindRoot($process.Id)
        for ($cleanupAttempt = 0; $cleanupAttempt -lt 4; $cleanupAttempt++) {
            $popup = [TodoPreview]::FindPopup($process.Id)
            if ($popup -eq [IntPtr]::Zero) { break }
            [void][TodoPreview]::PostMessage($popup, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
            Start-Sleep -Milliseconds 200
        }
        [void][TodoPreview]::PostMessage([TodoPreview]::FindRoot($process.Id), 0x111, [IntPtr]133, [IntPtr]::Zero)
        if (!$process.WaitForExit(5000)) { Write-Warning 'Isolated preview could not exit through File Exit; stopping this test process.'; $process.Kill() }
    }
}
