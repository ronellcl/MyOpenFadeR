//! One running copy per user: a lock on `<data folder>/openfader.lock`.
//!
//! The lock is held by the process until it exits (the OS drops it), so a crash does not leave a
//! stale "already running" state. A second copy returns `Ok(None)` and exits before opening the
//! window, the store or the audio device.

use std::{
    fs::{File, OpenOptions},
    path::Path,
};
#[cfg(not(windows))]
use std::process::Command;

/// Takes the per-user lock. `Ok(Some(file))` keeps it until the `File` is dropped; `Ok(None)` means
/// another copy already has it.
pub fn acquire() -> Result<Option<File>, String> {
    let dir = openfader::paths::data_dir()
        .ok_or_else(|| "no data folder (set HOME or XDG_DATA_HOME)".to_owned())?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    acquire_at(&dir.join("openfader.lock"))
}

/// Says why the second copy is leaving, using the desktop's native dialog when there is one.
/// A second process cannot stay alive usefully: it must exit after telling the user.
pub fn report_already_running() {
    #[cfg(target_os = "linux")]
    {
        let message = "MyOpenFadeR ya está abierto. Cierra la ventana existente antes de abrir otra copia.";
        if Command::new("kdialog")
            .args(["--title", "MyOpenFadeR", "--msgbox", message])
            .status()
            .is_ok()
        {
            return;
        }
        let text = format!("--text={message}");
        if Command::new("zenity")
            .args(["--info", "--title=MyOpenFadeR", &text])
            .status()
            .is_ok()
        {
            return;
        }
        let _ = Command::new("notify-send")
            .args(["--app-name", "MyOpenFadeR", "MyOpenFadeR", message])
            .status();
        return;
    }
    #[cfg(target_os = "macos")]
    {
        let script = "display dialog \"MyOpenFadeR ya está abierto. Cierra la ventana existente antes de abrir otra copia.\" buttons {\"OK\"} default button 1 with title \"MyOpenFadeR\"";
        let _ = Command::new("osascript").args(["-e", script]).status();
        return;
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            MB_ICONINFORMATION, MB_OK, MessageBoxW,
        };
        let text: Vec<u16> = "MyOpenFadeR ya está abierto. Cierra la ventana existente antes de abrir otra copia.\0"
            .encode_utf16()
            .collect();
        let title: Vec<u16> = "MyOpenFadeR\0".encode_utf16().collect();
        // SAFETY: both strings are NUL-terminated and live for the call.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        return;
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    eprintln!("MyOpenFadeR is already running; close the other window first.");
}

fn acquire_at(path: &Path) -> Result<Option<File>, String> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !try_lock(&file)? {
        return Ok(None);
    }
    // Diagnostic only: the lock, not the contents, is what decides.
    use std::io::Write;
    let _ = writeln!(&mut file, "{}", std::process::id());
    Ok(Some(file))
}

#[cfg(unix)]
fn try_lock(file: &File) -> Result<bool, String> {
    use std::os::unix::io::AsRawFd;
    // SAFETY: `file` is open for the duration of the call.
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        return Ok(true);
    }
    let e = std::io::Error::last_os_error();
    if e.kind() == std::io::ErrorKind::WouldBlock {
        Ok(false)
    } else {
        Err(format!("lock: {e}"))
    }
}

#[cfg(windows)]
fn try_lock(file: &File) -> Result<bool, String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;
    // SAFETY: the handle is live and the zeroed OVERLAPPED is a valid initial state.
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        LockFileEx(
            file.as_raw_handle() as _,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            0,
            0,
            &mut overlapped,
        )
    };
    if ok != 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(not(any(unix, windows)))]
fn try_lock(_file: &File) -> Result<bool, String> {
    // No portable advisory lock here: the app still runs, without the guard.
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_copy_is_refused_and_the_lock_is_reusable() {
        let dir = std::env::temp_dir().join(format!("openfader-lock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("openfader.lock");
        let first = acquire_at(&path).unwrap().expect("first lock");
        assert!(acquire_at(&path).unwrap().is_none());
        drop(first);
        assert!(acquire_at(&path).unwrap().is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
