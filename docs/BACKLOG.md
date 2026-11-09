# Project backlog

This is the source checklist for the generated [project todo.txt](../todo.txt).
Edit this file, then run `python scripts/sync-backlog.py` from the repository root.
`python scripts/sync-backlog.py --check` checks synchronization without writing;
CI runs that check too. Only the concise bold titles of unchecked items are
exported, one action per line, ordered by priority. Detailed requirements stay
in this document.
The `@parity` and `@deferred` contexts distinguish current work from future ideas.
Direct edits/completions in todo.txt do not flow back here and are overwritten
on regeneration; complete or edit the corresponding Markdown checkbox instead.
The generated file contains public project work only.

Priorities: `(A)` file-safety and reliability validation; `(B)` remaining parity
work; `(C)` practical future improvements and optional features; `(D)` larger or
exploratory ideas. Priorities do not override the current/deferred boundary.


## Current work: parity and validation

These are the remaining items described in [PARITY.md](PARITY.md). Each checkbox
has one independently finishable action; keep its bold title short. Put detailed
requirements in the notes below it.

- [x] (B) **Add Shift+9 task creation to the Windows UI.**
  Start a task with a priority prefix and show priority suggestions, matching
  the original shortcut.

- [x] (B) **Add Shift+9 task creation to the portable UI.**
  Match the same priority-prefix and suggestion behavior in the portable
  frontend.

- [x] (B) **Fix clipped Windows task suggestions.**
  Clip sibling painting and size the dropdown from real row metrics, including
  its border. Show all initial project/context/priority rows without navigation.

- [x] (B) **Use a calendar picker in Windows date dialogs.**
  Restore calendar selection and T/Up/Down shortcuts for due and threshold dates.

- [ ] (B) **Add calendar selection to portable date dialogs.**
  The original uses a date picker. The Windows frontend now has a native one;
  portable date dialogs still use text input and need calendar selection.

- [ ] (B) **Add font-family selection to the portable UI.**
  Preserve compatibility with the shared font preferences and native frontend.

- [ ] (B) **Add font-weight selection to the portable UI.**
  Expose the existing shared weight preference and apply it consistently.

- [ ] (B) **Add Linux system-tray support.**
  Support minimize/restore, close-to-tray and Exit; validate the desktop
  environments supported.

- [ ] (B) **Add Linux global-shortcut support.**
  Integrate window hide/restore, report shortcut ownership conflicts, and
  account for desktop support.

- [ ] (A) **Test the app on real Linux desktops.**
  Exercise task editing, dialogs, resizing, shortcuts and file safety. Hosted
  builds and headless GUI tests do not establish desktop runtime parity.

- [ ] (B) **Compare the Windows UI with todotxt.net.**
  Check wrapping, spacing, selection and dialog behavior against the running
  original and captured screenshots.

- [ ] (B) **Compare the Linux UI with todotxt.net.**
  Review the portable frontend for visual and interaction differences. Respect
  the accepted printing and donation differences; do not reopen embedded
  print-preview work.

- [ ] (C) **Add the optional new-version notification.**
  Check the advertised version and offer a website link. This is not an
  automatic update installer.

- [ ] (B) **Match Linux task sorting to the user's locale.**
  Review deterministic ordering against the original's culture-sensitive
  comparisons.

- [ ] (A) **Test direct saves and recovery on the affected rclone mount.**
  The maintainer considers the file issue resolved for now (2026-11-10).
  Keep this as additional validation, not an active investigation. Use fictional
  fixtures to verify saved contents and local previous/intended recovery copies.

- [ ] (A) **Test external-change safeguards on the affected rclone mount.**
  Additional validation after the maintainer reported the issue resolved for
  now: verify refusal of conflicting writes and safe idle refresh without
  repeatedly postponing uploads.

- [ ] (A) **Verify task-file contents after rclone uploads.**
  Check eventual remote contents as well as the mounted view; successful local
  read-back alone does not establish cloud durability.

- [ ] (B) **Test tray recovery after restarting Explorer.**
  Confirm the notification icon and its restore/Exit actions recover
  correctly.

## Deferred ideas

**Do not implement these before achieving parity with todotxt.net.** Recording
an idea does not authorize its implementation automatically after parity.

