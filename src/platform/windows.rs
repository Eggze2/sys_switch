//! Windows boot manager implementation using BCD WMI API.
//!
//! This module uses Windows Management Instrumentation (WMI) to interact
//! with the Boot Configuration Data (BCD) store.

use anyhow::{anyhow, Result};
use regex::Regex;
use serde::Deserialize;
use std::process::Command;
use std::fs::OpenOptions;
use std::io::Write;
use wmi::WMIConnection;

use super::{BootEntry, BootManager};

/// BCD WMI namespace
const BCD_NAMESPACE: &str = "root\\WMI";

/// GUID regex pattern
const GUID_PATTERN: &str = r"\{[0-9a-fA-F-]{36}\}";

/// Windows boot manager using BCD WMI API.
pub struct WindowsBootManager {
    show_recovery: bool,
}

/// BCD Object from WMI
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct BcdObject {
    Id: String,
    Type: u32,
}

impl WindowsBootManager {
    /// Create a new Windows boot manager.
    pub fn new() -> Self {
        Self {
            show_recovery: false,
        }
    }

    /// Set whether to show recovery environment entries.
    pub fn with_show_recovery(mut self, show: bool) -> Self {
        self.show_recovery = show;
        self
    }

    /// Open a WMI connection to the BCD store.
    fn open_bcd_connection(&self) -> Result<WMIConnection> {
        // let com = COMLibrary::new()?; // Removed in wmi 0.14+
        let wmi = WMIConnection::with_namespace_path(BCD_NAMESPACE)?;
        Ok(wmi)
    }

    /// Check if running as administrator.
    fn is_admin() -> bool {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
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
            ).is_err() {
                return false;
            }

