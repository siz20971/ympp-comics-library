use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::models::{ReleaseNoteEntry, UpdateCheckResult, UpdateDownloadProgress};

pub const DEFAULT_RELEASES_API_URL: &str =
    "https://api.github.com/repos/siz20971/ympp-comics-library/releases?per_page=30";
pub const RELEASES_PAGE_URL: &str =
    "https://github.com/siz20971/ympp-comics-library/releases";
pub const EXE_ASSET_NAME: &str = "YMPPComicsLibrary.exe";
const USER_AGENT: &str = concat!("YMPPComicsLibrary/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubAssetDto {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubReleaseDto {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub assets: Vec<GitHubAssetDto>,
}

/// Parses a semantic version string such as "v0.2.0" or "0.2.0" into (major, minor, patch).
/// Pre-release suffixes after '-' (if any) are treated as ineligible for stable matching unless stripped carefully.
pub fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() {
        return None;
    }
    let core = trimmed.split(['-', '+']).next()?.trim();
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn normalize_version_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('v') || trimmed.starts_with('V') {
        format!("v{}", trimmed.trim_start_matches(['v', 'V']))
    } else {
        format!("v{}", trimmed)
    }
}

pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    match (parse_semver(candidate), parse_semver(current)) {
        (Some(cand), Some(curr)) => cand > curr,
        _ => false,
    }
}

pub fn resolve_feed_url() -> String {
    std::env::var("YMPP_UPDATE_FEED_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASES_API_URL.to_string())
}

/// Filters non-draft, non-prerelease releases newer than `current_version`,
/// sorts newest-first, and collects all release notes plus the latest executable asset.
pub fn select_newer_releases(
    current_version: &str,
    releases: Vec<GitHubReleaseDto>,
) -> UpdateCheckResult {
    let mut newer: Vec<((u64, u64, u64), GitHubReleaseDto)> = releases
        .into_iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| {
            let semver = parse_semver(&r.tag_name)?;
            if is_newer_version(&r.tag_name, current_version) {
                Some((semver, r))
            } else {
                None
            }
        })
        .collect();

    // Sort descending so newest version is always first
    newer.sort_by(|a, b| b.0.cmp(&a.0));

    if newer.is_empty() {
        return UpdateCheckResult {
            current_version: normalize_version_label(current_version),
            latest_version: normalize_version_label(current_version),
            has_update: false,
            download_url: None,
            asset_name: None,
            asset_size: None,
            html_url: RELEASES_PAGE_URL.to_string(),
            releases: Vec::new(),
            is_dev_build: false,
        };
    }

    let latest_release = &newer[0].1;
    let latest_version = normalize_version_label(&latest_release.tag_name);
    let latest_html_url = latest_release.html_url.clone();

    let exe_asset = latest_release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(EXE_ASSET_NAME))
        .or_else(|| {
            latest_release
                .assets
                .iter()
                .find(|a| a.name.to_lowercase().ends_with(".exe"))
        });

    let notes = newer
        .iter()
        .map(|(_, r)| ReleaseNoteEntry {
            version: normalize_version_label(&r.tag_name),
            title: r
                .name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&r.tag_name)
                .to_string(),
            published_at: r.published_at.clone().unwrap_or_default(),
            body: r.body.clone().unwrap_or_default(),
            html_url: r.html_url.clone(),
        })
        .collect();

    UpdateCheckResult {
        current_version: normalize_version_label(current_version),
        latest_version,
        has_update: true,
        download_url: exe_asset.map(|a| a.browser_download_url.clone()),
        asset_name: exe_asset.map(|a| a.name.clone()),
        asset_size: exe_asset.map(|a| a.size),
        html_url: latest_html_url,
        releases: notes,
        is_dev_build: false,
    }
}

pub async fn fetch_releases_from_url(
    feed_url: &str,
    current_version: &str,
) -> Result<UpdateCheckResult, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    let resp = client
        .get(feed_url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("업데이트 서버 연결 실패: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("업데이트 확인 응답 오류 (HTTP {})", resp.status().as_u16()));
    }

    let releases: Vec<GitHubReleaseDto> = resp
        .json()
        .await
        .map_err(|e| format!("릴리즈 정보 파싱 실패: {}", e))?;

    Ok(select_newer_releases(current_version, releases))
}

