# Progress

## First release milestone (2026-10-09)

- Marked the native Windows milestone as the regular v0.1.0 release, rather than
  claiming complete parity. Release notes describe actual features, local
  validation, deferred requests and remaining Windows/Linux gaps.
- Packaged the native Windows x86-64 executable with README, help, license,
  upstream asset attribution and release notes, plus a SHA-256 checksum file.
  No personal tasks, settings, screenshots or local profiles are included.
- GitHub-hosted CI remains unverified. Local Windows validation is recorded
  separately; no successful Linux run is claimed.

## Archive destination picker and deferred shortcut (2026-10-09)

- Recorded Ctrl+D first in BACKLOG, with a conflict recheck, displayed default
  shortcut, and retained Ctrl+Shift+C alias. No shortcut behavior was changed.
- Native Windows Options and manual archiving now select existing or new archive
  destinations without an overwrite confirmation. The picker title explains
  that tasks will be appended. New File retains its overwrite confirmation.
  Selecting a destination does not write to it; archive append logic is unchanged.
  The portable frontend's separate file picker is unchanged.
- Expanded the real Windows GUI smoke check to select a nonempty fictional
  archive, verify no confirmation blocks selection, check the Options path,
  and compare exact file bytes afterward. This and the full native smoke pass.
- Formatting, all 23 tests, strict Clippy, and the optimized Windows build pass.
  Updated executable: `target/parity/release/todotxt-rs.exe`; the running normal
  release was left open. Linux runtime was not tested.

## Mounted-drive loading fix (2026-10-09)

- Reproduced error 1005 on the maintainer's mounted file without displaying its
  contents or writing to it: direct Rust reading succeeds, while canonicalize
  fails on the mount's final-path query.
- Made canonicalization optional for document loading and archive destinations.
  Use an absolute path when the filesystem cannot resolve the final path; keep
  canonical/symlink resolution when supported. Actual read errors still fail.
- Retained byte-based external-change checks and atomic saves. Windows self-archive
  protection also recognizes case and extended-prefix variants when final-path
  resolution is unavailable. Archive IO uses its resolved absolute destination.
- Added four regressions for unsupported-volume loading, unchanged bytes/BOM/
  newlines, successful local save and refused external changes, archive fallback,
  missing files, and Windows self-archive across path representations.
- The updated Document loader successfully opened the actual mounted source and
  rechecked it as unchanged. Its path and task contents are not committed.
- All 23 native/library regression tests, format check, strict Clippy, optimized
  release build and native GUI smoke passed. The smoke script now recognizes a
  running app owning Ctrl+Alt+M on both launches, dismissing only the expected
  shortcut-conflict dialog. No deferred merging, conflict resolution or watcher
  feature was implemented. The updated release is in `target/parity/release`
  because the normal release executable is running and locked.

## Tray right-click and global shortcut verification (2026-10-09)

- Verified the existing release's notification-area Exit menu against an isolated
  fictional todo file; the reported missing menu was not reproduced in that build.
- Updated the notification icon to Windows version-4 callbacks, decoded packed
  event messages while retaining legacy support, and kept its standard tooltip.
  Restore callback version after Explorer restarts. Release application state
  before the context menu's nested message loop so notifications and commands
  remain available while the menu is open.
- Expanded native smoke coverage to inspect the actual Exit item and select it
  while the owner is hidden and minimize-on-close is enabled. File Exit and tray
  Exit are verified independently. Both passed against the rebuilt release.
- With the original application closed, Windows SendInput Ctrl+Alt+M chords
  successfully hid and restored the isolated application. This verifies actual
  OS shortcut delivery rather than only posted WM_HOTKEY messages.
- Keyboard injection is opt-in (`-PhysicalHotkey`) for interactive local checks;
  ordinary CI smoke runs retain message-based checks.
- Built and smoke-tested `target/release/todotxt-rs.exe`. Test fixtures, profiles
  and captures stay ignored and contain fictional data. No personal file changed.

## Accepted printing and donation scope (2026-10-09)

- Maintainer accepts the current printing solution as finished. Closed embedded
  print preview and exact upstream paper layout; do not pursue those differences.
- Donations are finished by intentional omission. Do not add donation UI.
- Inspected upstream UpdateChecker and its menu handler: optional startup
  version checking plus a website link, with no update download or installation.
  Corrected the parity notes and README to describe that distinction. The Rust
  port does not currently check application versions.
- Recorded these decisions in AGENTS.md so future work respects them. No
  application behavior changed. Earlier entries below describe historical scope.

## Remaining Windows parity work (2026-10-09)