            elevation.TokenIsElevated != 0
        }
    }

    /// Check if a description indicates a recovery environment.
    fn is_recovery_environment(description: &str) -> bool {
        let indicators = [
            "Windows Recovery Environment",
            "Windows 恢复环境",
            "winre.wim",
            "recovery",
            "恢复",
        ];
        let desc_lower = description.to_lowercase();
        indicators.iter().any(|i| desc_lower.contains(&i.to_lowercase()))
    }

    /// Parse boot entries using bcdedit as fallback (WMI BCD API is complex).
    /// Uses CREATE_NO_WINDOW to avoid window flicker.
    fn list_entries_bcdedit(&self) -> Result<Vec<BootEntry>> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = Command::new("bcdedit")
            .args(["/enum", "firmware"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;

        if !output.status.success() {
            // Decode stderr as GBK
            let (decoded_err, _, _) = encoding_rs::GBK.decode(&output.stderr);
            return Err(anyhow!("bcdedit failed: {}", decoded_err));
        }

        // Decode stdout as GBK (Chinese Windows uses GBK/GB2312 encoding)
        let (text, _, _) = encoding_rs::GBK.decode(&output.stdout);
        self.parse_bcdedit_output(&text)
    }

    /// Parse bcdedit output to extract boot entries.
    fn parse_bcdedit_output(&self, text: &str) -> Result<Vec<BootEntry>> {
        let mut entries = Vec::new();
        let guid_re = Regex::new(GUID_PATTERN)?;
        let desc_re = Regex::new(r"(?im)^(?:description|描述|说明|說明)\s+(.+)$")?;
        let default_re = Regex::new(&format!(r"(?im)^\s*(?:default|默认)\s+({})", GUID_PATTERN))?;
        let bootseq_re = Regex::new(&format!(r"(?im)^\s*(?:bootsequence|启动序列)\s+((?:{}\s*)+)", GUID_PATTERN))?;

        // Find default and boot sequence
        let _current_fw = default_re.captures(text).map(|c| c.get(1).unwrap().as_str().to_string());
        let next_seq: Vec<String> = bootseq_re
            .captures(text)
            .map(|c| {
                guid_re
                    .find_iter(c.get(1).unwrap().as_str())
                    .map(|m| m.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Debug: write raw bcdedit output to file
        if let Ok(mut file) = OpenOptions::new().create(true).write(true).truncate(true).open("debug.log") {
            let _ = writeln!(file, "[DEBUG] bcdedit output length: {} bytes", text.len());
            let _ = writeln!(file, "[DEBUG] Raw output:\n{}", text);
            let _ = writeln!(file, "\n--- END RAW OUTPUT ---\n");

            // Parse blocks - handle both CRLF and LF line endings
            let blocks: Vec<&str> = if text.contains("\r\n\r\n") {
                text.split("\r\n\r\n").collect()
            } else {
                text.split("\n\n").collect()
            };
            let _ = writeln!(file, "[DEBUG] Found {} blocks", blocks.len());
            
            for (i, block) in blocks.iter().enumerate() {
                let _ = writeln!(file, "\n[DEBUG] Block {}: {} chars", i, block.len());
                let _ = writeln!(file, "Content:\n{}", block);
                let _ = writeln!(file, "--- END BLOCK {} ---", i);
            }
        }

        // Parse blocks - handle both CRLF and LF line endings
        let blocks: Vec<&str> = if text.contains("\r\n\r\n") {
            text.split("\r\n\r\n").collect()
        } else {
            text.split("\n\n").collect()
        };
        
        for (i, block) in blocks.iter().enumerate() {
            // Look for identifier
            let pattern = format!(r"(?:identifier|标识符)\s+({}|\{{[^}}]+\}})", GUID_PATTERN);

            // Log regex pattern and block for debugging
            if let Ok(mut file) = OpenOptions::new().append(true).open("debug.log") {
                let _ = writeln!(file, "\n[DEBUG] Block {} regex pattern: {}", i, pattern);
                let _ = writeln!(file, "[DEBUG] Block {} testing regex match...", i);
            }
            
            let id_re = Regex::new(&pattern)?;
            if let Some(id_cap) = id_re.captures(block) {
                if let Ok(mut file) = OpenOptions::new().append(true).open("debug.log") {
                    let _ = writeln!(file, "[DEBUG] Block {} MATCHED! Captured: {:?}", i, id_cap.get(1).map(|m| m.as_str()));
                }
                
                let id = id_cap.get(1).unwrap().as_str().to_string();
                let description = desc_re
                    .captures(block)
                    .map(|c| c.get(1).unwrap().as_str().trim().to_string())
                    .unwrap_or_else(|| id.clone());

                if let Ok(mut file) = OpenOptions::new().append(true).open("debug.log") {
                    let _ = writeln!(file, "[DEBUG] Block {} id={}, description={}", i, id, description);
                }

                // Skip recovery entries if not showing
                if !self.show_recovery && Self::is_recovery_environment(&description) {
                    if let Ok(mut file) = OpenOptions::new().append(true).open("debug.log") {
                        let _ = writeln!(file, "[DEBUG] Block {} SKIPPED: recovery environment", i);
                    }
                    continue;
                }

                // Skip firmware boot manager itself
                if block.contains("Firmware Boot Manager") || block.contains("固件启动管理器") {
                    if let Ok(mut file) = OpenOptions::new().append(true).open("debug.log") {
                        let _ = writeln!(file, "[DEBUG] Block {} SKIPPED: firmware boot manager", i);
                    }
                    continue;
                }

                let is_current = i == 1; // First entry after fwbootmgr is typically current
                let is_next = next_seq.first().map_or(false, |n| n == &id);

                if let Ok(mut file) = OpenOptions::new().append(true).open("debug.log") {
                    let _ = writeln!(file, "[DEBUG] Block {} ADDED as entry: id={}, desc={}", i, id, description);
                }

                entries.push(
                    BootEntry::new(id, description)
                        .with_current(is_current)
                        .with_next(is_next),
                );
            }
        }

        Ok(entries)
    }

    /// Enable a specific privilege for the current process token
    fn enable_privilege(name: &str) -> Result<()> {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{HANDLE, LUID};
        use windows::Win32::Security::{
            AdjustTokenPrivileges, LookupPrivilegeValueW, TOKEN_ADJUST_PRIVILEGES, TOKEN_QUERY,
            TOKEN_PRIVILEGES, SE_PRIVILEGE_ENABLED, LUID_AND_ATTRIBUTES,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        unsafe {
            // Open process token
            let mut token_handle = HANDLE::default();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token_handle).is_err() {
                return Err(anyhow!("Failed to open process token"));
            }

            // Lookup privilege LUID
            let mut luid = LUID::default();
            let name_wide: Vec<u16> = OsStr::new(name)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            
            if LookupPrivilegeValueW(None, PCWSTR(name_wide.as_ptr()), &mut luid).is_err() {
                return Err(anyhow!("Failed to lookup privilege '{}'", name));
            }

            // Adjust token privileges
            let mut tp = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
                ..Default::default()
            };

            if AdjustTokenPrivileges(
                token_handle, 
                false, 
                Some(&mut tp), 
                0, 
                None, 
                None
            ).is_err() {
                return Err(anyhow!("Failed to adjust token privilege '{}'", name));
            }

            Ok(())
        }
    }
}

impl Default for WindowsBootManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BootManager for WindowsBootManager {
    fn available(&self) -> bool {
        // Try to connect to WMI or run bcdedit
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        Command::new("bcdedit")
            .args(["/enum", "firmware"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_entries(&self) -> Result<Vec<BootEntry>> {
        // Try WMI first, fallback to bcdedit
        let result = match self.open_bcd_connection() {
            Ok(wmi) => {
                // BCD WMI queries are complex, for now use bcdedit as reliable fallback
                // The WMI BcdObject class requires specific handling
                // Future: implement full WMI-based listing
                let _ = wmi; // Acknowledge connection works
                self.list_entries_bcdedit()
            }
            Err(_) => self.list_entries_bcdedit(),
        };
        
        // Log the final result
        if let Ok(mut file) = OpenOptions::new().append(true).open("debug.log") {
            match &result {
                Ok(entries) => {
                    let _ = writeln!(file, "\n[DEBUG] list_entries() returning {} entries", entries.len());
                }
                Err(e) => {
                    let _ = writeln!(file, "\n[DEBUG] list_entries() returning error: {}", e);
                }
            }
        }
        
        result
    }

    fn set_next(&self, entry_id: &str) -> Result<String> {
        if !Self::is_admin() {
            return Err(anyhow!("Administrator privileges required to modify BCD"));
        }

        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Ensure entry_id has braces
        let eid = if entry_id.starts_with('{') {
            entry_id.to_string()
        } else {
            format!("{{{}}}", entry_id)
        };

        // Get firmware manager GUID
        let output = Command::new("bcdedit")
            .args(["/v", "/enum", "firmware"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;

        let (text, _, _) = encoding_rs::GBK.decode(&output.stdout);
        let _guid_re = Regex::new(GUID_PATTERN)?;
        
        // Find firmware boot manager GUID
        let mut fw_manager_guid = None;
        for block in text.split("\r\n\r\n") {
            if block.contains("Firmware Boot Manager") || block.contains("固件启动管理器") {
                let id_re = Regex::new(&format!(r"(?:identifier|标识符)\s+({})", GUID_PATTERN))?;
                if let Some(cap) = id_re.captures(block) {
                    fw_manager_guid = Some(cap.get(1).unwrap().as_str().to_string());
                    break;
                }
            }
        }

        let fw_guid = fw_manager_guid.ok_or_else(|| anyhow!("Cannot find Firmware Boot Manager GUID"))?;;

        // Try bootsequence first
        let output = Command::new("bcdedit")
            .args(["/set", &fw_guid, "bootsequence", &eid])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;

        if output.status.success() {
            return Ok(format!("Set next boot entry: {}", entry_id));
        }

        // Fallback to displayorder /addfirst
        let output = Command::new("bcdedit")
            .args(["/set", &fw_guid, "displayorder", &eid, "/addfirst"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;

        if output.status.success() {
            return Ok(format!("Set {} as default boot entry (persistent)", entry_id));
        }

        let (stderr, _, _) = encoding_rs::GBK.decode(&output.stderr);
        Err(anyhow!("Failed to set boot entry: {}", stderr))
    }

    fn reboot_now(&self) -> Result<()> {
        if !Self::is_admin() {
            return Err(anyhow!("Administrator privileges required to reboot"));
        }

        use windows::Win32::System::Shutdown::{
            ExitWindowsEx, EWX_REBOOT, SHTDN_REASON_FLAG_PLANNED,
        };

        // Enable SeShutdownPrivilege
        if let Err(e) = Self::enable_privilege("SeShutdownPrivilege") {
            return Err(anyhow!("Failed to enable shutdown privilege: {}", e));
        }

        unsafe {
            if let Err(e) = ExitWindowsEx(EWX_REBOOT, SHTDN_REASON_FLAG_PLANNED) {
                return Err(anyhow!("Failed to reboot system: {}", e));
            }
        }
        Ok(())
    }


}
