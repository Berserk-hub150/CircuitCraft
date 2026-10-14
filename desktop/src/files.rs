use std::path::{Path, PathBuf};
#[cfg(windows)]
pub fn choose(save: bool, title: &str, ext: &str, start: Option<&Path>) -> Option<PathBuf> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR,
        OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };
    let filter = format!("{title} (*.{ext})\0*.{ext}\0All files\0*.*\0\0")
        .encode_utf16()
        .collect::<Vec<_>>();
    let ext = ext.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut buffer = vec![0u16; 32768];
    if let Some(p) = start {
        let p = p.as_os_str().encode_wide().take(32766).collect::<Vec<_>>();
        buffer[..p.len()].copy_from_slice(&p);
    }
    let mut data: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    data.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    data.lpstrFilter = filter.as_ptr();
    data.nFilterIndex = 1;
    data.lpstrFile = buffer.as_mut_ptr();
    data.nMaxFile = buffer.len() as u32;
    data.lpstrDefExt = ext.as_ptr();
    data.Flags = OFN_EXPLORER
        | OFN_NOCHANGEDIR
        | OFN_PATHMUSTEXIST
        | if save {
            OFN_OVERWRITEPROMPT
        } else {
            OFN_FILEMUSTEXIST
        };
    if unsafe {
        if save {
            GetSaveFileNameW(&mut data)
        } else {
            GetOpenFileNameW(&mut data)
        }
    } == 0
    {
        return None;
    }
    let end = buffer.iter().position(|v| *v == 0).unwrap_or(buffer.len());
    Some(PathBuf::from(std::ffi::OsString::from_wide(&buffer[..end])))
}
#[cfg(not(windows))]
pub fn choose(save: bool, title: &str, ext: &str, start: Option<&Path>) -> Option<PathBuf> {
    let mut cmd = std::process::Command::new("zenity");
    cmd.args(["--file-selection", "--title", title]);
    if save {
        cmd.args(["--save", "--confirm-overwrite"]);
    }
    if let Some(p) = start {
        cmd.arg("--filename").arg(p);
    }
    cmd.arg("--file-filter").arg(format!("{title} | *.{ext}"));
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}