- Added native tray icon/context Exit, double-click show/hide, minimize-on-close,
  and optional Ctrl+Alt+M registration. Tray setup failures retain an accessible
  window; hotkey conflicts show an error while retaining tray use. Handle Explorer
  restarts by recreating the icon and restoring the window if recovery fails.
- Added the native Windows font chooser and persisted family, weight, italic,
  underline, strikeout, size and color. Native GDI task/group rendering uses the
  chosen font; completion/due/link colors retain upstream behavior.
- Replaced the tabbed filter dialog with a scrolling Active + nine-preset form.
  Added priority/project/context suggestions in native and portable filter fields.
  Portable task suggestions now also accept Space. The portable Options dialog
  supports font effects/color and the debug-logging preference.
- Added opt-in action logging without task, draft, filter or path contents;
  existing error logging remains available and may include paths in errors.
- Matched the seven-day title-bar calendar toggle in both frontends and the
  Windows task-list Right Shift binding.
- Added a native Windows printer chooser, GDI date/details table output and
  separate Ctrl+Shift+P preview command. Shared HTML output now reproduces the
  original date/details columns, group headers and colored metadata, safely
  escaping task text. Embedded preview and exact paper layout still differ.
- Windows raw/project/context sorting uses user-locale NLS comparison, matching
  the original .NET Framework culture behavior. Linux ordering remains deterministic.
- Regression coverage includes backward-compatible settings, font/tray setting
  round trips, log content boundaries, printable dates/groups/escaping, portable
  Space completion, and the expanded native Windows smoke checks.
- Native smoke passed the original file-safety/resize flows plus scrolling filter
  suggestions, font/printer cancellation, saved preferences, tray minimize,
  callback restore, close-to-tray, hotkey-message restore, title-bar calendar and
  forced File Exit. Physical printing and physical hotkey delivery are not claimed.
- Final validation passed: format check; 19 task/file/settings/printing regression
  tests in an isolated Cargo target directory; seven portable GUI event tests;
  strict Clippy for native and optional portable builds; optimized native release
  build and expanded release smoke checks. The release is available at
  `target/parity/release/todotxt-rs.exe`; the ordinary output was locked by a
  running application, which was left in place.
- GitHub-hosted CI did not start; Linux build/runtime validation is still pending.
  No deferred backlog feature was implemented. Full parity is not claimed.

## Fresh repository preparation (2026-10-08)

- The owner deleted the previous GitHub repository after its old commit URLs
  continued to expose the removed sample. Prepared one parentless initial commit
  containing the cleaned project and fictional fixture.
- Reviewed all tracked files for known original task lines, likely credential
  strings, local user paths, screenshots and build/local artifacts. No matches
  were found. The icon matches the public upstream source.
- Use the owner's GitHub no-reply address for the new commit. Ignore the whole
  samples and screenshots directories, browser automation output, environment
  files and common private-key file types to reduce accidental publication.

## Personal sample removal (2026-10-08)

- Replaced the personal sample with a fictional demo fixture. Updated both
  frontends and the fixture regression test; no real tasks are bundled now.
- Removed personal screenshots and blocked the old sample path and screenshot
  PNG files in Git ignore rules. Working rules prohibit publishing real tasks.
- Purged the personal sample and screenshots from every published commit and
  force-pushed the cleaned master branch with an explicit lease. Removed local
  tool snapshot references and pruned the original sensitive Git objects.
- Verified all rewritten commits for removed paths, original task text, and
  private screenshot blobs. GitHub had no forks, PRs, releases, or uploaded build
  artifacts requiring separate cleanup. Local cleanup notes contain no task data.
- Fictional fixture validation passed: 22 tests, strict Clippy, release build,
  and native Windows smoke flows including reflow and external-change safety.
- Old clones must be replaced or cleaned; merging old history could reintroduce
  the removed data. GitHub caches are outside this rewrite and were not pursued.

## Resize regression and parity assessment (2026-10-08)

- Reproduced the reported native resize failure. Default Win32 processing
  synchronously sends WM_SIZE, but our handler retained the mutable application
  borrow and skipped that nested notification. Released state before calling
  DefWindowProc so width/height changes and maximizing relayout controls.
- Skip zero-size minimized layouts and compare the actual list client width
  before reflowing task text. Height-only changes retain the task viewport and
  update status layout. Wrapping remains controlled by its existing preference.
- Extended the native smoke check with width changes, measured wrapped-row
  heights, height changes, maximize, taskbar minimize, restore to the preceding
  maximized state, and restore to normal. It passes against the rebuilt release,
  alongside task/editor/dialog and external-change safety checks.
