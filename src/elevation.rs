//! Privilege elevation utilities.

use anyhow::{anyhow, Result};
use std::env;
#[cfg(target_os = "linux")]
use std::process::Command;

/// Check if the process needs elevation and attempt to elevate if needed.
/// Returns `Ok(true)` if elevated instance was launched and current should exit.
/// Returns `Ok(false)` if already elevated or elevation not needed.
pub fn elevate_if_needed(_want_gui: bool) -> Result<bool> {
    if is_elevated() {
        return Ok(false);
    }

    #[cfg(target_os = "windows")]
    {
        elevate_windows()
    }

    #[cfg(target_os = "linux")]
    {
        elevate_linux(want_gui)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Ok(false)
    }
}

/// Check if current process has elevated privileges.
pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        is_elevated_windows()
    }

    #[cfg(target_os = "linux")]
    {
        unsafe { libc::geteuid() == 0 }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
fn is_elevated_windows() -> bool {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token_handle = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut return_length = 0u32;
        let size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

        if GetTokenInformation(
            token_handle,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size,
            &mut return_length,
        )
        .is_err()
        {
            return false;
        }

        elevation.TokenIsElevated != 0
    }
}

#[cfg(target_os = "windows")]
fn elevate_windows() -> Result<bool> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe = env::current_exe()?;
    let args: Vec<String> = env::args().skip(1).collect();
    let args_str = args.join(" ");

    let exe_wide: Vec<u16> = OsStr::new(exe.to_str().unwrap())
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let args_wide: Vec<u16> = OsStr::new(&args_str)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb_wide: Vec<u16> = OsStr::new("runas")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb_wide.as_ptr()),
            PCWSTR(exe_wide.as_ptr()),
            PCWSTR(args_wide.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    // ShellExecuteW returns > 32 on success
    if result.0 as usize > 32 {
        Ok(true)
    } else {
        Err(anyhow!(
            "Failed to elevate: error code {}",
            result.0 as usize
        ))
    }
}

#[cfg(target_os = "linux")]
fn elevate_linux(_want_gui: bool) -> Result<bool> {
    let exe = env::current_exe()?;
    let args: Vec<String> = env::args().skip(1).collect();

    // Try pkexec first
    if let Some(pkexec) = which("pkexec") {
        let mut env_args = Vec::new();
        for key in &[
            "DISPLAY",
            "XAUTHORITY",
            "WAYLAND_DISPLAY",
            "XDG_RUNTIME_DIR",
        ] {
            if let Ok(val) = env::var(key) {
                env_args.push(format!("{}={}", key, val));
            }
        }

        let mut cmd = Command::new(pkexec);
        cmd.arg("env");
        for arg in &env_args {
            cmd.arg(arg);
        }
        cmd.arg(&exe);
        cmd.args(&args);

        if cmd.spawn().is_ok() {
            return Ok(true);
        }
    }

    // Fallback: suggest running with sudo
    Err(anyhow!("Root privileges required. Please run with sudo"))
}

/// Find an executable in PATH.
#[cfg(target_os = "linux")]
fn which(cmd: &str) -> Option<String> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join(cmd);
                if full_path.is_file() {
                    Some(full_path.to_string_lossy().into_owned())
                } else {
                    None
                }
            })
            .next()
    })
}
