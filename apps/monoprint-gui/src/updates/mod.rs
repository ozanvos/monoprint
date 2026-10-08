//! Platform-owned update installation with one GPUI status surface.
#[cfg(any(target_os = "linux", all(test, unix)))]
pub mod linux_install;
#[cfg(all(target_os = "macos", not(feature = "app-store")))]
mod sparkle;
mod ui;

use gpui_kit::*;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use {
    std::time::Duration,
    velopack::{UpdateCheck, UpdateManager, VelopackAsset},
};

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(any(target_os = "windows", target_os = "linux")), allow(dead_code))]
pub enum Status {
    Disabled(String),
    Idle,
    Checking,
    Downloading,
    Current,
    Ready(String),
    Failed(String),
}

pub struct Updates {
    pub status: Status,
    pub automatic: bool,
    #[cfg(all(target_os = "macos", not(feature = "app-store")))]
    sparkle: Option<sparkle::Sparkle>,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    manager: Option<UpdateManager>,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    ready: Option<VelopackAsset>,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    _poll: Task<()>,
}

/// Must run before reading arguments or opening a window. We apply only after an
/// explicit restart action, so a pending update cannot bypass the draft flush.
pub fn startup() {
    #[cfg(target_os = "windows")]
    velopack::VelopackApp::build()
        .set_auto_apply_on_startup(false)
        .run();
    #[cfg(target_os = "linux")]
    if let Ok(locator) = linux_locator() {
        velopack::VelopackApp::build()
            .set_locator(locator)
            .set_auto_apply_on_startup(false)
            .run();
    }
}

impl Updates {
    pub fn new(cx: &mut Context<Self>) -> Self {
        #[cfg(all(target_os = "macos", not(feature = "app-store")))]
        {
            let _ = cx;
            match sparkle::Sparkle::new() {
                Ok(sparkle) => Self {
                    status: Status::Idle,
                    automatic: sparkle.automatic(),
                    sparkle: Some(sparkle),
                },
                Err(error) => Self {
                    status: Status::Disabled(error.to_string()),
                    automatic: false,
                    sparkle: None,
                },
            }
        }
        #[cfg(all(target_os = "macos", feature = "app-store"))]
        {
            let _ = cx;
            Self {
                status: Status::Disabled("Updates are delivered by the App Store".into()),
                automatic: false,
            }
        }
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            let manager = manager();
            let status = match &manager {
                Ok(_) => Status::Idle,
                Err(error) => Status::Disabled(error.to_string()),
            };
            let manager = manager.ok();
            let ready = manager
                .as_ref()
                .and_then(UpdateManager::get_update_pending_restart);
            let status = ready
                .as_ref()
                .map_or(status, |asset| Status::Ready(asset.Version.to_string()));
            let automatic = preferences_path()
                .and_then(|path| std::fs::read(path).ok())
                .and_then(|bytes| serde_json::from_slice::<bool>(&bytes).ok())
                .unwrap_or(true);
            let poll = cx.spawn(async move |this, cx| {
                loop {
                    if this
                        .update(cx, |this, cx| {
                            if this.automatic {
                                this.check(cx);
                            }
                        })
                        .is_err()
                    {
                        break;
                    }
                    cx.background_executor()
                        .timer(Duration::from_secs(6 * 60 * 60))
                        .await;
                }
            });
            Self {
                status,
                automatic,
                manager,
                ready,
                _poll: poll,
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            let _ = cx;
            Self {
                status: Status::Disabled("This platform uses manual updates".into()),
                automatic: false,
            }
        }
    }

    pub fn available(&self) -> bool {
        !matches!(self.status, Status::Disabled(_))
    }