- Formatting, 16 default tests, strict Clippy, and release build passed.
- Added a parity status table. Ordinary taskbar minimization is verified;
  tray/minimize-on-close/global hotkey remain missing. Full parity and Linux
  runtime validation are still outstanding. No deferred feature was started.

## Deferred requests (2026-10-08)

- Recorded CLI operations, recurring tasks, OR filters, safe external-change
  reconciliation/history/conflict dialogs, prompt reload with cloud/rclone
  fallbacks, and an Android-first mobile companion in `BACKLOG.md`.
- Added the parity-first restriction to working rules and clarified that
  current filters only support AND. Documentation only; none implemented.

## Native Windows frontend (2026-10-08)

- Windows now defaults to a Win32 frontend. Real OS menus, EDIT, owner-drawn
  LISTBOX, native selection/scrollbars, status bar, file pickers, and separate
  owned dialogs replace egui controls. Styled task text uses GDI/Segoe UI.
- Added themed Common Controls v6, the upstream ICO in the executable, and
  per-monitor DPI awareness. Native window geometry uses `native-window.json`.
- Kept the Linux egui frontend and optional Windows `portable-ui` fallback.
  Windows default dependency tree excludes egui/eframe/rfd/arboard/glutin.
- Shared preference schema and existing task/file library preserve behavior.
  Added controller regressions for retained drafts after external changes,
  safe reload/retry, physical duplicate identity, and shared preferences.
- Fixed Win32 reentry during window destruction and routed queued keyboard
  events by their target control rather than later focus state.
- Passed 16 default tests and 22 tests with the portable frontend enabled;
  strict Clippy passed for both configurations. Formatting and release build
  passed on Windows. Linux runtime remains unverified.
- Release smoke check passed against real controls: Enter/save, completion,
  Enter in an owned task dialog, native Options, external-change error dialog,
  unchanged external bytes, retained draft, screenshot, and clean shutdown.
  CI now includes that native Windows smoke check.
- Release executable: `target/release/todotxt-rs.exe`, 2,838,016 bytes.
  Updated README/help/parity rules and `docs/screenshots/rust.png`.
- Upstream remains unmodified.

## Initial portable frontend

- Created project working rules and parity specification.
- Cloned upstream dev branch into reference/todotxt.net.
- Found the running Windows original (todotxt.exe).
- Inspected task parsing, file IO, menu/editor/list XAML, sorting and filters.
- Built the Rust Cargo package, standalone task/file/view library, and native
  egui desktop application. Windows Segoe UI regular/bold fonts and the upstream
  icon are used; Linux has system/embedded font fallbacks.
- Implemented task CRUD, multiselection/clipboard, completion, priority/date
  changes, archive, eight sorts, multi-tag grouping, filters/presets, editor
  suggestions, preferences, auto refresh, status counts, help and printable HTML.
- Added user-facing README and embedded help covering actual features, safe
  default demo copying, external-change refusal/recovery, archive partial
  failures, file preservation, settings locations, and known fidelity gaps.
- Added a Windows native preview/capture script and Windows/Linux CI checks.
- 19 tests passed on Windows (13 library/file/parity regressions and 6 GUI-event
  tests). GUI tests cover keyboard task flows, dialog focus, date entry,
  Ctrl-Enter, suggestions, preserved drafts and reload safety, and status paint.
- Final validation passed: `cargo fmt --check`, `cargo test --locked` (19 tests),
  `cargo clippy --locked --all-targets -- -D warnings`, and
  `cargo build --locked --release`.
- A GUI regression caught Enter surrendering editor focus in Ctrl-Enter mode;
  fixed it and verified plain Enter keeps the draft editable and Ctrl-Enter saves.
- Release executable built at `target/release/todotxt-rs.exe` (8,497,664 bytes).
  The isolated native preview script passed against this release build and
  captured `docs/screenshots/rust.png`; the preview closed cleanly afterward.
- Native window launch and visual capture verified. Native keyboard automation
  was unreliable in this host session; GUI-event tests are the keyboard evidence.
- No Linux distribution is available in the host's WSL installation. Linux CI
  configuration is present but has not been run or claimed as passed.
- Upstream checkout remains clean. The initial personal sample was later removed
  and replaced by fictional data; see the privacy cleanup entry above.

The first working port is ready for manual Windows/Linux use. Remaining parity
work is tracked in `PARITY.md` and the root README, particularly tray/global
hotkey behavior and exact widget/wrapping metrics. No full-parity or Linux-runtime
claim is made.