pub async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let feed_url = resolve_feed_url();
    let mut result = fetch_releases_from_url(&feed_url, env!("CARGO_PKG_VERSION")).await?;
    result.is_dev_build = std::env::current_exe()
        .map(|p| is_dev_binary_path(&p))
        .unwrap_or(false);
    Ok(result)
}

pub fn is_dev_binary_path(exe_path: &Path) -> bool {
    let normalized = exe_path.to_string_lossy().replace('\\', "/");
    normalized.contains("/target/debug/") || normalized.contains("/target/release/")
}

pub fn cleanup_stale_update_files(app_dir: &Path) {
    let mut dirs = vec![app_dir.to_path_buf()];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if parent != app_dir {
                dirs.push(parent.to_path_buf());
            }
        }
        if let Some(file_name) = exe.file_name().and_then(|n| n.to_str()) {
            for suffix in [".old", ".new"] {
                let candidate = exe.with_file_name(format!("{}{}", file_name, suffix));
                if candidate.exists() {
                    let _ = std::fs::remove_file(candidate);
                }
            }
        }
    }

    for dir in dirs {
        for name in [
            format!("{}.old", EXE_ASSET_NAME),
            format!("{}.new", EXE_ASSET_NAME),
        ] {
            let p = dir.join(name);
            if p.exists() {
                let _ = std::fs::remove_file(p);
            }
        }
    }
}

fn emit_progress(
    app: Option<&AppHandle>,
    downloaded: u64,
    total: Option<u64>,
    stage: &str,
) {
    if let Some(handle) = app {
        let percent = match total {
            Some(t) if t > 0 => ((downloaded as f64 / t as f64) * 100.0).min(100.0),
            _ => 0.0,
        };
        let _ = handle.emit(
            "update://progress",
            UpdateDownloadProgress {
                downloaded,
                total,
                percent,
                stage: stage.to_string(),
            },
        );
    }
}

pub async fn download_update_to_path(
    download_url: &str,
    dest_path: &Path,
    expected_size: Option<u64>,
    app: Option<&AppHandle>,
) -> Result<u64, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("다운로드 클라이언트 생성 실패: {}", e))?;

    let mut resp = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("업데이트 파일 요청 실패: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "업데이트 파일 다운로드 실패 (HTTP {})",
            resp.status().as_u16()
        ));
    }

    let total = expected_size.or_else(|| resp.content_length());
    let mut file = tokio::fs::File::create(dest_path)
        .await
        .map_err(|e| format!("임시 파일 생성 실패: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut last_emitted: u64 = 0;
    emit_progress(app, 0, total, "downloading");

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("다운로드 스트림 오류: {}", e))?
    {
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("임시 파일 쓰기 실패: {}", e))?;
        downloaded += chunk.len() as u64;

        if downloaded - last_emitted >= 64 * 1024 || total == Some(downloaded) {
            last_emitted = downloaded;
            emit_progress(app, downloaded, total, "downloading");
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("파일 플러시 실패: {}", e))?;
    drop(file);

    emit_progress(app, downloaded, total, "verifying");
    validate_downloaded_executable(dest_path, downloaded, expected_size).await?;

    Ok(downloaded)
}

async fn validate_downloaded_executable(
    path: &Path,
    downloaded_size: u64,
    expected_size: Option<u64>,
) -> Result<(), String> {
    if downloaded_size == 0 {
        let _ = tokio::fs::remove_file(path).await;
        return Err("다운로드된 업데이트 파일이 비어 있습니다.".to_string());
    }
    if let Some(exp) = expected_size.filter(|&s| s > 0) {
        if downloaded_size != exp {
            let _ = tokio::fs::remove_file(path).await;
            return Err(format!(
                "다운로드 파일 크기가 일치하지 않습니다 (예상 {} bytes, 실제 {} bytes).",
                exp, downloaded_size
            ));
        }
    }

    let mut f = tokio::fs::File::open(path)
        .await
        .map_err(|e| format!("다운로드 파일 검증 열기 실패: {}", e))?;
    let mut magic = [0u8; 2];
    if f.read_exact(&mut magic).await.is_err() || &magic != b"MZ" {
        drop(f);
        let _ = tokio::fs::remove_file(path).await;
        return Err("유효한 Windows 실행 파일(.exe)이 아닙니다.".to_string());
    }
    Ok(())
}

