use crate::errors::{LauncherError, LauncherResult};
use crate::models::{ModChannel, Platform};
use serde::Deserialize;

const STFC_MOD_RELEASES_URL: &str = "https://api.github.com/repos/netniV/stfc-mod/releases";

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub prerelease: bool,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedModAsset {
    pub version: String,
    pub archive_name: String,
    pub archive_url: String,
    pub checksum_url: String,
}

fn expected_archive_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "stfc-community-mod-windows-x64.tar.zst",
        Platform::MacOs => "stfc-community-mod-macos-universal.tar.zst",
    }
}

pub fn select_release_asset(
    releases: &[GitHubRelease],
    platform: Platform,
    channel: ModChannel,
) -> LauncherResult<SelectedModAsset> {
    let archive_name = expected_archive_name(platform);
    let checksum_name = format!("{archive_name}.sha256");

    for release in releases {
        if release.prerelease != (channel == ModChannel::Prerelease) {
            continue;
        }
        let archive = release
            .assets
            .iter()
            .find(|asset| asset.name == archive_name);
        let checksum = release
            .assets
            .iter()
            .find(|asset| asset.name == checksum_name);
        if let (Some(archive), Some(checksum)) = (archive, checksum) {
            return Ok(SelectedModAsset {
                version: release.tag_name.clone(),
                archive_name: archive.name.clone(),
                archive_url: archive.browser_download_url.clone(),
                checksum_url: checksum.browser_download_url.clone(),
            });
        }
    }

    Err(LauncherError::InvalidData {
        context: "selecting mod release asset".into(),
        message: format!("no {archive_name} asset with checksum found for {channel:?}"),
    })
}

pub async fn fetch_releases(client: &reqwest::Client) -> LauncherResult<Vec<GitHubRelease>> {
    fetch_releases_from(client, STFC_MOD_RELEASES_URL).await
}

