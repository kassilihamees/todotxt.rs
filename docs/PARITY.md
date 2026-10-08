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

### Current assessment (2026-10-08)

Core task/file functionality is implemented and covered by regression tests;
full application parity has not been achieved. A percentage would imply a
complete feature audit and validation that we do not yet have.

| Area | Current status |
| --- | --- |
| Task editing, completion, priorities/dates, archive, sorts, grouping, filters/presets | Implemented; task/file tests and selected GUI flows pass |
| Windows menus, editor, list, status, owned dialogs | Native controls implemented; remaining WPF metric/dialog differences |
| Windows resize, maximize, taskbar minimize, restore | Native release smoke check passes; task text reflows when wrapping is enabled |
| System tray, minimize-on-close, global Ctrl+Alt+M | Missing; distinct from ordinary taskbar minimization |
| Full font family/style/color selection, filter suggestions, debug-logging option | Missing; current font-size setting and error log are available |
| Printing and calendar | Partial alternatives: browser HTML and a two-week date list |
| Original updater/donation and locale collation | Not reproduced |
| Linux runtime, desktop integration, exact visual/interaction comparison | Still needs validation; portable frontend is present |

Remaining parity work should start with tray/close/hotkey behavior, then the
missing interaction/preferences details and a systematic comparison on both
Windows and Linux. Deferred ideas in `BACKLOG.md` remain out of scope.

Implementation and validation status is recorded in `PROGRESS.md`. The working
port implements the main task/file workflows, eight sorts, grouping, filters,
presets, date/priority shortcuts, suggestions and configurable rendering.

Known gaps: tray/minimize/global hotkey behavior, original updater/donation,
full font selection, debug-logging preference, filter-field suggestions, portable Space acceptance of
suggestions, and original embedded printing/calendar.
Alphabetical collation is deterministic rather than culture-dependent. Blank
lines and untouched whitespace are retained even when hidden. A changed source
refuses writes instead of silently reloading and matching raw task strings.
See the root README for actual feature details and recovery instructions.

Windows now defaults to Win32 menus, EDIT, owner-drawn LISTBOX, status bar,
common file dialogs and owned modal windows with native controls. Styled task
text uses GDI; the portable egui frontend remains the Linux default and an
optional Windows fallback. Preferences and task/file logic are shared. Win32
metrics and wrapping differ from WPF; Linux widgets are not desktop themed.
Visual parity must be compared against captured host screenshots, not assumed
from source alone.
