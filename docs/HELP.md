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
Tab or Enter accepts a suggestion (Space also accepts on Windows).
Press Enter again to save the task. Windows uses native edit controls for
selection, clipboard commands, and Ctrl+Z undo; Alt activates the native menus.

Filters and files

F opens filter definitions. Each line is an AND condition; prefix - to exclude.
DONE/-DONE select complete/incomplete. due:today, due:past, due:future and
due:active filter due dates. 1–9 apply saved presets; 0 clears the text filter.
Ctrl+T hides future threshold tasks. Ctrl+H shows h:1 hidden tasks.
Ctrl+0–7 selects file/alphabetical/completed/context/due/created/priority/project sort.
O or Ctrl+O opens a file. C or Ctrl+N creates a file. . or F5 reloads it.
F10 opens Options. ? opens this help. Ctrl+P opens printable HTML in your browser.

File safety

Edits save immediately. The file must be UTF-8. Newlines, a UTF-8 BOM, blank lines,
untouched whitespace, and physical task order are preserved.
If another program changes the source file, a write is refused and editor text is kept.
Copy your draft if needed, reload with F5, reselect the task, then reapply your edit.
Reload cancels the edit target so an old line number cannot overwrite a different task.
Options can enable automatic refresh when the editor is empty.
Archive writes done.txt first. If removing tasks from todo.txt fails, duplicates can
remain, but tasks are not lost. The error dialog explains partial archive failures.

The README documents build instructions, settings locations and current parity gaps.
