//! Find the actual Codex ancestor without assigning a shell or desktop PID
//! to CLAUDE_PID. A process shared by chats is only a lifetime bound.

pub(super) fn codex_pid() -> Option<u32> {
    #[cfg(windows)]
    {
        ancestor(&windows::listing()?, std::process::id())
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(any(windows, test))]
fn ancestor(processes: &[(u32, u32, String)], mut pid: u32) -> Option<u32> {
    for _ in processes {
        let (_, parent, image) = processes.iter().find(|(id, _, _)| *id == pid)?;
        if image.eq_ignore_ascii_case("codex.exe") {
            return Some(pid);
        }
        pid = *parent;
    }
    None
}

#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "Windows Toolhelp reads the command's real process ancestry"
)]
mod windows {
    use std::ffi::c_void;

    #[repr(C)]
    struct Entry {
        size: u32,
        usage: u32,
        id: u32,
        heap: usize,
        module: u32,
        threads: u32,
        parent: u32,
        priority: i32,
        flags: u32,
        image: [u16; 260],
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, id: u32) -> *mut c_void;
        fn Process32FirstW(snapshot: *mut c_void, entry: *mut Entry) -> i32;
        fn Process32NextW(snapshot: *mut c_void, entry: *mut Entry) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    struct Snapshot(*mut c_void);
    impl Drop for Snapshot {
        fn drop(&mut self) {
            // SAFETY: this handle comes from a successful snapshot and is closed once.
            unsafe { CloseHandle(self.0) };
        }
    }

    pub(super) fn listing() -> Option<Vec<(u32, u32, String)>> {
        // SAFETY: TH32CS_SNAPPROCESS takes no pointer or process-specific input.
        let handle = unsafe { CreateToolhelp32Snapshot(2, 0) };
        if handle == (-1_isize) as *mut c_void {
            return None;
        }
        let snapshot = Snapshot(handle);
        // SAFETY: Entry contains only integer fields, all valid when zeroed.
        let mut entry: Entry = unsafe { std::mem::zeroed() };
        entry.size = u32::try_from(std::mem::size_of::<Entry>()).ok()?;
        // SAFETY: the live snapshot and writable, sized Entry satisfy Toolhelp's contract.
        if unsafe { Process32FirstW(snapshot.0, &raw mut entry) } == 0 {
            return None;
        }
        let mut processes = Vec::new();
        loop {
            let len = entry
                .image
                .iter()
                .position(|word| *word == 0)
                .unwrap_or(entry.image.len());
            processes.push((
                entry.id,
                entry.parent,
                String::from_utf16_lossy(&entry.image[..len]),
            ));
            // SAFETY: the snapshot and Entry remain live throughout enumeration.
            if unsafe { Process32NextW(snapshot.0, &raw mut entry) } == 0 {
                break;
            }
        }
        Some(processes)
    }
}

#[cfg(test)]
mod tests {
    use super::ancestor;

    #[test]
    fn only_the_actual_codex_ancestor_owns_activity() {
        let processes = vec![
            (1, 0, "Codex.exe".into()),
            (2, 1, "codex.exe".into()),
            (3, 2, "pwsh.exe".into()),
            (4, 3, "xtask.exe".into()),
            (5, 1, "claude.exe".into()),
        ];
        assert_eq!(ancestor(&processes, 4), Some(2));
        assert_eq!(ancestor(&processes, 99), None);
        assert_eq!(ancestor(&[(4, 4, "xtask.exe".into())], 4), None);
    }
}
