use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

static LAST_LAUNCH_RECORD: Mutex<Option<(String, Instant)>> = Mutex::new(None);

fn should_suppress_duplicate_launch(key: &str, cooldown_ms: u64) -> bool {
    if let Ok(mut guard) = LAST_LAUNCH_RECORD.lock() {
        let now = Instant::now();
        if let Some((ref last_key, ref last_time)) = *guard {
            if last_key == key && now.duration_since(*last_time).as_millis() < cooldown_ms as u128 {
                return true;
            }
        }
        *guard = Some((key.to_string(), now));
    }
    false
}

pub struct Launcher;

impl Launcher {
    /// Opens a comic file using a custom viewer or the default OS program
    pub fn open_file(file_path: &str, custom_viewer: Option<&str>) -> Result<(), String> {
        let clean_path = Self::normalize_windows_path(file_path);
        if should_suppress_duplicate_launch(&format!("open:{}", clean_path), 500) {
            return Ok(());
        }

        let p = Path::new(&clean_path);
        if !p.exists() {
            return Err(format!("파일이 존재하지 않거나 접근할 수 없습니다: {}", clean_path));
        }

        if let Some(viewer) = custom_viewer {
            let viewer = viewer.trim();
            if !viewer.is_empty() {
                let clean_viewer = Self::normalize_windows_path(viewer);
                let v_path = Path::new(&clean_viewer);
                if !v_path.exists() {
                    return Err(format!("지정된 외부 뷰어 프로그램을 찾을 수 없습니다: {}", clean_viewer));
                }

                Command::new(&clean_viewer)
                    .arg(&clean_path)
                    .spawn()
                    .map_err(|e| format!("외부 뷰어 실행 실패: {}", e))?;
                return Ok(());
            }
        }

        // Fallback to OS default handler
        #[cfg(target_os = "windows")]
        {
            // First try open::that_detached
            if let Err(open_err) = open::that_detached(&clean_path) {
                // If failed (e.g. no associated program for .cbz), try shell OpenAs dialog
                let status = Command::new("rundll32.exe")
                    .arg("shell32.dll,OpenAs_RunDLL")
                    .arg(&clean_path)
                    .spawn();

                if status.is_err() {
                    return Err(format!(
                        "파일 열기 실패: {}\n(설정에서 외부 만화 뷰어 프로그램을 지정해 주세요)",
                        open_err
                    ));
                }
            }
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            open::that_detached(&clean_path).map_err(|e| format!("기본 프로그램으로 열기 실패: {}", e))?;
            Ok(())
        }
    }

    /// Reveals a file or directory in Windows File Explorer
    pub fn show_in_file_manager(path_str: &str) -> Result<(), String> {
        let clean_path = Self::normalize_windows_path(path_str);
        if should_suppress_duplicate_launch(&format!("explorer:{}", clean_path), 500) {
            return Ok(());
        }

        let p = Path::new(&clean_path);
        if !p.exists() {
            return Err(format!("경로가 존재하지 않습니다: {}", clean_path));
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;

            if p.is_dir() {
                // Open directory directly: use raw_arg with quotes around the path
                let dir_arg = format!(r#""{}""#, clean_path);
                Command::new("explorer.exe")
                    .raw_arg(&dir_arg)
                    .spawn()
                    .map_err(|e| format!("탐색기 폴더 열기 실패: {}", e))?;
            } else {
                // Select file in explorer: MUST be `/select,"<path>"` where quotes only enclose the path!
                // Using standard .arg() wraps the whole argument including /select,, which breaks explorer.
                let select_arg = format!(r#"/select,"{}""#, clean_path);
                Command::new("explorer.exe")
                    .raw_arg(&select_arg)
                    .spawn()
                    .map_err(|e| format!("탐색기 파일 선택 열기 실패: {}", e))?;
            }
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            let target = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
            open::that_detached(target).map_err(|e| format!("파일 관리자 열기 실패: {}", e))?;
            Ok(())
        }
    }

    /// Normalizes path to standard Windows backslash representation
    pub fn normalize_windows_path(raw_path: &str) -> String {
        // If raw_path starts with `UNC\` or `UNC/`, recover it to `\\`
        let clean_raw = if let Some(rest) = raw_path.strip_prefix(r"UNC\").or_else(|| raw_path.strip_prefix("UNC/")) {
            format!(r"\\{}", rest)
        } else {
            raw_path.to_string()
        };

        let p = Path::new(&clean_raw);
        // If canonicalize succeeds, properly handle Windows UNC and local drive prefixes
        if let Ok(canonical) = p.canonicalize() {
            let s = canonical.to_string_lossy().to_string();
            // Windows UNC canonical format: `\\?\UNC\server\share\...`
            if let Some(unc) = s.strip_prefix(r"\\?\UNC\") {
                return format!(r"\\{}", unc).replace('/', "\\");
            }
            // Windows local drive canonical format: `\\?\C:\...`
            if let Some(stripped) = s.strip_prefix(r"\\?\") {
                return stripped.replace('/', "\\");
            }
            return s.replace('/', "\\");
        }

        // Fallback for paths that cannot be canonicalized directly
        let normalized = clean_raw.replace('/', "\\");
        if clean_raw.starts_with("//") || clean_raw.starts_with(r"\\") {
            let trimmed = normalized.trim_start_matches('\\');
            return format!(r"\\{}", trimmed);
        }
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_windows_path() {
        let path = "D:/folder/file.zip";
        let norm = Launcher::normalize_windows_path(path);
        assert!(!norm.contains('/'));
        assert!(norm.contains('\\'));

        // Test UNC path normalization
        let unc_local = r"\\localhost\c$\Windows\notepad.exe";
        let norm_unc = Launcher::normalize_windows_path(unc_local);
        assert!(norm_unc.starts_with(r"\\"), "Normalized UNC path must start with \\\\, got: {}", norm_unc);
        assert!(!norm_unc.starts_with(r"UNC\"), "UNC prefix must not lose double backslash");

        let unc_forward = "//localhost/c$/Windows/notepad.exe";
        let norm_fwd = Launcher::normalize_windows_path(unc_forward);
        assert!(norm_fwd.starts_with(r"\\"), "Forward-slash UNC must be normalized to \\\\, got: {}", norm_fwd);

        // Test UNC prefix recovery if already damaged
        let unc_damaged = r"UNC\localhost\c$\Windows\notepad.exe";
        let norm_damaged = Launcher::normalize_windows_path(unc_damaged);
        assert!(norm_damaged.starts_with(r"\\"), "Damaged UNC prefix must be recovered");
    }
}
