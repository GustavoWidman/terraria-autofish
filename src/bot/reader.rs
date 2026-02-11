use std::mem;

use eyre::{Result, eyre};
use log::{debug, info};
use rayon::prelude::*;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_NOACCESS, PAGE_PROTECTION_FLAGS,
    VirtualQueryEx,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};

use crate::utils;

pub struct ProcessReader {
    handle: HANDLE,
}

impl ProcessReader {
    pub fn new() -> Result<Self> {
        let pid = utils::process::find_process_by_name("Terraria.exe")?;
        info!("found terraria.exe (PID: {})", pid);

        unsafe {
            let handle = OpenProcess(
                PROCESS_VM_READ
                    | PROCESS_VM_WRITE
                    | PROCESS_VM_OPERATION
                    | PROCESS_QUERY_INFORMATION,
                false,
                pid,
            )
            .map_err(|e| eyre!("failed to open process:\n{:#?}", e))?;

            Ok(ProcessReader { handle })
        }
    }

    pub fn read_memory<T: Copy>(&self, address: usize) -> Result<T> {
        unsafe {
            let mut buffer: T = mem::zeroed();

            ReadProcessMemory(
                self.handle,
                address as *const _,
                &mut buffer as *mut T as *mut _,
                mem::size_of::<T>(),
                None,
            )
            .map_err(|e| eyre!("ReadProcessMemory failed:\n{:#?}", e))?;

            Ok(buffer)
        }
    }

    pub fn write_memory<T: Copy>(&self, address: usize, value: &T) -> Result<()> {
        unsafe {
            windows::Win32::System::Diagnostics::Debug::WriteProcessMemory(
                self.handle,
                address as *const _,
                value as *const T as *const _,
                mem::size_of::<T>(),
                None,
            )
            .map_err(|e| eyre!("WriteProcessMemory failed:\n{:#?}", e))?;

            Ok(())
        }
    }

    fn get_readable_regions(&self, start: usize, end: usize) -> Vec<(usize, usize)> {
        let mut regions = Vec::new();
        let mut current = start;

        unsafe {
            while current < end {
                let mut mbi: MEMORY_BASIC_INFORMATION = mem::zeroed();

                if VirtualQueryEx(
                    self.handle,
                    Some(current as *const _),
                    &mut mbi,
                    mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                ) == 0
                {
                    break;
                }

                if mbi.State == MEM_COMMIT
                    && (mbi.Protect & PAGE_NOACCESS) == PAGE_PROTECTION_FLAGS(0)
                    && (mbi.Protect & PAGE_GUARD) == PAGE_PROTECTION_FLAGS(0)
                {
                    let region_start = (mbi.BaseAddress as usize).max(start);
                    let region_end = ((mbi.BaseAddress as usize) + mbi.RegionSize).min(end);

                    if region_start < region_end {
                        regions.push((region_start, region_end - region_start));
                    }
                }

                current = mbi.BaseAddress as usize + mbi.RegionSize;
            }
        }

        debug!(
            "region scan results:\ntotal regions: {}\nregion range: 0x{:X} - 0x{:X}\ntotal size: {:.2} MB",
            regions.len(),
            start,
            end,
            (regions.iter().map(|(_, size)| *size as f64).sum::<f64>() / (1024.0 * 1024.0))
        );

        regions
    }

    pub fn pattern_scan(&self, start: usize, end: usize, pattern: &[Option<u8>]) -> Result<usize> {
        let regions = self.get_readable_regions(start, end);

        regions
            .into_par_iter()
            .find_map_any(|(addr, size)| {
                let buffer = self.read_bytes(addr, size).ok()?;
                let offset = utils::mem::find_pattern_in_buffer(&buffer, pattern)?;
                Some(addr + offset)
            })
            .ok_or_else(|| eyre!("could not find pattern across all readable memory regions"))
    }

    fn read_bytes(&self, address: usize, size: usize) -> Result<Vec<u8>> {
        unsafe {
            let mut buffer = vec![0u8; size];

            ReadProcessMemory(
                self.handle,
                address as *const _,
                buffer.as_mut_ptr() as *mut _,
                size,
                None,
            )
            .map_err(|e| eyre!("failed to read:\n{:#?}", e))?;

            Ok(buffer)
        }
    }
}

impl Drop for ProcessReader {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

// windows handles are thread-safe for reading, and we only use this struct for reading, so it should be safe to implement Sync and Send
// NOTE: ever since the [`write_memory`] functionality was added, the safety of this has become questionable
// no issues since we're writing in a moment where the memory is not being actively read or written to by terraria and we're only writing once to it but still risky since we're "on our own".
unsafe impl Sync for ProcessReader {}
unsafe impl Send for ProcessReader {}
