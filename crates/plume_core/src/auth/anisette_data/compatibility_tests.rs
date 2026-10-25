use super::*;
use crate::AnisetteMode;

const CLIENT_INFO: &str = "<Mac15,7> <macOS;27.0;26A> <ServerAuthKit/1>";
const USER_AGENT: &str = "akd/1.0 CFNetwork/3860.1.1 Darwin/26.0.0";

fn data(mode: AnisetteMode) -> AnisetteData {
    AnisetteData {
        base_headers: HashMap::from([
            ("X-Mme-Client-Info".into(), CLIENT_INFO.into()),
            ("User-Agent".into(), USER_AGENT.into()),
            ("X-Apple-I-MD".into(), "CaseSensitiveOTP".into()),
        ]),
        generated_at: SystemTime::now(),
        config: AnisetteConfiguration::default().set_mode(mode),
    }
}

#[test]
fn compatibility_identity_is_preserved_for_all_optional_modes() {
    for mode in AnisetteMode::ALL
        .into_iter()
        .filter(|m| *m != AnisetteMode::Standard)
    {
        let data = data(mode);
        assert_eq!(data.get_header("x-MME-client-INFO").unwrap(), CLIENT_INFO);
        assert_eq!(data.get_header("x-apple-i-md").unwrap(), "CaseSensitiveOTP");
        assert_eq!(data.user_agent("Xcode"), USER_AGENT);
        let headers = data.generate_headers(false, true, true);
        assert_eq!(headers["X-Mme-Client-Info"], CLIENT_INFO);
        assert_eq!(headers["User-Agent"], USER_AGENT);
        assert_eq!(headers["X-Apple-App-Info"], "com.apple.gs.xcode.auth");
        // Refresh retains the chosen mode and therefore its server and cache.
        assert_eq!(data.config.clone().mode(), mode);
    }
}

#[test]
fn compatibility_standard_keeps_existing_identity_and_user_agent_rules() {
    let mut data = data(AnisetteMode::Standard);
    data.base_headers.remove("User-Agent");
    assert_eq!(data.user_agent("Xcode"), "Xcode");
    assert_eq!(
        data.get_header("x-mme-client-info").unwrap(),
        "<mac15,7> <macos;27.0;26a> <com.apple.authkit/1 (com.apple.akd/1.0)>"
    );
    assert_eq!(data.get_header("x-apple-i-md").unwrap(), "casesensitiveotp");
}

#[test]
fn compatibility_cpd_omits_http_identity_headers() {
    let data = data(AnisetteMode::Iloader);
    let cpd = data.to_plist(true, false, false);
    assert!(!cpd.contains_key("User-Agent"));
    assert!(!cpd.contains_key("X-Mme-Client-Info"));
    assert_eq!(cpd["X-Apple-I-MD"].as_string(), Some("CaseSensitiveOTP"));
    assert_eq!(cpd["bootstrap"].as_string(), Some("true"));
}
