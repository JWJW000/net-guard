//! Process information module for macOS
//! 
//! Uses libproc to get process details.

use std::path::PathBuf;

/// Process information structure
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub bundle_path: Option<PathBuf>,
    pub icon_path: Option<String>,
}

/// Get process information for a given PID
#[cfg(target_os = "macos")]
pub fn get_process_info(pid: u32) -> Option<ProcessInfo> {
    let mut name_buf = [0u8; 256];
    let name_len = unsafe {
        libc::proc_pidpath(pid as libc::pid_t, name_buf.as_mut_ptr() as *mut libc::c_void, name_buf.len() as u32)
    };
    
    if name_len == 0 {
        return None;
    }
    
    let path = PathBuf::from(String::from_utf8_lossy(&name_buf[..name_len as usize]).into_owned());
    let name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    Some(ProcessInfo {
        pid,
        name,
        bundle_path: None,
        icon_path: None,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn get_process_info(pid: u32) -> Option<ProcessInfo> {
    let _ = pid;
    None
}
