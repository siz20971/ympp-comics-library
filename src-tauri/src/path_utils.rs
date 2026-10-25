use std::path::{Component, Path, PathBuf};

/// Returns the portable base directory of the application (where the executable lives).
pub fn get_app_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let s = parent.to_string_lossy().replace('\\', "/");
            // If running in development target directory, fallback to current working directory
            if s.contains("/target/debug") || s.contains("/target/release") {
                if let Ok(cwd) = std::env::current_dir() {
                    return normalize_path(&cwd);
                }
            }
            return normalize_path(parent);
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    normalize_path(&cwd)
}

/// Normalizes path separators and cleans up `.` and `..`
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(last) = components.last() {
                    match last {
                        Component::Normal(_) => {
                            components.pop();
                        }
                        _ => components.push(comp),
                    }
                } else {
                    components.push(comp);
                }
            }
            _ => components.push(comp),
        }
    }
    let res: PathBuf = components.into_iter().collect();
    // On Windows, if original path is UNC starting with // or \\, ensure the result starts with \\
    #[cfg(target_os = "windows")]
    {
        let raw = path.to_string_lossy();
        if raw.starts_with(r"\\") || raw.starts_with("//") {
            let s = res.to_string_lossy();
            let clean = format!(r"\\{}", s.trim_start_matches('\\').trim_start_matches('/'));
            return PathBuf::from(clean);
        }
    }
    res
}

/// Converts an absolute or target path into a relative path string with respect to base_dir,
/// if they share the same prefix/drive. If on different drives or UNC, returns the normalized absolute path.
pub fn to_relative_path(target: &Path, base: &Path) -> String {
    let target_norm = normalize_path(target);
    let base_norm = normalize_path(base);

    let target_comps: Vec<_> = target_norm.components().collect();
    let base_comps: Vec<_> = base_norm.components().collect();

    // Check if both paths share the same root/prefix (e.g. C: or D:)
    if target_comps.is_empty() || base_comps.is_empty() {
        let s = target_norm.to_string_lossy();
        if s.starts_with(r"\\") || s.starts_with("//") {
            let trimmed = s.trim_start_matches('\\').trim_start_matches('/');
            return format!(r"\\{}", trimmed.replace('/', "\\"));
        }
        return target_norm.to_string_lossy().replace('\\', "/");
    }

    if target_comps[0] != base_comps[0] {
        // Different drives (e.g. C: vs D:) or UNC network path vs local drive
        let s = target_norm.to_string_lossy();
        if s.starts_with(r"\\") || s.starts_with("//") {
            let trimmed = s.trim_start_matches('\\').trim_start_matches('/');
            return format!(r"\\{}", trimmed.replace('/', "\\"));
        }
        return target_norm.to_string_lossy().replace('\\', "/");
    }

    // Find common prefix length
    let mut common_len = 0;
    while common_len < target_comps.len()
        && common_len < base_comps.len()
        && target_comps[common_len] == base_comps[common_len]
    {
        common_len += 1;
    }

    // Number of `..` needed to get from base to common ancestor
    let ups = base_comps.len() - common_len;
    let mut result_parts = Vec::new();

    for _ in 0..ups {
        result_parts.push("..");
    }

    for comp in &target_comps[common_len..] {
        if let Component::Normal(os_str) = comp {
            result_parts.push(os_str.to_str().unwrap_or(""));
        }
    }

    if result_parts.is_empty() {
        ".".to_string()
    } else {
        result_parts.join("/")
    }
}

/// Resolves a path (which might be relative to base_dir or absolute) to an absolute PathBuf.
pub fn resolve_to_absolute(path_str: &str, base: &Path) -> PathBuf {
    if path_str.starts_with(r"\\") || path_str.starts_with("//") {
        let clean = format!(r"\\{}", path_str.trim_start_matches('\\').trim_start_matches('/').replace('/', "\\"));
        return normalize_path(Path::new(&clean));
    }
    let p = Path::new(path_str);
    if p.is_absolute() {
        normalize_path(p)
    } else {
        normalize_path(&base.join(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_path_calculation() {
        let base = Path::new(r"D:\PortableApp\Viewer");
        let target1 = Path::new(r"D:\PortableApp\Comics");
        assert_eq!(to_relative_path(target1, base), "../Comics");

        let target2 = Path::new(r"D:\PortableApp\Viewer\library");
        assert_eq!(to_relative_path(target2, base), "library");

        let target3 = Path::new(r"D:\PortableApp\Viewer");
        assert_eq!(to_relative_path(target3, base), ".");

        let target_diff_drive = Path::new(r"C:\Comics");
        assert_eq!(
            to_relative_path(target_diff_drive, base),
            r"C:/Comics"
        );
    }

    #[test]
    fn test_resolve_to_absolute() {
        let base = Path::new(r"D:\PortableApp\Viewer");
        assert_eq!(
            resolve_to_absolute("../Comics", base),
            PathBuf::from(r"D:\PortableApp\Comics")
        );
        assert_eq!(
            resolve_to_absolute("library", base),
            PathBuf::from(r"D:\PortableApp\Viewer\library")
        );
        assert_eq!(
            resolve_to_absolute(r"E:\Comics", base),
            PathBuf::from(r"E:\Comics")
        );
    }

    #[test]
    fn test_unc_paths() {
        let base = Path::new(r"D:\PortableApp\Viewer");
        let unc = Path::new(r"\\networkserver\comics\folder");
        let rel = to_relative_path(unc, base);
        assert_eq!(rel, r"\\networkserver\comics\folder");

        let resolved = resolve_to_absolute(&rel, base);
        assert_eq!(resolved, PathBuf::from(r"\\networkserver\comics\folder"));

        let unc_fwd = "//networkserver/comics/folder";
        let resolved_fwd = resolve_to_absolute(unc_fwd, base);
        assert_eq!(resolved_fwd, PathBuf::from(r"\\networkserver\comics\folder"));
    }
}
