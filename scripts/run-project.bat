@echo off
setlocal EnableExtensions DisableDelayedExpansion
rem Works from scripts or when copied beside target\release\todotxt-rs.exe.
if exist "%~dp0todotxt-rs.exe" (
    for %%I in ("%~dp0..\..") do set "PROJECT_ROOT=%%~fI"
) else (
    for %%I in ("%~dp0..") do set "PROJECT_ROOT=%%~fI"
)
set "PROJECT_EXE=%PROJECT_ROOT%\target\release\todotxt-rs.exe"
set "PROJECT_TODO=%PROJECT_ROOT%\todo.txt"
set "PROFILE_BASE=%LOCALAPPDATA%"
if not defined PROFILE_BASE set "PROFILE_BASE=%APPDATA%"
if not defined PROFILE_BASE (
    echo Cannot find the Windows application-data directory.
    exit /b 1
)
set "PROJECT_CONFIG=%PROFILE_BASE%\todotxt.rs\profiles\project-todotxt-rs"
if not exist "%PROJECT_ROOT%\Cargo.toml" (
    echo Keep this launcher in scripts or beside target\release\todotxt-rs.exe.
    exit /b 1
)
if not exist "%PROJECT_EXE%" (
    echo Missing executable: "%PROJECT_EXE%"
    echo Build it with cargo build --locked --release.
    exit /b 1
)
if not exist "%PROJECT_TODO%" (
    echo Missing project task list: "%PROJECT_TODO%"
    exit /b 1
)
if /i "%~1"=="--dry-run" (
    echo Executable: "%PROJECT_EXE%"
    echo Task file: "%PROJECT_TODO%"
    echo Separate config: "%PROJECT_CONFIG%"
    exit /b 0
)
if not exist "%PROJECT_CONFIG%\settings.json" (
    if not exist "%PROJECT_CONFIG%" mkdir "%PROJECT_CONFIG%"
    if not exist "%PROJECT_CONFIG%" exit /b 1
    >"%PROJECT_CONFIG%\settings.json" echo {"sort":"Priority","word_wrap":true,"auto_refresh":true,"minimize_to_tray":false,"minimize_on_close":false,"auto_archive":false}
    if errorlevel 1 exit /b 1
)
rem todo.txt is generated; lasting task changes belong in docs\BACKLOG.md.
start "" /D "%PROJECT_ROOT%" "%PROJECT_EXE%" "%PROJECT_TODO%" --config-dir "%PROJECT_CONFIG%"
exit /b %errorlevel%
