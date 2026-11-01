use std::io::Write;
use std::net::TcpListener;
use std::thread;

use serde_json::json;

use super::*;
use crate::error::ErrorCode;

fn mock_check(current: &str, status: u16, body: &str) -> (Result<UpdateCheck, AppError>, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/releases/latest", listener.local_addr().unwrap());
    let body = body.to_owned();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        while !request.windows(4).any(|part| part == b"\r\n\r\n") {
            let mut buffer = [0; 1024];
            let count = stream.read(&mut buffer).unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&buffer[..count]);
        }
        let _ = write!(
            stream,
            "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        String::from_utf8(request).unwrap()
    });
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let result = fetch_update(&client, current, &url);
    (result, server.join().unwrap())
}

fn release(tag: &str) -> String {
    json!({"tag_name": tag, "draft": false, "prerelease": false}).to_string()
}

#[test]
fn stable_versions_compare_components_and_reject_malformed_tags() {
    assert!(stable_version("v0.10.0").unwrap() > stable_version("0.9.99").unwrap());
    assert!(stable_version("1.0.0").unwrap() > stable_version("0.99.99").unwrap());
    assert_eq!(stable_version("v1.2.3").unwrap(), [1, 2, 3]);
    assert_eq!(stable_version("4294967295.0.0").unwrap(), [u32::MAX, 0, 0]);
    for tag in [
        "",
        "v",
        "1.2",
        "1.2.3.4",
        "V1.2.3",
        "vv1.2.3",
        "01.2.3",
        "1.02.3",
        "1.2.03",
        "1.2.3-beta.1",
        "1.2.3+build",
        " 1.2.3",
        "1.2.3\n",
        "+1.2.3",
        "-1.2.3",
        "1..3",
        "１.2.3",
        "4294967296.0.0",
        "1234567890123456789012345678901234.0.0",
    ] {
        assert_eq!(
            stable_version(tag).unwrap_err().code,
            ErrorCode::ConfigurationFailed,
            "{tag:?}"
        );
    }
}

#[test]
fn mocked_github_success_preserves_wire_shape_and_never_offers_a_downgrade() {
    for (current, tag, latest, available) in [
        ("0.9.0", "v0.10.0", "0.10.0", true),
        ("0.10.0", "0.9.0", "0.9.0", false),
        ("0.1.0", "v0.1.0", "0.1.0", false),
        ("0.1.0", "1.0.0", "1.0.0", true),
    ] {
        let (result, request) = mock_check(current, 200, &release(tag));
        assert_eq!(
            serde_json::to_value(result.unwrap()).unwrap(),
            json!({
                "currentVersion": current,
                "latestVersion": latest,
                "updateAvailable": available
            })
        );
        let request = request.to_ascii_lowercase();
        assert!(request.starts_with("get /releases/latest http/1.1\r\n"));
        assert!(request.contains(&format!("user-agent: opencode-mom/{current} ")));
        assert!(request.contains("accept: application/vnd.github+json\r\n"));
        assert!(request.contains("x-github-api-version: 2022-11-28\r\n"));
        assert!(!request.contains("authorization:"));
        assert!(!request.contains("cookie:"));
    }
}

#[test]
fn only_http_404_means_no_stable_release() {
    let (result, _) = mock_check("0.1.0", 404, "not JSON");
    assert_eq!(
        serde_json::to_value(result.unwrap()).unwrap(),
        json!({"currentVersion":"0.1.0", "latestVersion":null, "updateAvailable":false})
    );
    for status in [301, 403, 429, 500] {
        let (result, _) = mock_check("0.1.0", status, "failure");
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        if status == 403 || status == 429 {
            assert!(error.message.contains("rate limited"));
        }
    }
}

#[test]
fn malformed_unstable_and_oversized_release_responses_are_errors() {
    for body in [
        "not JSON".to_owned(),
        "null".to_owned(),
        "{}".to_owned(),
        json!({"tag_name": 123, "draft":false, "prerelease":false}).to_string(),
        json!({"tag_name":"v1.0.0", "prerelease":false}).to_string(),
        json!({"tag_name":"v1.0.0", "draft":true, "prerelease":false}).to_string(),
        json!({"tag_name":"v1.0.0", "draft":false, "prerelease":true}).to_string(),
        release("v1.0.0-beta.1"),
        release("latest"),
        " ".repeat(MAX_RESPONSE_BYTES as usize + 1),
    ] {
        let (result, _) = mock_check("0.1.0", 200, &body);
        assert_eq!(result.unwrap_err().code, ErrorCode::ConfigurationFailed);
    }
}

#[test]
fn network_failures_are_not_reported_as_no_update() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/releases/latest", listener.local_addr().unwrap());
    drop(listener);
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    assert_eq!(
        fetch_update(&client, "0.1.0", &url).unwrap_err().code,
        ErrorCode::ConfigurationFailed
    );
}

#[test]
fn project_pages_allow_only_fixed_destinations_without_launching_them() {
    assert_eq!(project_url("repository").unwrap(), REPOSITORY_URL);
    assert_eq!(project_url("releases").unwrap(), RELEASES_URL);
    for page in [
        "",
        "Repository",
        " repository",
        "releases/latest",
        "https://github.com/seho-dev/opencode-mom/",
        "https://example.com",
        "repository; echo injected",
        "--help",
    ] {
        assert_eq!(
            project_command(page).unwrap_err().code,
            ErrorCode::ValidationFailed
        );
        assert_eq!(
            open_project_page(page).unwrap_err().code,
            ErrorCode::ValidationFailed
        );
    }
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    for page in ["repository", "releases"] {
        let command = project_command(page).unwrap();
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args.last().unwrap(), &project_url(page).unwrap());
        #[cfg(target_os = "macos")]
        assert_eq!(command.get_program(), "/usr/bin/open");
        #[cfg(target_os = "windows")]
        {
            assert_eq!(command.get_program(), "rundll32.exe");
            assert_eq!(args[0], "url.dll,FileProtocolHandler");
        }
        #[cfg(target_os = "linux")]
        assert_eq!(command.get_program(), "xdg-open");
    }
}

#[test]
fn app_info_returns_the_supplied_package_version_and_native_platform() {
    let info = app_info("3.2.1".to_owned());
    let expected = match std::env::consts::OS {
        "macos" | "windows" | "linux" => std::env::consts::OS,
        _ => "other",
    };
    assert_eq!(
        serde_json::to_value(info).unwrap(),
        json!({"version":"3.2.1", "platform":expected})
    );
}
