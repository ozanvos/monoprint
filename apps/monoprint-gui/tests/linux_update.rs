//! Runs against a real package only in the Linux packaging qualification step.
#![cfg(target_os = "linux")]

#[test]
#[ignore = "requires an isolated packaged AppImage fixture"]
fn packaged_linux_upgrade() {
    use std::{env, fs, path::PathBuf};
    use velopack::{UpdateCheck, UpdateManager, UpdateOptions, locator::VelopackLocatorConfig};

    let fixture = PathBuf::from(env::var_os("MONOPRINT_UPDATE_FIXTURE").unwrap());
    let source = PathBuf::from(env::var_os("MONOPRINT_PACKAGE_SOURCE").unwrap());
    let locator = VelopackLocatorConfig {
        RootAppDir: fixture.join("Monoprint.AppImage"),
        CurrentBinaryDir: fixture.join("squashfs-root/usr/bin"),
        UpdateExePath: fixture.join("squashfs-root/usr/bin/UpdateNix"),
        ManifestPath: fixture.join("old.version"),
        PackagesDir: fixture.join("packages"),
        IsPortable: true,
    };
    let manager = UpdateManager::new(
        velopack::sources::FileSource::new(source),
        Some(UpdateOptions {
            ExplicitChannel: Some(env::var("MONOPRINT_TEST_UPDATE_CHANNEL").unwrap()),
            ..Default::default()
        }),
        Some(locator),
    )
    .unwrap();
    let UpdateCheck::UpdateAvailable(update) = manager.check_for_updates().unwrap() else {
        panic!("The older test host must discover the packaged release");
    };
    manager.download_updates(&update, None).unwrap();
    assert!(manager.get_update_pending_restart().is_some());
    fs::write(
        fixture.join("downloaded-version"),
        &update.TargetFullRelease.Version,
    )
    .unwrap();
    // Use the same graceful-exit API as the app. The Python parent verifies
    // replacement only after this test process exits, then launches the result.
    manager
        .wait_exit_then_apply_updates(&update.TargetFullRelease, true, false, Vec::<String>::new())
        .unwrap();
}
