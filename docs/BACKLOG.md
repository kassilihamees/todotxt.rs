# Deferred ideas

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

All entries are deferred. No application behavior changes with this document.
