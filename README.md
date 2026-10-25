# Impactor

## Custom entitlements

With **Advanced > Only Register Main Bundle** enabled, a custom entitlements plist adds to or overrides the generated signing entitlements for apps and app extensions. A file containing only iCloud settings is sufficient; it does not need to repeat `application-identifier`, the team identifier, or existing keychain groups. The app and team identity from the selected provisioning profile take precedence over values in the custom file. Frameworks and libraries do not receive the custom entitlements. Both XML and binary plist dictionaries are supported.

The provisioning profile must still authorize the requested capabilities and iCloud containers. Importing an entitlements file cannot grant access to a container that the signing account/profile does not allow. See [Apple's entitlements troubleshooting guidance](https://developer.apple.com/library/archive/technotes/tn2415/).

Regression checks: `cargo test --locked -p plume_utils --lib custom_entitlements`.

## Optional iLoader compatibility mode

Open **Settings > Authentication mode** and choose **iLoader compatibility - Jkcoxson** to try iLoader's current remote authentication approach. SideStore and StikStore are available as alternatives. The choice is saved and applies to the next login, signing operation, team lookup, certificate export, and automatic refresh. Operations already running finish with the configuration they started with.

**Standard (default)** preserves Impactor's existing behavior: try local anisette first, then fall back to its existing remote server. Existing settings files without a mode continue to use Standard. Selecting Standard again restores that path and its original cache.

The command line shares the setting with the desktop app:

```sh
plumesign account login --username you@example.com --anisette-mode iloader
plumesign account login --username you@example.com --anisette-mode iloader-sidestore
plumesign account login --username you@example.com --anisette-mode iloader-stikstore
plumesign account login --username you@example.com --anisette-mode standard
```

The CLI saves an explicitly selected mode after a successful login. Omitting the flag uses the saved mode, or Standard for a new installation. The password and verification code prompts work as before.

### What this changes

The optional mode always uses remote anisette, even when local libraries are present. It fetches both `client_info` and `user_agent` from the selected server's `/v3/client_info` endpoint and preserves their values and case through login, app-token requests, two-factor authentication, and developer requests. It keeps HTTP identity headers out of GrandSlam's `cpd` dictionary. Each server has its own provisioning cache beneath the Impactor data directory:

| Mode | Remote server | Cache directory |
| --- | --- | --- |
| `iloader` | `https://mac-ani.jkcoxson.com` | `iloader-jkcoxson` |
| `iloader-sidestore` | `https://new.ani.sidestore.io` | `iloader-sidestore` |
| `iloader-stikstore` | `https://ani3.stikstore.app` | `iloader-stikstore` |

The existing one-time recovery from anisette error `-45061` also creates a fresh device identity in compatibility mode, matching iLoader's dependency. Other remote errors are returned instead of panicking. **Reset ADI** clears the selected mode's remote cache; it does not remove accounts, certificates, or other modes' caches.

### Upstream evidence and limits

iLoader initially addressed `-80045` in [v2.3.7](https://github.com/nab138/iloader/releases/tag/v2.3.7) by using Android client information in its `isideload` dependency. That workaround broke some SMS verification flows. Its subsequent [v2.3.8 fix](https://github.com/nab138/iloader/releases/tag/v2.3.8) switched server families and [fetched the matching client information from the server](https://github.com/nab138/isideload/commit/d424b1786664f1b9b6b98dd70d5826b161c12f45). [Error -45061 recovery](https://github.com/nab138/isideload/commit/8ca39b6203263e539408986e546c42e7c1cc3699) also resets the provisioning identity. [iLoader v2.4.0's server selection](https://github.com/nab138/iloader/commit/902597564315442f00ab299dcc758ee0e13b8fa8) favors Jkcoxson's Mac server and lists the other two Mac servers above. This option follows that later approach rather than hardcoding the earlier Android identity.

Impactor already used Jkcoxson's Mac server as its remote fallback before this change. The added behavior is the explicit remote selection, unmodified matching identity, consistent session configuration, and separate caches.

These are third-party services providing authentication data; your choice determines which service handles anisette provisioning. Impactor continues to send account credentials and verification codes directly to Apple. Availability and Apple's acceptance can change, and this mode cannot guarantee a fix for every account. You may need to sign in again. Trusted-device verification remains available; SMS verification can still fail because of upstream Apple/server restrictions. This does not change signing limits, entitlements, device pairing, or installation protocols.

### Verification

The regression tests use local HTTP fixtures and synthetic settings, without real Apple accounts or live anisette servers:

```sh
cargo test --locked -p omnisette --features remote-anisette-v3 --lib compatibility
cargo test --locked -p plume_core -p plume_store --lib compatibility
cargo test --locked -p plumesign --bin plumesign compatibility
cargo check --locked --workspace --exclude plume_gestalt --all-targets
```

The check above excludes the existing macOS-only `plume_gestalt` helper on non-Apple hosts. The desktop and CLI applications are both checked. On macOS, omit the exclusion to check the entire workspace.

The tests cover default behavior, CLI validation, settings migration and persistence across account changes and running refresh jobs, server/cache separation, exact identity headers, cached state reuse, invalid server responses, and the `-45061` recovery decision. A real Apple login, verification, install, and refresh on a connected iOS device remain necessary to establish end-to-end compatibility.