async fn fetch_releases_from(
    client: &reqwest::Client,
    url: &str,
) -> LauncherResult<Vec<GitHubRelease>> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "stfc-mod-launcher")
        .send()
        .await
        .map_err(|source| crate::errors::LauncherError::Network {
            context: "fetching STFC mod releases".into(),
            source,
        })?
        .error_for_status()
        .map_err(|source| crate::errors::LauncherError::Network {
            context: "checking STFC mod releases response".into(),
            source,
        })?;
    response
        .json()
        .await
        .map_err(|source| crate::errors::LauncherError::Network {
            context: "parsing STFC mod releases response".into(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    struct TestServer {
        url: String,
        request: mpsc::Receiver<String>,
        handle: thread::JoinHandle<()>,
    }

    impl TestServer {
        fn respond_once(path: &str, status: &str, body: &str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
            let addr = listener.local_addr().expect("test server addr");
            let url = format!("http://{addr}{path}");
            let body = body.to_string();
            let status = status.to_string();
            let (request_tx, request_rx) = mpsc::channel();

            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
                let mut request = String::new();

                loop {
                    let mut line = String::new();
                    let bytes = reader.read_line(&mut line).expect("read request");
                    if bytes == 0 || line == "\r\n" {
                        break;
                    }
                    request.push_str(&line);
                }

                request_tx.send(request).expect("send request");
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            });

            Self {
                url,
                request: request_rx,
                handle,
            }
        }

        fn request(self) -> String {
            let request = self.request.recv().expect("server request");
            self.handle.join().expect("server finished");
            request
        }
    }

    fn asset(name: &str, url: &str) -> GitHubAsset {
        GitHubAsset {
            name: name.into(),
            browser_download_url: url.into(),
        }
    }

    fn release(tag_name: &str, prerelease: bool, assets: Vec<GitHubAsset>) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag_name.into(),
            prerelease,
            assets,
        }
    }

    fn assert_invalid_data(result: LauncherResult<SelectedModAsset>) {
        match result {
            Err(LauncherError::InvalidData { context, message }) => {
                assert_eq!(context, "selecting mod release asset");
                assert!(
                    message.contains("stfc-community-mod-windows-x64.tar.zst"),
                    "{message}"
                );
            }
            other => panic!("expected invalid data error, got {other:?}"),
        }
    }

    #[test]
    fn stable_channel_skips_prereleases() {
        let releases: Vec<GitHubRelease> =
            serde_json::from_str(include_str!("../tests/fixtures/github_releases.json"))
                .expect("fixture");

        let selected = select_release_asset(
            &releases,
            crate::models::Platform::Windows,
            crate::models::ModChannel::Stable,
        )
        .expect("stable asset");

        assert_eq!(selected.version, "v1.2.0");
        assert_eq!(selected.archive_url, "https://example.test/win.tar.zst");
    }

    #[test]
    fn prerelease_channel_can_select_prerelease() {
        let releases: Vec<GitHubRelease> =
            serde_json::from_str(include_str!("../tests/fixtures/github_releases.json"))
                .expect("fixture");

        let selected = select_release_asset(
            &releases,
            crate::models::Platform::Windows,
            crate::models::ModChannel::Prerelease,
        )
        .expect("prerelease asset");

        assert_eq!(selected.version, "v1.3.0-beta.1");
        assert_eq!(
            selected.checksum_url,
            "https://example.test/win-beta.sha256"
        );
    }

    #[test]
    fn prerelease_channel_ignores_stable_releases() {
        let releases = vec![
            release(
                "v1.4.0",
                false,
                vec![
                    asset(
                        "stfc-community-mod-windows-x64.tar.zst",
                        "https://example.test/win-stable.tar.zst",
                    ),
                    asset(
                        "stfc-community-mod-windows-x64.tar.zst.sha256",
                        "https://example.test/win-stable.sha256",
                    ),
                ],
            ),
            release(
                "v1.5.0-beta.1",
                true,
                vec![
                    asset(
                        "stfc-community-mod-windows-x64.tar.zst",
                        "https://example.test/win-prerelease.tar.zst",
                    ),
                    asset(
                        "stfc-community-mod-windows-x64.tar.zst.sha256",
                        "https://example.test/win-prerelease.sha256",
                    ),
                ],
            ),
        ];

        let selected = select_release_asset(&releases, Platform::Windows, ModChannel::Prerelease)
            .expect("prerelease asset");

        assert_eq!(selected.version, "v1.5.0-beta.1");
        assert_eq!(
            selected.archive_url,
            "https://example.test/win-prerelease.tar.zst"
        );
    }

    #[test]
    fn prerelease_channel_without_prereleases_returns_invalid_data() {
        let releases = vec![release(
            "v1.4.0",
            false,
            vec![
                asset(
                    "stfc-community-mod-windows-x64.tar.zst",
                    "https://example.test/win-stable.tar.zst",
                ),
                asset(
                    "stfc-community-mod-windows-x64.tar.zst.sha256",
                    "https://example.test/win-stable.sha256",
                ),
            ],
        )];

        let selected = select_release_asset(&releases, Platform::Windows, ModChannel::Prerelease);

        assert_invalid_data(selected);
    }

    #[test]
    fn archive_without_checksum_is_not_accepted() {
        let releases = vec![release(
            "v1.4.0",
            false,
            vec![asset(
                "stfc-community-mod-windows-x64.tar.zst",
                "https://example.test/win-incomplete.tar.zst",
            )],
        )];

        let selected = select_release_asset(&releases, Platform::Windows, ModChannel::Stable);

        assert_invalid_data(selected);
    }

    #[test]
    fn checksum_without_archive_is_not_accepted() {
        let releases = vec![release(
            "v1.4.0",
            false,
            vec![asset(
                "stfc-community-mod-windows-x64.tar.zst.sha256",
                "https://example.test/win-incomplete.sha256",
            )],
        )];

        let selected = select_release_asset(&releases, Platform::Windows, ModChannel::Stable);

        assert_invalid_data(selected);
    }

    #[test]
    fn incomplete_newer_release_is_skipped_for_next_complete_release() {
        let releases = vec![
            release(
                "v1.4.0",
                false,
                vec![asset(
                    "stfc-community-mod-windows-x64.tar.zst",
                    "https://example.test/win-incomplete.tar.zst",
                )],
            ),
            release(
                "v1.3.0",
                false,
                vec![
                    asset(
                        "stfc-community-mod-windows-x64.tar.zst",
                        "https://example.test/win-complete.tar.zst",
                    ),
                    asset(
                        "stfc-community-mod-windows-x64.tar.zst.sha256",
                        "https://example.test/win-complete.sha256",
                    ),
                ],
            ),
        ];

        let selected =
            select_release_asset(&releases, Platform::Windows, ModChannel::Stable).expect("asset");

        assert_eq!(selected.version, "v1.3.0");
        assert_eq!(
            selected.archive_url,
            "https://example.test/win-complete.tar.zst"
        );
        assert_eq!(
            selected.checksum_url,
            "https://example.test/win-complete.sha256"
        );
    }

    #[test]
    fn stable_channel_selects_macos_asset_pair() {
        let releases: Vec<GitHubRelease> =
            serde_json::from_str(include_str!("../tests/fixtures/github_releases.json"))
                .expect("fixture");

        let selected = select_release_asset(&releases, Platform::MacOs, ModChannel::Stable)
            .expect("macos stable asset");

        assert_eq!(selected.version, "v1.2.0");
        assert_eq!(
            selected.archive_name,
            "stfc-community-mod-macos-universal.tar.zst"
        );
        assert_eq!(selected.archive_url, "https://example.test/mac.tar.zst");
        assert_eq!(selected.checksum_url, "https://example.test/mac.sha256");
    }

    #[test]
    fn fetch_releases_sends_expected_path_and_user_agent() {
        let server = TestServer::respond_once(
            "/repos/netniV/stfc-mod/releases",
            "200 OK",
            include_str!("../tests/fixtures/github_releases.json"),
        );
        let client = reqwest::Client::new();

        let releases = tauri::async_runtime::block_on(fetch_releases_from(&client, &server.url))
            .expect("releases");
        let request = server.request();

        assert_eq!(releases.len(), 2);
        assert!(request.starts_with("GET /repos/netniV/stfc-mod/releases HTTP/1.1"));
        assert!(request.contains("user-agent: stfc-mod-launcher\r\n"));
    }

    #[test]
    fn fetch_releases_maps_non_success_status_to_network_error() {
        let server = TestServer::respond_once("/releases", "500 Internal Server Error", "[]");
        let client = reqwest::Client::new();

        let result = tauri::async_runtime::block_on(fetch_releases_from(&client, &server.url));
        server.request();

        match result {
            Err(LauncherError::Network { context, .. }) => {
                assert_eq!(context, "checking STFC mod releases response");
            }
            other => panic!("expected status-checking network error, got {other:?}"),
        }
    }

    #[test]
    fn fetch_releases_maps_invalid_json_to_network_error() {
        let server = TestServer::respond_once("/releases", "200 OK", "not json");
        let client = reqwest::Client::new();

        let result = tauri::async_runtime::block_on(fetch_releases_from(&client, &server.url));
        server.request();

        match result {
            Err(LauncherError::Network { context, .. }) => {
                assert_eq!(context, "parsing STFC mod releases response");
            }
            other => panic!("expected parsing network error, got {other:?}"),
        }
    }

    #[test]
    fn fetch_releases_parses_release_json() {
        let server = TestServer::respond_once(
            "/releases",
            "200 OK",
            include_str!("../tests/fixtures/github_releases.json"),
        );
        let client = reqwest::Client::new();

        let releases = tauri::async_runtime::block_on(fetch_releases_from(&client, &server.url))
            .expect("releases");
        server.request();

        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag_name, "v1.3.0-beta.1");
        assert!(releases[0].prerelease);
        assert_eq!(releases[1].tag_name, "v1.2.0");
    }
}
