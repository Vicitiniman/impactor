use super::*;
use std::path::PathBuf;

const GENERATED: &str = r#"<?xml version="1.0"?><plist version="1.0"><dict>
<key>application-identifier</key><string>TESTTEAM01.com.example.app</string>
<key>com.apple.developer.team-identifier</key><string>TESTTEAM01</string>
<key>keychain-access-groups</key><array><string>TESTTEAM01.com.example.app</string></array>
<key>get-task-allow</key><true/>
<key>com.apple.developer.icloud-container-environment</key><string>Production</string>
</dict></plist>"#;

// An iCloud-only file, like files used with signers that merge custom entitlements.
const ICLOUD: &str = r#"<?xml version="1.0"?><plist version="1.0"><dict>
<key>com.apple.developer.icloud-container-identifiers</key><array><string>iCloud.com.example.app.TESTTEAM01</string></array>
<key>com.apple.developer.icloud-services</key><array><string>CloudKit</string><string>CloudDocuments</string></array>
<key>com.apple.developer.ubiquity-container-identifiers</key><array><string>iCloud.com.example.app.TESTTEAM01</string></array>
<key>com.apple.developer.ubiquity-kvstore-identifier</key><string>TESTTEAM01.com.example.app.TESTTEAM01</string>
<key>com.apple.developer.icloud-container-environment</key><string>Development</string>
</dict></plist>"#;

struct Fixture(PathBuf);

impl Fixture {
    fn new(data: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "impactor-entitlements-{}.plist",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&path, data).unwrap();
        Self(path)
    }

    fn signer(&self) -> Signer {
        let mut options = SignerOptions::default();
        options.mode = SignerMode::Pem;
        options.embedding.single_profile = true;
        options.custom_entitlements = Some(self.0.clone());
        Signer::new(None, options)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn merged(signer: &Signer, kind: BundleType, generated: &str) -> plist::Dictionary {
    let xml = signer
        .signing_entitlements(&kind, generated.to_string())
        .unwrap();
    plist::from_bytes(xml.as_bytes()).unwrap()
}

#[test]
fn custom_entitlements_icloud_overlay_preserves_signing_identity() {
    let fixture = Fixture::new(ICLOUD.as_bytes());
    let result = merged(&fixture.signer(), BundleType::App, GENERATED);
    let base: plist::Dictionary = plist::from_bytes(GENERATED.as_bytes()).unwrap();
    for key in [
        "application-identifier",
        "com.apple.developer.team-identifier",
        "keychain-access-groups",
        "get-task-allow",
    ] {
        assert_eq!(result[key], base[key]);
    }
    let custom: plist::Dictionary = plist::from_bytes(ICLOUD.as_bytes()).unwrap();
    for (key, value) in custom {
        assert_eq!(result[&key], value);
    }
}

#[test]
fn custom_entitlements_stale_app_and_team_cannot_replace_profile_identity() {
    let custom = ICLOUD.replace("</dict>", "<key>application-identifier</key><string>OLDTEAM001.com.old.app</string><key>com.apple.developer.team-identifier</key><string>OLDTEAM001</string></dict>");
    let fixture = Fixture::new(custom.as_bytes());
    let result = merged(&fixture.signer(), BundleType::App, GENERATED);
    assert_eq!(
        result["application-identifier"].as_string(),
        Some("TESTTEAM01.com.example.app")
    );
    assert_eq!(
        result["com.apple.developer.team-identifier"].as_string(),
        Some("TESTTEAM01")
    );
}

#[test]
fn custom_entitlements_extension_keeps_its_own_identity() {
    let fixture = Fixture::new(ICLOUD.as_bytes());
    let extension = GENERATED.replace("com.example.app", "com.example.app.extension");
    let result = merged(&fixture.signer(), BundleType::AppExtension, &extension);
    assert_eq!(
        result["application-identifier"].as_string(),
        Some("TESTTEAM01.com.example.app.extension")
    );
}

#[test]
fn custom_entitlements_libraries_do_not_read_or_receive_custom_file() {
    let fixture = Fixture::new(ICLOUD.as_bytes());
    let mut signer = fixture.signer();
    signer.options.custom_entitlements = Some(fixture.0.join("missing"));
    for kind in [
        BundleType::Framework,
        BundleType::Dylib,
        BundleType::Unknown,
    ] {
        assert_eq!(
            signer
                .signing_entitlements(&kind, GENERATED.into())
                .unwrap(),
            GENERATED
        );
    }
}

#[test]
fn custom_entitlements_disabled_without_single_profile() {
    let fixture = Fixture::new(ICLOUD.as_bytes());
    let mut signer = fixture.signer();
    signer.options.embedding.single_profile = false;
    assert_eq!(
        signer
            .signing_entitlements(&BundleType::App, GENERATED.into())
            .unwrap(),
        GENERATED
    );
}

#[test]
fn custom_entitlements_adhoc_signing_is_unchanged() {
    let fixture = Fixture::new(ICLOUD.as_bytes());
    let mut signer = fixture.signer();
    signer.options.mode = SignerMode::Adhoc;
    assert_eq!(
        signer
            .signing_entitlements(&BundleType::App, GENERATED.into())
            .unwrap(),
        GENERATED
    );
}

#[test]
fn custom_entitlements_no_file_keeps_generated_xml() {
    let fixture = Fixture::new(ICLOUD.as_bytes());
    let mut signer = fixture.signer();
    signer.options.custom_entitlements = None;
    assert_eq!(
        signer
            .signing_entitlements(&BundleType::App, GENERATED.into())
            .unwrap(),
        GENERATED
    );
}

#[test]
fn custom_entitlements_binary_plist_is_supported() {
    let value = Value::from_reader_xml(ICLOUD.as_bytes()).unwrap();
    let mut binary = Vec::new();
    value.to_writer_binary(&mut binary).unwrap();
    let fixture = Fixture::new(&binary);
    let result = merged(&fixture.signer(), BundleType::App, GENERATED);
    assert_eq!(
        result["com.apple.developer.icloud-container-environment"].as_string(),
        Some("Development")
    );
}

#[test]
fn custom_entitlements_invalid_input_reports_an_error() {
    for input in ["invalid plist", "<plist version=\"1.0\"><array/></plist>"] {
        let fixture = Fixture::new(input.as_bytes());
        assert!(
            fixture
                .signer()
                .signing_entitlements(&BundleType::App, GENERATED.into())
                .is_err()
        );
    }
}

#[test]
fn custom_entitlements_missing_application_identifier_fails_before_signing() {
    let fixture = Fixture::new(ICLOUD.as_bytes());
    let result = fixture.signer().signing_entitlements(
        &BundleType::App,
        "<plist version=\"1.0\"><dict/></plist>".into(),
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("application-identifier")
    );
}