    pub fn check(&mut self, cx: &mut Context<Self>) {
        #[cfg(all(target_os = "macos", not(feature = "app-store")))]
        if let Some(sparkle) = &self.sparkle {
            sparkle.check();
        }
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            if matches!(
                self.status,
                Status::Checking | Status::Downloading | Status::Ready(_)
            ) {
                return;
            }
            let Some(manager) = self.manager.clone() else {
                return;
            };
            self.status = Status::Checking;
            let check = cx
                .background_executor()
                .spawn(async move { manager.check_for_updates() });
            cx.spawn(async move |this, cx| match check.await {
                Ok(UpdateCheck::UpdateAvailable(info)) => {
                    let download = this.update(cx, |this, cx| {
                        this.status = Status::Downloading;
                        cx.notify();
                        let manager = this.manager.clone().unwrap();
                        cx.background_executor().spawn(async move {
                            manager.download_updates(&info, None)?;
                            Ok::<_, velopack::Error>(info.TargetFullRelease.clone())
                        })
                    });
                    if let Ok(download) = download {
                        let result = download.await;
                        let _ = this.update(cx, |this, cx| {
                            match result {
                                Ok(asset) => {
                                    this.status = Status::Ready(asset.Version.to_string());
                                    this.ready = Some(asset);
                                }
                                Err(error) => this.status = Status::Failed(error.to_string()),
                            }
                            cx.notify();
                        });
                    }
                }
                result => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = match result {
                            Ok(UpdateCheck::RemoteIsEmpty) => {
                                Status::Failed("The update feed is empty".into())
                            }
                            Ok(_) => Status::Current,
                            Err(error) => Status::Failed(error.to_string()),
                        };
                        cx.notify();
                    });
                }
            })
            .detach();
        }
        cx.notify();
    }

    pub fn toggle_automatic(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        #[cfg(all(target_os = "macos", not(feature = "app-store")))]
        if let Some(sparkle) = &self.sparkle {
            self.automatic = sparkle.automatic();
        }
        let enabled = !self.automatic;
        #[cfg(all(target_os = "macos", not(feature = "app-store")))]
        if let Some(sparkle) = &self.sparkle {
            sparkle.set_automatic(enabled);
        }
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            let path = preferences_path()
                .ok_or_else(|| anyhow::anyhow!("Cannot locate update preferences"))?;
            std::fs::create_dir_all(path.parent().unwrap())?;
            crate::document::write_atomic(&path, &serde_json::to_vec(&enabled)?)?;
        }
        self.automatic = enabled;
        if enabled {
            self.check(cx);
        }
        cx.notify();
        Ok(())
    }

    pub fn restart(&self) -> anyhow::Result<()> {
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        if let (Some(manager), Some(asset)) = (&self.manager, &self.ready) {
            manager.wait_exit_then_apply_updates(asset, false, true, Vec::<String>::new())?;
            return Ok(());
        }
        anyhow::bail!("No downloaded update is ready")
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn preferences_path() -> Option<std::path::PathBuf> {
    Some(crate::document::data_directory()?.join("automatic-updates.json"))
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn manager() -> anyhow::Result<UpdateManager> {
    if let Some(reason) = std::env::var_os("MONOPRINT_UPDATE_EXPLANATION") {
        anyhow::bail!("{}", reason.to_string_lossy());
    }
    #[cfg(target_os = "linux")]
    {
        anyhow::ensure!(
            std::env::var_os("FLATPAK_ID").is_none() && std::env::var_os("SNAP").is_none(),
            "Updates are delivered by your package manager"
        );
        let image = std::env::var_os("APPIMAGE")
            .ok_or_else(|| anyhow::anyhow!("Updates are available in the AppImage"))?;
        anyhow::ensure!(
            std::fs::OpenOptions::new().write(true).open(image).is_ok(),
            "Move the AppImage to a writable location to enable updates"
        );
    }
    let feed = option_env!("MONOPRINT_UPDATE_URL")
        .ok_or_else(|| anyhow::anyhow!("Updates are not configured for this build"))?;
    let url = url::Url::parse(feed)?;
    anyhow::ensure!(
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none(),
        "The update feed must use HTTPS"
    );
    let options = velopack::UpdateOptions {
        ExplicitChannel: option_env!("MONOPRINT_UPDATE_CHANNEL").map(str::to_owned),
        ..Default::default()
    };
    #[cfg(target_os = "linux")]
    let locator = Some(linux_locator()?);
    #[cfg(target_os = "windows")]
    let locator = None;
    Ok(UpdateManager::new(
        velopack::sources::HttpSource::new(feed),
        Some(options),
        locator,
    )?)
}

#[cfg(target_os = "linux")]
fn linux_locator() -> anyhow::Result<velopack::locator::VelopackLocatorConfig> {
    use anyhow::Context;
    use std::path::PathBuf;
    let image =
        PathBuf::from(std::env::var_os("APPIMAGE").context("Missing AppImage")?).canonicalize()?;
    let binary = std::env::current_exe()?;
    let directory = binary.parent().context("Missing binary directory")?;
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .context("Missing update cache directory")?
        .join("monoprint/packages");
    Ok(velopack::locator::VelopackLocatorConfig {
        RootAppDir: image,
        CurrentBinaryDir: directory.to_owned(),
        UpdateExePath: directory.join("UpdateNix"),
        ManifestPath: directory.join("sq.version"),
        PackagesDir: cache,
        IsPortable: true,
    })
}
