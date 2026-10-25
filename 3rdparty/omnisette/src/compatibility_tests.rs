use crate::{AnisetteConfiguration, AnisetteHeaders, AnisetteHeadersProviderType, AnisetteMode};
use std::collections::HashSet;
use std::path::PathBuf;

#[test]
fn compatibility_standard_preserves_existing_configuration() {
    let config = AnisetteConfiguration::default().set_configuration_path(PathBuf::from("data"));
    assert_eq!(config.mode(), AnisetteMode::Standard);
    assert_eq!(config.remote_url(), crate::DEFAULT_ANISETTE_URL_V3);
    assert_eq!(config.remote_configuration_path(), PathBuf::from("data"));
    assert!(!config.mode().uses_server_identity());
}

#[test]
fn compatibility_profiles_force_remote_and_isolate_state() {
    let mut paths = HashSet::new();
    for mode in AnisetteMode::ALL {
        let config = AnisetteConfiguration::default()
            .set_mode(mode)
            .set_configuration_path(PathBuf::from("data"));
        assert!(paths.insert(config.remote_configuration_path()));
        if mode != AnisetteMode::Standard {
            assert_eq!(Some(config.remote_url()), mode.server_url());
            let provider = AnisetteHeaders::get_anisette_headers_provider(config).unwrap();
            assert!(matches!(
                provider.provider_type,
                AnisetteHeadersProviderType::Remote
            ));
        }
    }
}

#[test]
fn compatibility_mode_names_round_trip_and_reject_unknown_values() {
    for (value, mode) in [
        ("standard", AnisetteMode::Standard),
        ("iloader", AnisetteMode::Iloader),
        ("iloader-sidestore", AnisetteMode::IloaderSidestore),
        ("iloader-stikstore", AnisetteMode::IloaderStikstore),
    ] {
        assert_eq!(value.parse::<AnisetteMode>().unwrap(), mode);
        assert_eq!(
            serde_json::to_string(&mode).unwrap(),
            format!("\"{value}\"")
        );
        assert_eq!(
            serde_json::from_str::<AnisetteMode>(&format!("\"{value}\"")).unwrap(),
            mode
        );
    }
    assert!("unknown".parse::<AnisetteMode>().is_err());
    assert!(serde_json::from_str::<AnisetteMode>("\"unknown\"").is_err());
}