/// Atomically swaps `current_exe` -> `<exe>.old` and `new_exe` -> `current_exe`.
/// Rolls back if swapping `new_exe` fails.
pub fn swap_executables_with_backup(current_exe: &Path, new_exe: &Path) -> Result<PathBuf, String> {
    let exe_name = current_exe
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "현재 실행 파일 이름을 확인할 수 없습니다.".to_string())?;
    let backup_path = current_exe.with_file_name(format!("{}.old", exe_name));

    if backup_path.exists() {
        std::fs::remove_file(&backup_path)
            .map_err(|e| format!("기존 백업 파일 제거 실패: {}", e))?;
    }

    std::fs::rename(current_exe, &backup_path)
        .map_err(|e| format!("현재 실행 파일 백업 이동 실패: {}", e))?;

    if let Err(err) = std::fs::rename(new_exe, current_exe) {
        let _ = std::fs::rename(&backup_path, current_exe);
        return Err(format!("새 실행 파일 적용 실패 (원본 복구됨): {}", err));
    }

    Ok(backup_path)
}

pub fn rollback_executable_swap(current_exe: &Path, backup_path: &Path) {
    if current_exe.exists() {
        let _ = std::fs::remove_file(current_exe);
    }
    let _ = std::fs::rename(backup_path, current_exe);
}

