# Project backlog

This is the source checklist for the generated [project todo.txt](../todo.txt).
Edit this file, then run `python scripts/sync-backlog.py` from the repository root.
`python scripts/sync-backlog.py --check` checks synchronization without writing;
CI runs that check too. Only unchecked items are exported, one task per line.
The `@parity` and `@deferred` contexts distinguish current work from future ideas.
Direct edits/completions in todo.txt do not flow back here and are overwritten
on regeneration; complete or edit the corresponding Markdown checkbox instead.
The generated file contains public project work only.

## Current work: parity and validation

These are the remaining items described in [PARITY.md](PARITY.md). Keep this
checklist current when resolving or discovering parity gaps; PARITY.md retains
the detailed behavior specification and accepted differences.

- [ ] **Implement Shift+9 priority-prefilled task creation.** Match the original
  shortcut and priority suggestions in the native and portable frontends.

- [ ] **Finish portable font selection.** Add font-family and font-weight
  selection to the portable frontend, preserving the shared preference format.

- [ ] **Implement Linux tray and global shortcut integration.** Validate
  minimize/restore, close-to-tray, Exit, shortcut ownership, and desktop support.

- [ ] **Validate the Linux desktop application.** Run it on real Linux desktops
  and compare task editing, dialogs, resizing, shortcuts, and file safety. Hosted
  builds and headless GUI tests do not establish desktop runtime parity.

- [ ] **Compare visual and interaction parity on both platforms.** Review native
  Windows controls and portable Linux behavior against the original, including
  wrapping, spacing, selection and dialogs. Respect the accepted printing and
  donation differences; do not reopen embedded print-preview work.

- [ ] **Implement the optional upstream version notification.** Check advertised
  versions and provide a website link; this is not an automatic update installer.

- [ ] **Match locale-aware ordering on Linux.** Review the current deterministic
  ordering against the original's culture-sensitive comparison.

- [ ] **Validate mounted-file saving on the affected setup.** Use fictional
  fixtures to verify direct writing, recovery, external-change refusal and idle
  refresh with rclone, including eventual remote uploads. Preserve the unresolved
  status until affected-machine results confirm the fix.

- [ ] **Validate Windows tray recovery after Explorer restarts.** Confirm that
  the notification icon and restore/Exit actions recover correctly.

## Deferred ideas

Recorded from the maintainer's request on 2026-10-08.

**Do not implement these before achieving parity with todotxt.net.** The current
priority is the original application's appearance and behavior, with gaps
tracked in [PARITY.md](PARITY.md). These are future ideas, not implemented
features or authorization to start them automatically after parity.

- [ ] **Ctrl+D for Copy Task to New Task.** Check for shortcut conflicts before
  implementation. Make Ctrl+D the default shortcut displayed for this command,
  while keeping Ctrl+Shift+C usable as an alias. Neither frontend currently
  binds Ctrl+D; check again when implementing. Deferred; do not implement now.

- [ ] **Command-line task operations.** Make the application usable without a
  GUI through simple commands to add, list, complete, and otherwise manage
  tasks. Reuse task semantics and file protections across CLI and desktop.
  Command syntax remains to be designed.

- [ ] **Recurring tasks.** Recognize recurring-task metadata and handle the next
  occurrence when completing a task. Decide and document compatible recurrence
  syntax, due/threshold date handling, and recurrence behavior before building
  it; preserve the plain todo.txt format.

- [ ] **Promote dated tasks** Create a subsystem, where the tasks with threshold
  date have no-priority, but that will grow over time with approaching deadline
  towards A. The tasks that are overdue are always A.

- [ ] **Quick ad hoc filter.** Add a separate Filter... command to the Filter
  menu, opening a simple text input instead of the preset-definition dialog.
  Apply the entered text as the active filter and retain it until it is changed,
  cleared, or replaced by another filter/preset. Keep it separate from editing
  saved preset definitions; decide restart persistence when implementing.

- [ ] **OR filters and clear documentation.** Allow one filter to include tasks
  from either of two projects, alongside existing AND and exclusion conditions.
  For example, the intended query is `(project A OR project B) AND incomplete`;
  this is a requirement example, not supported syntax.
  Current implementation only ANDs nonempty filter lines and treats ordinary
  conditions as literal substrings. Two project lines require both projects;
  `OR`, `|`, and regex alternatives do not currently express a union. Choose
  clear syntax and explain combinations with AND, exclusions, and presets.

- [ ] **Reconcile externally changed files without losing local work.** Improve
  protections on reload and save using the last loaded state, current on-disk
  state, and local changes/drafts. Consider diffs, lightweight version history,
  or Git-backed recovery as design options, rather than assuming Git must be
  installed. Preserve recoverable versions before reconciliation. Combine
  unambiguous additions/edits automatically, including tasks added in this
  instance while the file changed elsewhere. Account for edits, completion,
  deletion, ordering, and duplicate task lines; avoid identifying tasks solely
  by stale line numbers or identical text. When changes conflict without a
  clear solution, show a simple resolution dialog with both versions and
  explicit choices. Recheck disk state before committing a merged result.
  Current behavior remains write refusal plus retained drafts and explicit
  reload; automatic merging and conflict resolution are not implemented.

- [ ] **Prompt disk-change detection with a reliable fallback.** Aim for nearly
  instant reload when safe, while preserving active drafts and edits. Consider
  filesystem notifications with polling/rechecks where needed. Files may live
  in cloud storage or rclone-mounted directories, where timely notifications
  and fresh reads cannot be assumed. Reconciliation and write-time checks must
  still protect data when changes arrive late or events are missed.

- [ ] **Mobile companion, Android first.** Complement Windows and Linux with an
  Android app using compatible todo.txt behavior and shared Rust logic where
  practical. Consider mobile file access, cloud/offline use, and the same
  conflict protections. An iOS companion is conditional on finding a maintainer.

- [ ] **Reorganize the README for readability.** Keep the GitHub front page
  short and easy to scan, with a clear introduction, download/run instructions,
  and essential current limitations. Move detailed usage, build, recovery and
  development material into focused documents and link them clearly. Preserve
  useful information while reducing duplication. Record now; do not reorganize yet.

- [ ] **Evaluate support for multiple todo.txt files.** First check whether this
  is actually a good idea for a deliberately simple application and which real
  use cases justify it. Compare easy file switching with simultaneous lists or
  combined views; consider per-file filters/preferences, archive destinations,
  recovery and external-change handling. Decide the scope only after evaluating
  the added complexity. This is an idea to assess, not an approved implementation.

All entries in this deferred section remain deferred. No application behavior
changes with this document.
