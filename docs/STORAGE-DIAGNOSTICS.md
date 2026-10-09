# Mounted-storage diagnostic

The affected computer has now reported a save verification failure with
0.1.2-dev. This rules out an older build as the explanation for that particular
attempt. The staged file passed its read-back check; destination read-back
after replacement did not match. That narrows the investigation to replacement
or subsequent filesystem visibility, but does not prove a specific driver bug.

Do not test by repeatedly editing a real task file. Keep the original backup
and recovery copies. `previous.txt` contains the file before the failed save;
`intended.txt` contains the requested completion/edit. Check these privately.

## Run on the affected computer

Download/extract the diagnostic ZIP onto ordinary local storage (for example,
Downloads on C:), open PowerShell in its directory on the affected computer,
and run:

```powershell
.\storage_probe.exe 'H:\todo'
```

Supply an existing directory on the problematic mount. The probe rejects a
file argument and creates a new, uniquely named `todotxt-rs-fictional-probe-*`
subdirectory. It reads/writes only its new fictional fixtures, never an existing
task file. Direct-write testing truncates only its own fictional test file.
The test takes about 40 seconds plus filesystem latency.

It compares four methods on Windows:

1. Direct overwrite, similar to the upstream StreamWriter approach.
2. The Rust application's current staged, closed, verified tempfile replacement.
3. A normal staged file followed by a rename, without tempfile persistence's
   temporary-file attribute handling.
4. Windows ReplaceFile with a normal staged file.

Each method toggles between two fictional states and checks immediate and
delayed reads. Output contains only expected/actual byte counts, comparisons,
and error kinds/codes. The report excludes task contents and existing filenames.
A separate report is retained in the user's local data directory. Fictional
test files are retained; they may be removed manually after investigation.

Send the report, plus the affected machine's rclone version and mount options,
especially VFS cache mode, write-back delay, cache directory and network mode.
Omit credentials and remote account names. Do not paste task or recovery contents.

Matching bytes through the mounted view do not prove eventual cloud upload.
The fixtures are freshly created; cached new files may behave differently from
an existing remote file, so even passing results do not exonerate the mount.
Probe results must be assessed on the affected computer; passing results on
the development machine cannot establish compatibility there. The application
does not automatically switch to direct overwrite after a failed replacement.

## Build from source

```powershell
cargo build --locked --release --example storage_probe
.\target\release\examples\storage_probe.exe 'H:\todo'
```

The probe works on ordinary local directories as a control. Windows-specific
ReplaceFile is omitted on other platforms. Linux runtime has not been validated.
