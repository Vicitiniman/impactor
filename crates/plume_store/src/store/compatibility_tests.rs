use super::*;

const OLD_STORE: &str = r#"{"selected_account":null,"accounts":{},"locale":"en"}"#;

fn directory() -> PathBuf {
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("impactor-store-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn compatibility_old_settings_default_to_standard() {
    let store: AccountStore = serde_json::from_str(OLD_STORE).unwrap();
    assert_eq!(store.anisette_mode(), AnisetteMode::Standard);
    assert_eq!(store.locale(), Some("en"));
}

#[test]
fn compatibility_mode_persists_across_reload_and_account_changes() {
    let directory = directory();
    let path = Some(directory.join("accounts.json"));
    let mut store = AccountStore::load_sync(&path).unwrap();
    store
        .set_anisette_mode_sync(AnisetteMode::IloaderStikstore)
        .unwrap();
    let account = |email: &str| {
        GsaAccount::new(
            email.into(),
            "Name".into(),
            "id".into(),
            "token".into(),
            "team".into(),
        )
    };
    store
        .accounts_add_sync(account("first@example.test"))
        .unwrap();
    store
        .accounts_add_sync(account("second@example.test"))
        .unwrap();
    store.account_select_sync("first@example.test").unwrap();
    store.accounts_remove_sync("second@example.test").unwrap();
    let restored = AccountStore::load_sync(&path).unwrap();
    assert_eq!(restored.anisette_mode(), AnisetteMode::IloaderStikstore);
    assert_eq!(
        restored.selected_account().unwrap().email(),
        "first@example.test"
    );
    let config = restored.anisette_configuration(directory.clone());
    assert_eq!(config.remote_url(), "https://ani3.stikstore.app");
    assert_eq!(
        config.remote_configuration_path(),
        directory.join("iloader-stikstore")
    );
    store
        .set_anisette_mode_sync(AnisetteMode::Standard)
        .unwrap();
    let config = AccountStore::load_sync(&path)
        .unwrap()
        .anisette_configuration(directory.clone());
    assert_eq!(config.mode(), AnisetteMode::Standard);
    assert_eq!(config.remote_configuration_path(), directory);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn compatibility_failed_save_rolls_back_mode() {
    let directory = directory();
    let mut store = AccountStore::default();
    // Writing settings to a directory fails without relying on filesystem permissions.
    store.path = Some(directory.clone());
    assert!(store.set_anisette_mode_sync(AnisetteMode::Iloader).is_err());
    assert_eq!(store.anisette_mode(), AnisetteMode::Standard);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn compatibility_refresh_save_keeps_a_mode_changed_while_job_was_running() {
    let directory = directory();
    let path = Some(directory.join("accounts.json"));
    let mut settings = AccountStore::load_sync(&path).unwrap();
    let mut running_job = settings.clone();
    settings
        .set_anisette_mode_sync(AnisetteMode::IloaderSidestore)
        .unwrap();
    running_job.reload_sync().unwrap();
    running_job
        .add_or_update_refresh_device_sync(RefreshDevice {
            udid: "test-device".into(),
            name: "Test".into(),
            account: "first@example.test".into(),
            apps: Vec::new(),
            is_mac: false,
        })
        .unwrap();
    let restored = AccountStore::load_sync(&path).unwrap();
    assert_eq!(restored.anisette_mode(), AnisetteMode::IloaderSidestore);
    assert!(restored.get_refresh_device("test-device").is_some());
    std::fs::remove_dir_all(directory).unwrap();
}
