//! Windows registry Run-key autostart.

#[cfg(windows)]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_WRITE,
        REG_SZ,
    };

    const SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    const VALUE: &str = "RestReminderPet";

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.to_string_lossy();
    // Quote path for spaces.
    let cmd = format!("\"{}\"", exe_str);

    let subkey_wide: Vec<u16> = std::ffi::OsStr::new(SUBKEY)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let value_wide: Vec<u16> = std::ffi::OsStr::new(VALUE)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey = std::mem::zeroed();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_wide.as_ptr()),
            Some(0),
            KEY_WRITE,
            &mut hkey,
        );
        if status != ERROR_SUCCESS {
            return Err(format!("打开注册表失败: {status:?}"));
        }

        let result = if enabled {
            let data: Vec<u16> = std::ffi::OsStr::new(&cmd)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let bytes = std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * 2,
            );
            let st = RegSetValueExW(
                hkey,
                PCWSTR(value_wide.as_ptr()),
                Some(0),
                REG_SZ,
                Some(bytes),
            );
            if st != ERROR_SUCCESS {
                Err(format!("写入自启失败: {st:?}"))
            } else {
                Ok(())
            }
        } else {
            let st = RegDeleteValueW(hkey, PCWSTR(value_wide.as_ptr()));
            // ignore missing value
            let _ = st;
            Ok(())
        };

        let _ = RegCloseKey(hkey);
        result
    }
}

#[cfg(not(windows))]
pub fn set_autostart(_enabled: bool) -> Result<(), String> {
    Ok(())
}
