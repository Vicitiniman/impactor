use super::*;
use std::io::{BufRead, BufReader, Read, Write as IoWrite};
use std::net::TcpListener;
use std::thread::JoinHandle;
use std::time::Duration;

const IDENTITY: &str = r#"{"client_info":"<Mac15,7> <macOS;27.0;26A> <AuthKit/1>","user_agent":"akd/1.0 CFNetwork/3860.1.1 Darwin/26.0.0"}"#;
const HEADERS: &str = r#"{"result":"Headers","X-Apple-I-MD-M":"MachineCaseSensitive","X-Apple-I-MD":"OtpCaseSensitive","X-Apple-I-MD-RINFO":"17106176"}"#;

// Keep loopback fixtures sequential, including when the suite runs on Windows.
static HTTP_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn server(responses: Vec<(&'static str, u16, &'static str)>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        for (expected_path, status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line.split_whitespace().nth(1), Some(expected_path));
            let mut length = 0;
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some((key, value)) = line.split_once(':') {
                    if key.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().unwrap();
                    }
                }
            }
            reader.read_exact(&mut vec![0; length]).unwrap();
            drop(reader);
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.shutdown(std::net::Shutdown::Write).unwrap();
            // Let the client consume the response before closing the socket on Windows.
            let _ = stream.read(&mut [0; 1]);
        }
    });
    (url, handle)
}

fn provisioned_state() -> AnisetteState {
    AnisetteState {
        keychain_identifier: [7; 16],
        adi_pb: Some(vec![1, 2, 3]),
    }
}

#[tokio::test]
async fn compatibility_remote_uses_server_identity_and_cached_state() {
    let _lock = HTTP_TEST_LOCK.lock().unwrap();
    let (url, server) = server(vec![
        ("/v3/client_info", 200, IDENTITY),
        ("/v3/get_headers", 200, HEADERS),
    ]);
    let path = std::env::temp_dir().join(format!("impactor-anisette-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    plist::to_file_xml(path.join("state.plist"), &provisioned_state()).unwrap();
    let mut provider =
        RemoteAnisetteProviderV3::new(url, path.clone(), "0".into()).with_server_identity(true);
    let headers = provider.get_authentication_headers().await.unwrap();
    assert_eq!(
        headers["X-Mme-Client-Info"],
        "<Mac15,7> <macOS;27.0;26A> <AuthKit/1>"
    );
    assert_eq!(
        headers["User-Agent"],
        "akd/1.0 CFNetwork/3860.1.1 Darwin/26.0.0"
    );
    assert_eq!(headers["X-Apple-I-MD"], "OtpCaseSensitive");
    assert_eq!(
        provider.state.as_ref().unwrap().keychain_identifier,
        [7; 16]
    );
    server.join().unwrap();
    fs::remove_dir_all(path).unwrap();
}

#[tokio::test]
async fn compatibility_standard_does_not_override_user_agent() {
    let _lock = HTTP_TEST_LOCK.lock().unwrap();
    let (url, server) = server(vec![
        ("/v3/client_info", 200, IDENTITY),
        ("/v3/get_headers", 200, HEADERS),
    ]);
    let client = AnisetteClient::new(url).await.unwrap();
    let headers = client
        .get_headers(&provisioned_state())
        .await
        .unwrap()
        .get_headers("0".into());
    assert!(!headers.contains_key("User-Agent"));
    server.join().unwrap();
}

#[tokio::test]
async fn compatibility_invalid_server_identity_is_rejected() {
    let _lock = HTTP_TEST_LOCK.lock().unwrap();
    let (url, server) = server(vec![(
        "/v3/client_info",
        200,
        r#"{"client_info":"","user_agent":"bad\nheader"}"#,
    )]);
    assert!(matches!(
        AnisetteClient::new_with_server_identity(url, true).await,
        Err(AnisetteError::RemoteServer(_))
    ));
    server.join().unwrap();
}

#[tokio::test]
async fn compatibility_remote_errors_are_returned_without_panicking() {
    let _lock = HTTP_TEST_LOCK.lock().unwrap();
    for (status, body) in [
        (
            200,
            r#"{"result":"GetHeadersError","message":"server unavailable"}"#,
        ),
        (503, "unavailable"),
    ] {
        let (url, server) = server(vec![
            ("/v3/client_info", 200, IDENTITY),
            ("/v3/get_headers", status, body),
        ]);
        let client = AnisetteClient::new_with_server_identity(url, true)
            .await
            .unwrap();
        assert!(client.get_headers(&provisioned_state()).await.is_err());
        server.join().unwrap();
    }
}

#[tokio::test]
async fn compatibility_minus_45061_signals_reprovisioning() {
    let _lock = HTTP_TEST_LOCK.lock().unwrap();
    let (url, server) = server(vec![
        ("/v3/client_info", 200, IDENTITY),
        (
            "/v3/get_headers",
            200,
            r#"{"result":"GetHeadersError","message":"ADI error -45061"}"#,
        ),
    ]);
    let client = AnisetteClient::new_with_server_identity(url, true)
        .await
        .unwrap();
    assert!(matches!(
        client.get_headers(&provisioned_state()).await,
        Err(AnisetteError::AnisetteNotProvisioned)
    ));
    server.join().unwrap();
}

#[test]
fn compatibility_reprovisioning_resets_only_optional_mode_identity() {
    let mut state = provisioned_state();
    state.invalidate(false);
    assert!(!state.is_provisioned());
    assert_eq!(state.keychain_identifier, [7; 16]);
    state.invalidate(true);
    assert!(!state.is_provisioned());
    assert_ne!(state.keychain_identifier, [7; 16]);
}
