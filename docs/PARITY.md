# Upstream specification and parity

Reference: https://github.com/benrhughes/todotxt.net
Checkout: `reference/todotxt.net`, dev branch, commit
`c69f334bf6bdbf406f23199172b3b368892b6bae`.

## Interface

MainWindow.xaml defines a standard decorated window, File/Edit/Task/Sort/Filter/
Help menus, a 23-pixel single-line editor, a white raw-text task list with a
10-pixel top margin, and a 23-pixel status bar. Default font: Segoe UI, 12.
Grouping uses bold headers and indented tasks. Completed rows are struck out;
URLs are blue, underlined hyperlinks. There is no toolbar, checkbox column,
sidebar, or card design.

## Behavior

Tasks save immediately. Completion prefixes `x YYYY-MM-DD` and removes priority;
uncompletion removes completion/date without restoring the priority. Sorts:
raw alphabetical, completion, first context, due date, priority, first project,
creation date, file order. Projects/contexts can occur anywhere and are unique
and sorted for metadata. Grouping by project/context displays tasks in all tags.

Filters AND one condition per line; minus prefixes exclude; DONE/-DONE test
completion. due:today/future/past/active are special date filters. Hide future
threshold tasks defaults on; hidden h:1 tasks default off. Presets 1–9; 0 clears.
Relative due/t dates accept today, tomorrow, and abbreviated/full weekdays.

Primary keyboard commands: O open, C new file, N add, U/F2 edit, T append,
X toggle, D/Delete/Backspace delete with confirmation, A archive, J/K move,
F filters, I priority, S due, P postpone, ./F5 reload, ? help. Ctrl+0–7 sorts;
Ctrl+S threshold; Alt arrows priority; Ctrl arrows threshold; Ctrl+Alt arrows due.

## Validation and remaining gaps

### Current assessment (2026-11-10)

Core task/file functionality is implemented and covered by regression tests;
full application parity has not been achieved. A percentage would imply a
complete feature audit and validation that we do not yet have.

The maintainer considers the file save/refresh issue resolved for now as of
2026-11-10; further mount checks remain validation work, not an active blocker.
The following records the earlier failures and safeguards.

v0.1.0 has a reported Windows rclone mounted-save data-loss issue and is marked
with a critical warning. v0.1.1 closes staged writers before replacement,
verifies saved bytes, and keeps local recovery copies. Ten isolated native
mounted completion toggles passed with automatic refresh enabled; the original
failure's precise cause remains unconfirmed. Remote upload durability is not
proved by successful read-back from a mount. See `releases/v0.1.1.md`.
After a further report, both public milestones carry mounted-file warnings.
0.1.2-dev protects against a delayed empty reload and exposes its running
version, but the underlying mounted-save failure remains unresolved. Use an
ordinary local task file while investigating.
The affected computer has since confirmed a native 0.1.2-dev save-verification
failure. Development-machine process/volume observations do not diagnose that
computer. A diagnostic-only probe is described in `STORAGE-DIAGNOSTICS.md`;
its affected-machine fictional-fixture checks passed without reproducing the
failure. The maintainer paused this investigation and removed the probe release.

| Area | Current status |
| --- | --- |
| Task editing, completion, priorities/dates, archive, sorts, grouping, filters/presets | Implemented; task/file tests and selected GUI flows pass |
| Windows menus, editor, list, status, owned dialogs | Native controls implemented; remaining WPF metric/dialog differences |
| Windows resize, maximize, taskbar minimize, restore | Native release smoke check passes; task text reflows when wrapping is enabled |
| Windows system tray, minimize-on-close, global Ctrl+Alt+M | Implemented; native minimize/restore/close/forced-exit and hotkey message routing and local Windows keyboard-input delivery pass; Linux integration remains |
| Font selection, filter suggestions, debug logging | Windows native full font chooser; scrolling ten-field filter form with suggestions; portable size/effects/color and filter suggestions; optional action log |
| Printing and calendar | Finished: maintainer accepts the native Windows printer dialog and shared HTML preview; embedded preview/exact paper layout are excluded. Title-bar calendar matches the seven-day toggle |
| Donations | Finished by maintainer decision: donation UI is intentionally omitted |
| Update notification and locale collation | Original optional version check/website link is not implemented; it is not an automatic installer. Windows culture ordering implemented, Linux deterministic ordering retained |
| Linux runtime, desktop integration, exact visual/interaction comparison | Still needs validation; portable frontend is present |

Remaining parity work includes portable font-family/weight selection, Linux
tray/global hotkey integration, and systematic visual/runtime comparison on both
platforms. The original optional update notification remains unimplemented. Deferred ideas in `BACKLOG.md` remain out of scope.

Implementation and validation status is recorded in `PROGRESS.md`. The working
port implements the main task/file workflows, eight sorts, grouping, filters,
presets, date/priority shortcuts, suggestions and configurable rendering.

