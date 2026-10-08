# Working rules for todotxt.rs

Read this file and `docs/PARITY.md` before making changes. Keep `docs/PROGRESS.md`
current when completing a substantial change.
Read `docs/BACKLOG.md` for deferred requests. Do not implement those ideas before
parity with todotxt.net is achieved. Their recording is not authorization to
begin implementation automatically; keep parity work as the current scope.

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
