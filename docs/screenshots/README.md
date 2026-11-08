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

The preview creates fictional demo tasks in a unique system-temporary directory,
captures a screenshot there, and checks
native controls, resize/reflow, maximize/minimize/restore, editor and dialog
flows, and external-change refusal with retained drafts. It changes only its
private fixture copy. Successful runs remove their profile and screenshot.
Failed runs retain their temporary directory for diagnosis and print its path.
Use `-KeepArtifacts` to retain a successful run for visual inspection:

```powershell
./scripts/smoke-windows.ps1 -Binary target/release/todotxt-rs.exe -KeepArtifacts
```

Direct captures with `capture-window.ps1` also default to the system temporary
directory unless an output path is specified. Generated screenshots stay local
and are not committed. Recovery safeguards still use the app's usual local data
directory; housekeeping never deletes that real-app recovery storage.
