# todotxt.rs

A native Rust desktop port of [todotxt.net](https://github.com/benrhughes/todotxt.net),
targeting Windows and Linux. It follows the original's compact menu bar, one-line
editor, raw-text task list, keyboard navigation, grouping, and status counts.
This is a working first port; the remaining differences are listed below.

**Windows uses Win32 controls by default:** OS menus, a standard edit box with
Windows text selection and undo, native list selection and scrollbars, a status
bar, native file pickers, and separate owned Options/filter/task dialogs. Only
the styled task text is custom drawn with GDI. The executable includes the
upstream icon, Common Controls v6 manifest, and per-monitor DPI support.
Linux retains the portable egui frontend. Both use the same Rust task/file
library and preference format.
Windows controls resize with the window, including maximize/restore. Task text
reflows when word wrap is enabled; ordinary taskbar minimization is supported.
System-tray minimization remains a parity gap.

## Run

Install stable Rust. On Windows, use the MSVC Rust toolchain and Visual Studio
Build Tools with the C++ workload. This workspace already includes a Cargo
package, lockfile, toolchain configuration, and formatting/lint checks.

```sh
cargo run --locked
cargo run --locked -- --demo
cargo run --locked -- "/path/to/todo.txt"
cargo build --locked --release
```

Windows executable: `target/release/todotxt-rs.exe`.
Linux executable: `target/release/todotxt-rs`.
No .NET runtime is required. Windows builds need no OpenGL or GPU UI dependency.
The Linux frontend requires a graphical desktop and OpenGL.

The previous portable interface is an optional Windows fallback:

```sh
cargo run --locked --features portable-ui -- --portable-ui
```

Enabling the feature includes its dependencies; `--portable-ui` selects that
frontend. An ordinary Windows build excludes egui, eframe, rfd, and arboard.

On Debian/Ubuntu, install the desktop build libraries:

```sh
sudo apt install build-essential pkg-config libx11-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev
```

Linux file dialogs use the desktop's XDG portal. Install `xdg-desktop-portal`
and the appropriate backend for your desktop if file dialogs do not appear.

With no filename, the app reopens the last successfully opened file. On the
first run it creates a **writable private copy** of the fictional `fixtures/demo.todo` in its
settings directory and opens it with priority grouping and wrapping, matching
the captured original's layout. `--demo` reopens that private copy; it does not
reset it. Existing demo copies are not replaced automatically, since they may
contain edits you want to keep. Your running todotxt.net file is untouched unless
you explicitly open it in this app and edit it. Use a fresh `--config-dir` for a
new fictional demo. Never commit real task files or screenshots of personal tasks.

For isolated settings and demo data:

```sh
cargo run --locked -- --demo --config-dir .local/my-profile
```

## Implemented features

- Open/create todo files; add, update, append, delete with confirmation, and
  toggle completion. Changes save immediately.
- Multiple selection with Ctrl-click and Shift-click; select all, copy, paste
  one task per line, and duplicate a task into the editor.
- Priorities A–Z, due dates, threshold dates, date shifting and postponement.
  Relative dates accept `today`, `tomorrow`, and full/abbreviated weekdays.
- Project/context/priority completion suggestions in the task editor.
- Eight sorts: file order, alphabetical, completion, context, due date,
  creation date, priority, project. Sorting affects the display, not file order.
- Optional grouping; tasks with multiple projects/contexts appear in each
  group. Counts include each physical task once.
- Multiline AND filters, exclusion conditions, nine presets, completion and
  due-date filters, hidden tasks, and future threshold filtering.
- Alternating rows, completed-task fading/strikethrough, green due-today tasks,
  red overdue tasks, clickable blue URLs, optional wrapping, and status counts.
- Manual/automatic archiving, optional creation dates, Ctrl-Enter mode,
  automatic refresh, font appearance, grouping, wrapping, status-bar and debug-log preferences.
- Persistent file/preferences and window size/position; error log, help,
  title-bar calendar and printable date/details tables. Windows also has tray mode and a native printer dialog.

Completion follows the original: it removes priority and prefixes
`x YYYY-MM-DD`. Reopening a completed task removes completion/date and does not
restore its old priority. Dates use the machine's local calendar date. A named
weekday means its next occurrence, even if that is seven days from today.
`rec:` and other unrecognized metadata remain text; there is no recurrence engine.

## Keyboard controls

List shortcuts apply while the editor is unfocused. Typing in the editor is
ordinary text entry. Enter saves; Options can require Ctrl-Enter. Escape cancels
editing. Up/Down selects a completion suggestion; Tab or Enter accepts it,
then Enter saves the task.

| Action | Key |
| --- | --- |
| Open / create file | O or Ctrl+O / C or Ctrl+N |
| New / edit task | N / U, F2, or double-click |
| Append / complete / delete | T / X / D, Delete, or Backspace |
| Archive / reload | A / . or F5 |
| Next / previous | J / K, or arrow keys |
| Select all / copy / paste | Ctrl+A / Ctrl+C / Ctrl+V |
| Copy one task to editor | Ctrl+Shift+C |
| Filters / clear / presets | F / 0 / 1–9 |
| Hide future / show hidden | Ctrl+T / Ctrl+H |
| Set priority | I |
| Raise / lower / clear priority | Alt+Up / Alt+Down / Alt+Left or Right |
| Set due / threshold date | S / Ctrl+S |
| Postpone due / threshold date | P / Ctrl+Alt+P |
| Shift due date ±1 day | Ctrl+Alt+Up or Down |
| Shift threshold date ±1 day | Ctrl+Up or Down |
| Remove due / threshold date | Ctrl+Alt+Left or Right / Ctrl+Left or Right |
| Sort file/alphabetical/completed/context/due/created/priority/project | Ctrl+0–7 |
| Options / help | F10 / ? |
| Print / print preview (Windows) | Ctrl+P / Ctrl+Shift+P |
| Toggle title-bar calendar (Windows task list) | Right Shift |
| Restore/minimize window (Windows tray mode) | Ctrl+Alt+M |

## Filters

Use one condition per line. All conditions must match. Text search is
case-insensitive unless enabled in Options; prefix `-` to exclude matches.

```text
+example1
-DONE
-@waiting
due:active
```

This shows incomplete `+example1` tasks that lack `@waiting` and have a due date on
or before today. `DONE` and `-DONE` are uppercase completion tests.
`due:today`, `due:future`, `due:past`, and `due:active` are special date tests;
minus prefixes invert them. `due:active` requires a due date.

Filters currently support AND only. Putting `+projectA` and `+projectB` on
separate lines requires both projects on the same task. There is no OR operator
or regex alternative syntax yet; OR support is recorded in the deferred backlog.

Hide future tasks is enabled initially: `t:` dates later than today are hidden.
`h:1` tasks are hidden until Show hidden tasks is enabled. Clearing the text
filter with `0` does not disable these two switches.

## File safety and recovery

The todo file must be UTF-8, with or without a UTF-8 BOM. Existing LF/CRLF
line endings, final-newline presence, blank lines, untouched whitespace, tag
placement, and physical task order are preserved. Duplicate task lines can be
edited independently. Normal editor submissions trim surrounding whitespace;
Options can preserve it. Blank lines can be displayed or hidden without
removing them from the file.

**If the source file has changed outside the app, writes are refused.** Before
a write, the current bytes are compared with the bytes originally loaded.
The error appears in a dialog and is logged. The file remains as the external
writer left it, and your editor draft is retained.

To recover:

1. Copy any draft text you need to keep and dismiss the error.
2. Press F5 or choose File → Reload File.
3. Reselect the task and reapply your change. Reload clears the old edit target;
   retained editor text will create a new task rather than overwrite a new line.

Automatic refresh is optional and initially disabled. When enabled, it reloads
external changes about once a second while the editor is empty and no dialog
is open. It pauses while a draft is being entered.

Writes use a synced temporary file in the same directory followed by replacement,
so a failed write does not leave a partially written todo file. Existing file
permission flags are retained (Unix mode bits or the Windows read-only flag);
replacement does not preserve every OS-specific attribute or ACL. This is
conflict detection, not a lock shared with
other editors or sync tools: there is still a small race between the comparison
and replacement. Keep normal backups for sync workflows.

Archiving appends completed tasks to the chosen archive, then removes them from
the source. Automatically select archive path chooses `done.txt` beside the
todo file. Source and archive must be different files. The two-file operation
is not atomic: if source removal fails after the archive write succeeds, the
dialog explains that completed tasks remain in both files. Resolve those
duplicates before archiving again. An archive failure leaves the source intact.

Creating a file at an existing path opens that file without clearing it.

## Settings and logs

Default settings directories:

- Windows: `%APPDATA%\todotxt.rs\config`
- Linux: `$XDG_CONFIG_HOME/todotxt.rs` or `~/.config/todotxt.rs`

`settings.json` stores preferences, `error.log` records failures, `demo/todo.txt`
holds the demo, and `print-preview.html` is the last printable view. The Windows
frontend stores geometry as `native-window.json` in the settings directory.
The portable frontend saves geometry separately in its application data
directory, or as `window.ron` under an explicit `--config-dir`.
Preferences from the original .NET application are not imported automatically.

## Current fidelity limits

Windows uses Win32 rather than the original WPF. Its menus, editing, selection,
scrollbars, and dialogs follow Windows behavior; text uses Segoe UI through GDI.
Task wrapping and font rasterization can differ from WPF. Linux uses egui with
installed Noto Sans or DejaVu Sans, then embedded fonts; portable task dialogs
remain inside the main window. Linux widgets do not yet follow a desktop theme.

Windows tray mode, minimize-on-close, Ctrl+Alt+M, full native font selection,
filter-field suggestions, and the optional debug log are implemented. Enable
tray mode in Options; close-to-tray applies only while its icon is available.
Double-click the icon or press Ctrl+Alt+M to restore. File > Exit or the tray's
Exit always quits. Right-click the notification-area icon near the clock to
open its Exit menu. A shortcut already owned by todotxt.net or another program
produces a visible error; the tray remains usable. The icon is recreated after
Explorer restarts, with window restoration if that fails.

The calendar toggles seven dates in the title bar. Both frontends accept task
and filter suggestions with Tab, Enter, or Space. Windows uses the native font
chooser for family, weight/style, size, color, underline and strikeout; portable
Options supports size, color, italic, underline and strikeout, with system font
fallbacks. Debug logging records action names/IDs rather than tasks, drafts,
filters or paths; error messages can still include file paths.

Windows Ctrl+P opens the native printer dialog and prints the visible sorted,
filtered, grouped tasks in a Done/Created/Due/Details table. Ctrl+Shift+P opens
HTML preview in the browser. Linux uses the same printable HTML table. The
maintainer accepts this printing solution as finished; recreating the original
embedded preview or exact paper layout is out of scope. Printer cancellation
was exercised; physical printing has not been validated.

Donations are closed by maintainer decision; no donation menu is planned.
Linux tray/global hotkey integration and full portable font-family selection
remain outstanding. The original optional update notification is not implemented:
it checked a published application version and offered a website link, without
downloading or installing updates. This is unrelated to todo.txt auto-refresh.
Windows alphabetical/project/context ordering now uses the user's Windows
locale; Linux retains deterministic lowercase ordering.
Some malformed-text parsing quirks are not
intentionally reproduced.

Mounted drives (including rclone/WinFsp) can provide readable files while rejecting
Windows final-path queries. Opening a document now keeps an absolute path when
canonicalization is unavailable; ordinary reads still report genuine IO errors.
The fix was verified read-only against an rclone-mounted source. External-change
checks and atomic replacement remain in place; writes still require the mount
backend to support the save operations. See the [corresponding Windows mount
compatibility report](https://github.com/microsoft/edit/issues/947).

## Verification and project layout

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Tests cover task parsing, date/completion changes, all sorts, multi-tag grouping,
filters, the fictional demo fixture, Unicode, BOM/newlines/blank lines, duplicate identity,
archive behavior and refused external writes. GUI event tests exercise add/edit,
completion, delete confirmation/cancellation, suggestions, preserved drafts,
reload safety, and status rendering (`cargo test --features portable-ui` on
Windows). `scripts/smoke-windows.ps1` exercises actual Win32 controls, editor
Enter/save, task completion, native dialog Enter, owned Options/font dialogs, filter suggestions, printer cancellation, tray
minimize/restore/close, calendar, and external change refusal with retained drafts against an isolated fictional fixture copy, and captures
a screenshot. It also verifies width/height reflow, maximize, taskbar minimize,
and restore, plus right-click tray Exit while hidden. Add `-PhysicalHotkey`
to verify Ctrl+Alt+M through Windows keyboard input on an interactive desktop.
See [the parity assessment](docs/PARITY.md) for remaining work.
Linux CI is configured; a Linux desktop
run still needs verification.

| Location | Purpose |
| --- | --- |
| `src/task.rs`, `src/view.rs`, `src/document.rs` | Toolkit-independent library |
| `src/windows/`, `src/desktop.rs`, `src/main.rs` | Win32 UI, portable UI, launcher |
| `src/model.rs`, `src/settings.rs` | Windows controller and shared preferences |
| `tests/model.rs` | Draft recovery, duplicate identity, shared settings |
| `tests/parity.rs` | Upstream-derived regression cases and file tests |
| `fixtures/demo.todo` | Fictional demo data safe to distribute |
| `reference/todotxt.net` | Unmodified upstream checkout, ignored by root Git |
| `AGENTS.md` | Instructions for future work |
| `docs/HELP.md` | Help embedded in the application |
| `docs/PARITY.md`, `docs/PROGRESS.md` | Specification, gaps, verification status |
| `docs/BACKLOG.md` | Future ideas deferred until original-app parity |
| `docs/screenshots` | Host-window visual references |

To restore the reference checkout in a fresh clone:

```sh
git clone https://github.com/benrhughes/todotxt.net.git reference/todotxt.net
git -C reference/todotxt.net checkout c69f334bf6bdbf406f23199172b3b368892b6bae
```

The port translates behavior from Ben Hughes's BSD-licensed source and reuses
its icon. The upstream copyright notice and redistribution terms are retained
in [LICENSE](LICENSE).
