Work with tasks

N adds a task. Type in the editor above the list and press Enter to save.
U or F2 edits one selected task; double-click also edits. Escape cancels editing.
T appends text to selected tasks. X toggles completion; completed tasks lose their priority.
D, Delete, or Backspace asks before deleting. A archives completed tasks.
J/K or arrow keys move the selection. Ctrl-click toggles rows; Shift-click selects a range.
Ctrl+C copies selected tasks; Ctrl+Shift+C copies one task into the new-task editor.
Ctrl+V in the list pastes one task per line. Ctrl+A selects all visible tasks.

Priorities and dates

I sets priority A–Z; blank removes it. Alt+Up/Down raises/lowers priority.
Alt+Left/Right removes priority. S sets due date; Ctrl+S sets threshold date.
P postpones due date; Ctrl+Alt+P postpones threshold date.
Ctrl+Alt+Up/Down shifts due date; Ctrl+Up/Down shifts threshold date.
Left/Right with the same modifiers removes the date.
Dates accept YYYY-MM-DD, today, tomorrow, and weekdays (mon or monday, etc.).
The next named weekday is always in the future, including seven days if today matches.
Typing +, @ or ( shows project, context or priority suggestions. Up/Down selects;
Tab, Enter or Space accepts a suggestion.
Press Enter again to save the task. Windows uses native edit controls for
selection, clipboard commands, and Ctrl+Z undo; Alt activates the native menus.

Filters and files

F opens filter definitions. Each line is an AND condition; prefix - to exclude.
DONE/-DONE select complete/incomplete. due:today, due:past, due:future and
due:active filter due dates. 1–9 apply saved presets; 0 clears the text filter.
Ctrl+T hides future threshold tasks. Ctrl+H shows h:1 hidden tasks.
Ctrl+0–7 selects file/alphabetical/completed/context/due/created/priority/project sort.
O or Ctrl+O opens a file. C or Ctrl+N creates a file. . or F5 reloads it.
F10 opens Options. ? opens this help. Windows Ctrl+P opens the native printer
chooser; Ctrl+Shift+P opens printable HTML preview. Linux Ctrl+P opens the HTML
table in your browser. Print uses the visible tasks and current sort/groups.
Help ? Show Calendar toggles seven dates in the title bar. On the Windows task
list, Right Shift also toggles it.

Windows tray and appearance

Options can enable system-tray minimization and minimize-on-close. The latter
requires a working tray icon. Double-click the icon or press Ctrl+Alt+M to show
or hide the window. If another app owns this shortcut, an error is shown and
the icon remains usable. File > Exit and the tray's Exit quit the application.
Options ? Select Font chooses the Windows task font, style, size and color.
Completion/due colors and hyperlink styling override the ordinary text color.
The portable frontend supports size, color, italic, underline and strikeout.
Optional debug logging writes action names/IDs to error.log in the settings
folder, without copying task text or drafts. Error messages can include paths.
Filter fields offer the same project/context/priority suggestions as the task
editor. Use Enter for another filter line after accepting a suggestion.

File safety

Edits save immediately. The file must be UTF-8. Newlines, a UTF-8 BOM, blank lines,
untouched whitespace, and physical task order are preserved.
If another program changes the source file, a write is refused and editor text is kept.
Copy your draft if needed, reload with F5, reselect the task, then reapply your edit.
Reload cancels the edit target so an old line number cannot overwrite a different task.
Options can enable automatic refresh when the editor is empty.
Saving closes and verifies the staged file, rechecks the source, and verifies
the resulting destination. Local recovery copies retain previous and intended
task contents before replacement. On Windows these are under
%LOCALAPPDATA%\todotxt.rs\data\recovery. A failed save keeps the in-memory
tasks, pauses automatic refresh, and reports the recovery folder. Check or
restore the disk file before explicitly reloading. Cloud uploads can be delayed;
verification of the mounted view does not verify eventual remote storage.
Windows Options archive selection only sets a destination; it leaves existing
contents unchanged. Archiving appends completed tasks to that destination.
Archive writes done.txt first. If removing tasks from todo.txt fails, duplicates can
remain, but tasks are not lost. The error dialog explains partial archive failures.

The README documents build instructions, settings locations and current parity gaps.