- [ ] (C) **Make Ctrl+D duplicate a task.**
  Check shortcut conflicts first. Display Ctrl+D for Copy Task to New Task and
  retain Ctrl+Shift+C as an alias. Neither frontend currently binds Ctrl+D;
  check again when implementing. Keep this as the first deferred idea.

- [ ] (C) **Design the task-management command-line interface.**
  Choose syntax for task operations and any further commands beyond
  add/list/complete. Reuse shared task semantics and file protections.

- [ ] (C) **Add a command to list tasks.**
  Follow the agreed CLI design and existing task/filter behavior.

- [ ] (C) **Add a command to create a task.**
  Use the shared task and safe-save logic.

- [ ] (C) **Add a command to complete a task.**
  Use the shared completion semantics and file protections.

- [ ] (C) **Define recurring-task rules and metadata.**
  Decide compatible recurrence syntax, due/threshold date handling and
  recurrence behavior; preserve the plain todo.txt format.

- [ ] (C) **Create the next recurring task when completing an occurrence.**
  Implement the agreed recurrence rules using shared task logic.

- [ ] (D) **Raise dated-task priorities as deadlines approach.**
  Tasks with a threshold date initially have no priority. Increase priority as
  the deadline approaches; overdue tasks are always priority A.

- [ ] (C) **Add a quick text filter to the Filter menu.**
  Add a separate Filter... input instead of opening preset definitions. Keep
  the entered text active until changed, cleared or replaced by another
  filter/preset. Decide restart persistence when implementing.

- [ ] (C) **Design syntax for combining AND and OR filters.**
  For example, support the intended query (project A OR project B) AND
  incomplete, including exclusions. Choose clear syntax.

- [ ] (C) **Implement OR matching in task filters.**
  Allow either of two projects to match. Current nonempty filter lines are
  ANDed and ordinary conditions are literal substrings; two project lines
  require both projects. OR, | and regex alternatives do not currently express
  a union.

- [ ] (C) **Document the combined filter syntax.**
  Explain AND, OR, exclusions and presets with practical examples.

- [ ] (C) **Retain the versions needed to merge external edits.**
  Build on existing recovery copies. Preserve the last loaded state, current
  disk state and local changes/drafts. Consider diffs, lightweight history or
  optional Git support without requiring Git to be installed.

- [ ] (C) **Merge unambiguous external and local task changes.**
  Combine independent additions and edits, including tasks added in this
  instance. Account for completion, deletion, ordering and duplicate lines; do
  not identify tasks solely by stale line numbers or identical text.

- [ ] (C) **Add a dialog for resolving conflicting task edits.**
  Show both versions and explicit choices when there is no clear automatic
  solution.

- [ ] (C) **Recheck the file before saving a merged task list.**
  Preserve recoverable versions and detect changes arriving during
  reconciliation. Current behavior remains write refusal, retained drafts and
  explicit reload; automatic merging is not implemented.

- [ ] (C) **Watch local task files for changes.**
  Aim for nearly instant reload when safe while preserving active drafts and
  edits.

- [ ] (C) **Add reliable change detection for cloud-mounted task files.**
  Use polling/rechecks where notifications are unavailable or delayed. Account
  for cloud storage and rclone mounts without continually postponing uploads.
  Reconciliation and save-time checks must protect data when events are late
  or missed.

- [ ] (D) **Build an Android companion app.**
  Complement Windows and Linux using compatible todo.txt behavior and shared
  Rust logic where practical. Consider mobile file access, cloud/offline use
  and conflict protection. An iOS companion is conditional on finding a
  maintainer.

- [ ] (C) **Move detailed usage and recovery help into focused guides.**
  Preserve the useful information currently in the README and reduce
  duplication.

- [ ] (C) **Move build and development instructions into focused guides.**
  Keep detailed contributor material available through clear links.

- [ ] (C) **Shorten the README and link the detailed guides.**
  Keep a clear introduction, download/run instructions and essential current
  limitations on the GitHub front page. Record now; do not reorganize yet.

- [ ] (D) **Evaluate whether supporting multiple todo.txt files is worthwhile.**
  First assess real use cases against added complexity in a deliberately
  simple app. Compare easy file switching with simultaneous lists/combined
  views. Consider per-file filters/preferences, archives, recovery and
  external-change handling. Evaluate usefulness before deciding scope;
  implementation is not approved.

All entries in this deferred section remain deferred. No application behavior
changes with this document.