Known gaps: optional update notification, full portable font-family/weight
selection, Linux tray/global hotkey and
locale-dependent collation, desktop theming and exact WPF metrics.
Blank lines and untouched whitespace are retained even when hidden. A changed source
refuses writes instead of silently reloading and matching raw task strings.
See the root README for actual feature details and recovery instructions.

Windows now defaults to Win32 menus, EDIT, owner-drawn LISTBOX, status bar,
common file dialogs and owned modal windows with native controls. Styled task
text uses GDI; the portable egui frontend remains the Linux default and an
optional Windows fallback. Preferences and task/file logic are shared. Win32
metrics and wrapping differ from WPF; Linux widgets are not desktop themed.
Visual parity must be compared against captured host screenshots, not assumed
from source alone.

Windows-specific validation: the smoke script opens/cancels the real font and
printer dialogs, accepts a filter suggestion with Space, persists tray/font/log
preferences, minimizes to the tray, restores through the icon callback, closes
to the tray, restores through WM_HOTKEY, and exits through File > Exit. This
verifies hotkey routing. Local interactive runs with `-PhysicalHotkey` also use
Windows SendInput to verify Ctrl+Alt+M hide/restore through OS keyboard delivery.
The actual notification-area right-click menu's Exit item is inspected and
selected while the window is hidden; both tray Exit and File Exit quit despite
minimize-on-close. Explorer-restart recovery and physical printing still need
validation.

The notification icon now requests Windows version-4 callbacks and handles both
packed modern and legacy events, following [Microsoft's notification icon
specification](https://learn.microsoft.com/windows/win32/api/shellapi/ns-shellapi-notifyicondataw).

## Accepted differences (2026-10-09)

The maintainer marked printing and donations finished. Keep the current native
Windows print dialog and browser HTML preview; do not recreate the embedded
preview. Exact upstream paper layout is not a parity requirement. A donation
menu is intentionally omitted and is not outstanding work.

The original UpdateChecker optionally fetches Updates.xml at startup, compares
the advertised version and exposes a website link when it differs from the
running version (the upstream check does not compare version ordering).
It does not download or install an update. The Rust port currently performs no
application-version checks; this is separate from refreshing changed task files.

## Shortcut and mounted-refresh fixes (2026-10-10)

Windows now routes WM_SYSKEYDOWN as well as WM_KEYDOWN: Alt priority shortcuts
and Ctrl+Alt due/defer shortcuts can reach the application instead of being
skipped as system keys. Unhandled system keys still reach Windows menus.
Numeric-keypad filter presets and sorts are recognized. Alt+F4 invokes Exit
even when minimize-on-close is enabled. Portable Alt+Left/Right now remove
priority as documented. Right Shift retains its existing scan-code handling.
Ctrl+D remains a deferred request; Ctrl+Shift+C remains duplication.

Both frontends poll size/mtime instead of reading unchanged contents each second.
Manual reload and full byte comparisons before saving remain intact. Metadata
caching and edits preserving size/mtime can delay idle detection; these checks
are not a remote upload guarantee. The mounted-save corruption report remains
unresolved, independent of the idle polling change.

Shift+9 now begins a new task with a priority prefix and suggestions in both
frontends. Positive filter text is preserved after the prefix, with the caret
after the opening parenthesis, matching upstream. Editor-focused list shortcuts
remain native text editing, as documented in Help.

## Configurable save method

0.1.4-dev adds a shared Options > Save method selector to native Windows and the
portable frontend. Temporary-file replacement remains the default for old and
new preferences. Direct write (rclone compatibility) avoids temporary files and
renames beside the task/archive, matching upstream StreamWriter's in-place save.
Both methods require local recovery copies, check external bytes before writes,
and verify saved bytes. Direct writes can leave partial files on interruption;
recovery remains manual. File settings/recovery copies still use replacement.
The option applies to open/newly opened documents, archive destinations, reloads
and restarts. Successful local tests do not establish remote cloud durability
or confirm that the affected computer's rclone failure is resolved.

## Self-archive aliases (2026-11-08)

0.1.5-dev fixes a reproduced Windows self-archive failure with short (8.3) versus
long path aliases. Text-only comparison missed that both paths named the same
file, so archive append happened before the source-change check failed. Archive
selection now additionally compares canonical paths and available filesystem
identities (128-bit Windows file IDs, Unix device/inode). Unsupported mounted
metadata is optional. Both save methods reject short-path and hard-link aliases
before creating recovery copies or changing archive/source contents.

## Suggestion and date-dialog parity (2026-11-10)

Windows suggestion lists clip sibling painting so the task list cannot paint
through the dropdown. Dropdown height uses the actual font's row metrics plus
its border, rather than a fixed row estimate. The same sizing applies to filter
suggestions.

Upstream SetDueDateDialog uses a WPF DatePicker, not a plain text prompt.
Windows due/threshold dialogs now use the native date/time picker with a calendar
dropdown, ISO date display, T for today and Up/Down for one-day changes when the
calendar is closed. Enter accepts and Escape cancels; the open calendar keeps
its native navigation. Portable date dialogs still use text input; calendar
selection there is explicitly outstanding.