pub async fn download_and_apply_update(
    app: AppHandle,
    download_url: String,
    expected_size: Option<u64>,
) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("실행 파일 경로를 확인할 수 없습니다: {}", e))?;

    if is_dev_binary_path(&current_exe) {
        return Err(
            "개발 실행 모드에서는 실행 파일 자가 교체가 비활성화되어 있습니다. 릴리즈 실행 파일(YMPPComicsLibrary.exe)에서 이용해 주세요."
                .to_string(),
        );
    }

    let exe_name = current_exe
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "현재 실행 파일 이름을 확인할 수 없습니다.".to_string())?;
    let staged_new_path = current_exe.with_file_name(format!("{}.new", exe_name));

    let downloaded = download_update_to_path(
        &download_url,
        &staged_new_path,
        expected_size,
        Some(&app),
    )
    .await?;

    emit_progress(Some(&app), downloaded, expected_size, "replacing");

    let backup_path = swap_executables_with_backup(&current_exe, &staged_new_path)?;

    emit_progress(Some(&app), downloaded, expected_size, "restarting");

    if let Err(err) = std::process::Command::new(&current_exe).spawn() {
        rollback_executable_swap(&current_exe, &backup_path);
        return Err(format!("새 버전 재실행 실패 (원본 복구됨): {}", err));
    }

    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn sample_release(
        tag: &str,
        body: &str,
        draft: bool,
        prerelease: bool,
        asset_size: u64,
    ) -> GitHubReleaseDto {
        GitHubReleaseDto {
            tag_name: tag.to_string(),
            name: Some(format!("YMPP Comics Library {}", tag)),
            body: Some(body.to_string()),
            draft,
            prerelease,
            published_at: Some("2026-11-15T00:00:00Z".to_string()),
            html_url: format!("https://github.com/test/releases/tag/{}", tag),
            assets: vec![GitHubAssetDto {
                name: EXE_ASSET_NAME.to_string(),
                browser_download_url: format!("http://127.0.0.1/download/{}.exe", tag),
                size: asset_size,
            }],
        }
    }

    #[test]
    fn test_semver_parsing_and_multi_release_aggregation() {
        assert_eq!(parse_semver("v0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_semver("0.3.1"), Some((0, 3, 1)));
        assert!(is_newer_version("v0.3.0", "0.2.0"));
        assert!(!is_newer_version("v0.2.0", "0.2.0"));
        assert!(!is_newer_version("v0.1.9", "0.2.0"));

        let releases = vec![
            sample_release("v0.2.0", "base", false, false, 100),
            sample_release("v0.3.0", "notes 0.3", false, false, 200),
            sample_release("v0.5.0-rc.1", "prerelease", false, true, 300),
            sample_release("v0.4.0", "notes 0.4", false, false, 250),
            sample_release("v0.5.0", "draft", true, false, 400),
        ];

        let res = select_newer_releases("0.2.0", releases);
        assert!(res.has_update);
        assert_eq!(res.current_version, "v0.2.0");
        assert_eq!(res.latest_version, "v0.4.0");
        assert_eq!(res.asset_size, Some(250));
        assert_eq!(res.releases.len(), 2);
        assert_eq!(res.releases[0].version, "v0.4.0");
        assert_eq!(res.releases[0].body, "notes 0.4");
        assert_eq!(res.releases[1].version, "v0.3.0");
        assert_eq!(res.releases[1].body, "notes 0.3");
    }

    #[test]
    fn test_mock_http_feed_download_validation_and_swap() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let fake_exe_bytes: Vec<u8> = b"MZ\x90\x00FAKE_NEW_EXECUTABLE_BINARY_CONTENT".to_vec();
        let bad_bytes: Vec<u8> = b"<html>error</html>".to_vec();

        let feed_json = serde_json::to_string(&vec![GitHubReleaseDto {
            tag_name: "v0.3.0".to_string(),
            name: Some("YMPP Comics Library v0.3.0".to_string()),
            body: Some("## Auto Updater\n- Added update modal".to_string()),
            draft: false,
            prerelease: false,
            published_at: Some("2026-11-15T01:00:00Z".to_string()),
            html_url: format!("http://{}/release/v0.3.0", addr),
            assets: vec![GitHubAssetDto {
                name: EXE_ASSET_NAME.to_string(),
                browser_download_url: format!("http://{}/exe", addr),
                size: fake_exe_bytes.len() as u64,
            }],
        }])
        .unwrap();

        let exe_clone = fake_exe_bytes.clone();
        let bad_clone = bad_bytes.clone();
        let server_task = tokio::spawn(async move {
            // Serve 3 sequential requests: /releases, /exe, /bad
            for _ in 0..3 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap();
                let req = String::from_utf8_lossy(&buf[..n]);

                let (content_type, body): (&str, Vec<u8>) = if req.contains("GET /releases") {
                    ("application/json", feed_json.as_bytes().to_vec())
                } else if req.contains("GET /bad") {
                    ("text/html", bad_clone.clone())
                } else {
                    ("application/octet-stream", exe_clone.clone())
                };

                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content_type,
                    body.len()
                );
                socket.write_all(header.as_bytes()).await.unwrap();
                socket.write_all(&body).await.unwrap();
            }
        });

        let feed_url = format!("http://{}/releases", addr);
        let check = fetch_releases_from_url(&feed_url, "0.2.0").await.unwrap();
        assert!(check.has_update);
        assert_eq!(check.latest_version, "v0.3.0");
        assert_eq!(check.releases.len(), 1);

        let temp_dir = std::env::temp_dir().join(format!(
            "ympp_updater_test_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let current_exe = temp_dir.join(EXE_ASSET_NAME);
        let new_exe = temp_dir.join(format!("{}.new", EXE_ASSET_NAME));
        std::fs::write(&current_exe, b"MZOLD_BINARY").unwrap();

        // 1. Valid exe download + swap
        let dl_size = download_update_to_path(
            check.download_url.as_deref().unwrap(),
            &new_exe,
            check.asset_size,
            None,
        )
        .await
        .unwrap();
        assert_eq!(dl_size, fake_exe_bytes.len() as u64);

        let backup = swap_executables_with_backup(&current_exe, &new_exe).unwrap();
        assert_eq!(std::fs::read(&current_exe).unwrap(), fake_exe_bytes);
        assert_eq!(std::fs::read(&backup).unwrap(), b"MZOLD_BINARY");

        // 2. Non-MZ file rejection preserves existing binary
        let bad_url = format!("http://{}/bad", addr);
        let bad_res = download_update_to_path(&bad_url, &new_exe, None, None).await;
        assert!(bad_res.is_err());
        assert!(!new_exe.exists());

        cleanup_stale_update_files(&temp_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = server_task.await;
        });
    }
}
