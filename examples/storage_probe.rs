//! Storage compatibility probe. Never reads or modifies an existing task file.
use std::{
    fs,
    io::{self, Write},
    path::Path,
    thread,
    time::Duration,
};

fn direct_write(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(destination)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn regular_rename(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    let staged = destination.with_extension("staged");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if fs::read(&staged)? != bytes {
        return Err(io::Error::other("Regular staging read-back mismatch"));
    }
    fs::rename(staged, destination)
}

#[cfg(windows)]
fn windows_replace(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *const (),
            reserved: *const (),
        ) -> i32;
    }
    let staged = destination.with_extension("replace-staged");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if fs::read(&staged)? != bytes {
        return Err(io::Error::other("ReplaceFile staging read-back mismatch"));
    }
    let target: Vec<u16> = destination.as_os_str().encode_wide().chain([0]).collect();
    let staged: Vec<u16> = staged.as_os_str().encode_wide().chain([0]).collect();
    // Used only on new fictional files owned by this probe.
    let accepted = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            staged.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if accepted == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let parent = std::env::args_os().nth(1).ok_or_else(|| io::Error::other("Usage: storage_probe.exe DIRECTORY (for example H:\\todo). DIRECTORY must exist; a new fictional test folder is created inside it."))?;
    let parent = Path::new(&parent);
    if !parent.is_dir() {
        return Err(io::Error::other(
            "Supply an existing directory, not a todo file.",
        ));
    }
    // Keep failed artifacts for inspection. Never delete or reuse a user file.
    let folder = tempfile::Builder::new()
        .prefix("todotxt-rs-fictional-probe-")
        .tempdir_in(parent)?
        .keep();
    let original = format!(
        "\u{feff}{}",
        (0..256)
            .map(|n| format!("Fictional diagnostic task {n} +probe @test café\r\n"))
            .collect::<String>()
    )
    .into_bytes();
    let intended = String::from_utf8(original.clone())
        .unwrap()
        .replacen(
            "Fictional diagnostic task 0",
            "x 2026-10-09 Fictional diagnostic task 0",
            1,
        )
        .into_bytes();
    type Writer = fn(&Path, &[u8]) -> io::Result<()>;
    let methods: Vec<(&str, Writer)> = vec![
        ("direct-write", direct_write),
        ("current-app-save", todotxt_rs::document::atomic_write),
        ("regular-file-rename", regular_rename),
        #[cfg(windows)]
        ("windows-ReplaceFile", windows_replace),
    ];
    let mut report = format!(
        "todotxt.rs storage probe version {}\nAll files are newly created fictional fixtures. No existing file was read or changed.\n",
        env!("CARGO_PKG_VERSION")
    );
    println!("{report}");
    println!("Test folder: {}", folder.display());
    for (name, write) in methods {
        let destination = folder.join(format!("{name}.txt"));
        let mut initial = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        initial.write_all(&original)?;
        initial.sync_all()?;
        drop(initial);
        for round in 1..=2 {
            let expected = if round == 1 { &intended } else { &original };
            let outcome = match write(&destination, expected) {
                Ok(()) => "returned success".to_owned(),
                Err(error) => format!(
                    "error kind={:?} os_code={:?}",
                    error.kind(),
                    error.raw_os_error()
                ),
            };
            let line = format!(
                "{name} round={round}: {outcome}; expected_bytes={}\n",
                expected.len()
            );
            print!("{line}");
            report.push_str(&line);
            for delay in [0, 1000, 4000] {
                thread::sleep(Duration::from_millis(delay));
                let line = match fs::read(&destination) {
                    Ok(actual) => format!(
                        "  delay_ms={delay} actual_bytes={} matches={}\n",
                        actual.len(),
                        actual == *expected
                    ),
                    Err(error) => format!(
                        "  delay_ms={delay} read_error={:?} os_code={:?}\n",
                        error.kind(),
                        error.raw_os_error()
                    ),
                };
                print!("{line}");
                report.push_str(&line);
            }
        }
    }
    // Also keep the report on ordinary local storage, independent of the test mount.
    let report_directory = directories::ProjectDirs::from("", "", "todotxt.rs")
        .map(|dirs| dirs.data_local_dir().join("diagnostics"))
        .unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&report_directory)?;
    let report_file = tempfile::Builder::new()
        .prefix("todotxt-rs-storage-report-")
        .suffix(".txt")
        .tempfile_in(report_directory)?;
    let (mut file, report_path) = report_file.into_parts();
    file.write_all(report.as_bytes())?;
    file.sync_all()?;
    drop(file);
    let report_path = report_path.keep().map_err(|error| error.error)?;
    println!("Local report: {}", report_path.display());
    println!(
        "Only byte counts, comparisons and error codes are reported. Test files were retained."
    );
    Ok(())
}
