# Working rules for todotxt.rs

Read this file and `docs/PARITY.md` before making changes. Keep `docs/PROGRESS.md`
current when completing a substantial change.
Read `docs/BACKLOG.md` for deferred requests. Do not implement those ideas before
parity with todotxt.net is achieved. Their recording is not authorization to
begin implementation automatically; keep parity work as the current scope.

`docs/BACKLOG.md` is the authoritative project checklist, including current
parity/validation work and deferred ideas. Keep its current-work items aligned
with `docs/PARITY.md`. After changing or completing backlog items, regenerate
root `todo.txt` with `python scripts/sync-backlog.py`, then verify with
`python scripts/sync-backlog.py --check`. Do not edit the generated task list
independently; synchronization is one way from Markdown to todo.txt. Keep both
files in the same commit. The generated tasks do not authorize deferred work.
Each pending checkbox must have an explicit `(A)`-`(Z)` priority followed by
one concise, bold, actionable title on its first line.
Split independently finishable work into separate checkboxes. Keep detailed
requirements in continuation notes; only the titles belong in todo.txt.

The goal is a faithful Windows/Linux Rust port of todotxt.net, not a redesigned
task manager. Use the checked-out C# and XAML in `reference/todotxt.net` as the
behavioral specification. Keep the reference checkout unmodified and retain
its BSD notice when translating code or reusing assets.

Keep the task library independent of the desktop toolkit. Preserve UTF-8,
line endings, task ordering, and placement of tags. Never test by modifying the
user's live todo file; use temporary copies and the fictional fixture in
`fixtures/demo.todo`. Never commit real tasks, personal screenshots, private
demo copies, or binaries containing private task data. The old personal sample
and screenshots have been purged; do not restore them from old clones/history.
Detect external changes before writes. File failures must be visible to users.

Use plain menus, a one-line task editor, raw-text task rows, and the upstream
shortcuts. Document any fidelity gaps honestly in `docs/PARITY.md`.
Windows defaults to the Win32 frontend in `src/windows`; keep native controls
and platform-specific dependencies behind that boundary. Preserve the portable
Linux frontend and optional Windows `portable-ui` fallback. Preference changes
must remain compatible with both frontends through `src/settings.rs`.

Validate substantive task/file changes with regression tests. Before finishing,
run `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`.
Do not claim Linux runtime validation from Windows-only checks.

## Accepted parity decisions (2026-10-09)

The maintainer accepts the current native Windows printing and browser HTML
preview as finished. Do not implement the original embedded print preview or
chase its exact paper layout as parity work. Donations are also closed: do not
add a donation menu or donation infrastructure. These are accepted differences,
not remaining gaps. The upstream updater is only an optional version check and
website link, not an automatic installer; describe it accurately.

## Workspace housekeeping

Use Cargo's standard target directory for builds. Avoid parallel target trees,
versioned executable copies, release archives, tool downloads and historical
preview profiles in the workspace unless specifically needed for the task.
Use system temporary directories for scratch work and isolated GUI profiles;
remove successful test artifacts. The smoke script retains failures for diagnosis
and supports -KeepArtifacts when a screenshot is explicitly needed.
Keep upstream reference source, Git metadata, project sources and documents.
Inspect ignored files before cleanup; never remove live task files or real-app
recovery storage. Check resolved paths and refuse traversal through reparse points
before recursive deletion. A housekeeping pass may retain the current executable
at target/release/todotxt-rs.exe while removing rebuildable caches and older copies.

Do not launch GUI smoke tests for housekeeping or documentation-only changes.
When a task needs interactive UI validation, explain which windows/dialogs will
open before starting. Never launch or interrupt the user's installed app for
such checks; use only the isolated fictional test profile.
