# Safe visual checks

Historical screenshots were removed because they showed personal task data.
Screenshot PNG files are ignored by Git to prevent accidental publication.
Use only the fictional fixture in `fixtures/demo.todo` for future captures.
Never capture the running original application while it displays real tasks.

Build and check a separate Rust preview using a fresh settings directory:

```powershell
cargo build --locked --release
./scripts/smoke-windows.ps1 -Binary target/release/todotxt-rs.exe
```

The preview creates fictional demo tasks, captures a local screenshot, checks
native controls, resize/reflow, maximize/minimize/restore, editor and dialog
flows, and external-change refusal with retained drafts. It changes only its
private fixture copy. Generated screenshots stay local and are not committed.
