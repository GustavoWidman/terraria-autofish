use std::mem;

use eyre::{Result, eyre};
use windows::Win32::{
    Foundation::CloseHandle,
    System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    },
};

pub fn find_process_by_name(name: &str) -> Result<u32> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .map_err(|e| eyre!("failed to create snapshot:\n{:#?}", e))?;

        let mut entry = PROCESSENTRY32W {
            dwSize: mem::size_of::<PROCESSENTRY32W>() as u32,
            ..mem::zeroed()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let process_name = String::from_utf16_lossy(&entry.szExeFile)
                    .trim_end_matches('\0')
                    .to_lowercase();

                if process_name == name.to_lowercase() {
                    let pid = entry.th32ProcessID;
                    let _ = CloseHandle(snapshot);
                    return Ok(pid);
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        Err(eyre!("process '{}' not found", name))
    }
}
