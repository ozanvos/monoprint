//! Explicit per-user AppImage integration; never overwrites an existing install.
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::{Read, Seek, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

const ICON: &str = include_str!("../../assets/monoprint.svg");

/// Velopack replaces the AppImage through its containing directory. Opening
/// the running image for writing fails with ETXTBSY even when replacement is
/// permitted, so probe a temporary sibling instead. Never modify the image.
pub fn ensure_appimage_replaceable(image: &Path) -> Result<()> {
    let image = image
        .canonicalize()
        .context("Cannot locate the running AppImage")?;
    ensure!(image.is_file(), "The AppImage path is not a regular file");
    let parent = image
        .parent()
        .context("The AppImage has no parent directory")?;
    let probe = tempfile::Builder::new()
        .prefix(".monoprint-update-check-")
        .tempfile_in(parent)
        .with_context(|| format!("Cannot create an update in {}. Move the AppImage to a writable folder or install it for this user", parent.display()))?;
    probe
        .close()
        .context("Cannot remove the temporary update check")?;
    Ok(())
}

#[derive(Clone)]
pub struct Installation {
    pub image: PathBuf,
    desktop: PathBuf,
    icon: PathBuf,
    marker: PathBuf,
    state: PathBuf,
}

impl Installation {
    #[cfg(target_os = "linux")]
    pub fn for_user() -> Result<Self> {
        let home =
            PathBuf::from(std::env::var_os("HOME").context("Cannot locate your home directory")?);
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        let state = crate::document::data_directory().context("Cannot locate saved drafts")?;
        Self::new(&home, &data, &state)
    }

    pub fn new(home: &Path, data: &Path, state: &Path) -> Result<Self> {
        ensure!(
            home.is_absolute() && data.is_absolute() && state.is_absolute(),
            "Installation paths must be absolute"
        );
        let folder = home.join("Applications/Monoprint");
        Ok(Self {
            image: folder.join("Monoprint.AppImage"),
            desktop: data.join("applications/app.monoprint.studio.desktop"),
            icon: data.join("monoprint/monoprint.svg"),
            marker: folder.join("installation.json"),
            state: state.to_owned(),
        })
    }

    pub fn installed(&self) -> bool {
        self.owned().is_ok()
    }

    fn desktop_text(&self) -> Result<String> {
        Ok(format!(
            "[Desktop Entry]\nType=Application\nName=Monoprint\nComment=Create beautiful code images\nExec={} %f\nIcon={}\nTerminal=false\nCategories=Development;Graphics;\nStartupWMClass=Monoprint\n",
            exec_path(&self.image)?,
            value_path(&self.icon)?
        ))
    }

    fn marker_text(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(
            &serde_json::json!({"app": "app.monoprint.studio", "image": self.image, "desktop": self.desktop, "icon": self.icon}),
        )?)
    }

    fn owned(&self) -> Result<()> {
        for path in [&self.image, &self.marker, &self.desktop, &self.icon] {
            let metadata = fs::symlink_metadata(path)?;
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "Installed files must be regular files"
            );
        }
        ensure!(
            fs::read(&self.marker)? == self.marker_text()?,
            "This installation is not owned by Monoprint"
        );
        ensure!(
            fs::read_to_string(&self.desktop)? == self.desktop_text()?,
            "The launcher was changed; remove it manually"
        );
        ensure!(
            fs::read_to_string(&self.icon)? == ICON,
            "The icon was changed; remove it manually"
        );
        Ok(())
    }

    pub fn install(&self, source: &Path) -> Result<PathBuf> {
        let mut input = fs::File::open(source).context("Cannot read the AppImage")?;
        let mut header = [0; 11];
        input.read_exact(&mut header)?;
        ensure!(
            &header[..4] == b"\x7fELF" && &header[8..11] == b"AI\x02",
            "Expected a type-2 AppImage"
        );
        // Validate before creating anything. persist_noclobber also protects
        // against destinations appearing between this check and the write.
        let desktop = self.desktop_text()?;
        let marker = self.marker_text()?;
        for path in [&self.image, &self.desktop, &self.icon, &self.marker] {
            ensure!(
                fs::symlink_metadata(path)
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
                "An installation file already exists: {}",
                path.display()
            );
        }
        let mut created = Vec::new();
        let result = (|| {
            input.rewind()?;
            create_file(&self.image, &mut input, true)?;
            created.push(&self.image);
            create_file(&self.icon, &mut ICON.as_bytes(), false)?;
            created.push(&self.icon);
            create_file(&self.desktop, &mut desktop.as_bytes(), false)?;
            created.push(&self.desktop);
            create_file(&self.marker, &mut marker.as_slice(), false)?;
            Ok(self.image.clone())
        })();
        if result.is_err() {
            for path in created.into_iter().rev() {
                let _ = fs::remove_file(path);
            }
        }
        result
    }

    pub fn uninstall(&self, remove_data: bool) -> Result<()> {
        self.owned()?;
        // Never remove arbitrary session overrides or recursively delete folders.
        if remove_data {
            for name in ["studio.json", "automatic-updates.json"] {
                let path = self.state.join(name);
                match fs::symlink_metadata(&path) {
                    Ok(meta) => {
                        ensure!(
                            meta.is_file() && !meta.file_type().is_symlink(),
                            "Saved data must be a regular file"
                        );
                        fs::remove_file(path)?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                    Err(error) => return Err(error.into()),
                }
            }
        }
        for path in [&self.desktop, &self.icon, &self.image, &self.marker] {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

fn create_file(path: &Path, input: &mut impl Read, executable: bool) -> Result<()> {
    let parent = path.parent().context("Missing installation folder")?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(input, &mut file)?;
    file.flush()?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(if executable {
            0o755
        } else {
            0o644
        }))?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path)?;
    Ok(())
}

fn value_path(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .context("The installation path must be UTF-8")?;
    ensure!(
        !value.chars().any(char::is_control),
        "The installation path contains control characters"
    );
    Ok(value.replace('\\', "\\\\"))
}

fn exec_path(path: &Path) -> Result<String> {
    // Desktop entries apply string unescaping before Exec argument unescaping.
    let value = value_path(path)?;
    Ok(format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\\\"")
            .replace('`', "\\\\`")
            .replace('$', "\\\\$")
            .replace('%', "%%")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn running_executable_can_be_replaced_without_opening_it_for_write() {
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join("Monoprint.AppImage");
        fs::copy("/bin/sleep", &image).unwrap();
        let before = fs::read(&image).unwrap();
        let mut process = std::process::Command::new(&image)
            .arg("30")
            .spawn()
            .unwrap();
        let write_error = fs::OpenOptions::new().write(true).open(&image).unwrap_err();
        let replaceable = ensure_appimage_replaceable(&image);
        process.kill().unwrap();
        process.wait().unwrap();
        assert_eq!(write_error.raw_os_error(), Some(26), "expected ETXTBSY");
        replaceable.unwrap();
        assert_eq!(fs::read(&image).unwrap(), before);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn missing_image_and_directory_paths_are_not_update_targets() {
        let directory = tempfile::tempdir().unwrap();
        assert!(ensure_appimage_replaceable(&directory.path().join("missing")).is_err());
        assert!(ensure_appimage_replaceable(directory.path()).is_err());
    }

    #[test]
    fn install_is_executable_and_uninstall_preserves_draft_by_default() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home with spaces");
        let state = home.join("state");
        let install = Installation::new(&home, &home.join("data"), &state).unwrap();
        let source = temp.path().join("download.AppImage");
        fs::write(&source, b"\x7fELF\x02\x01\x01\x00AI\x02fixture").unwrap();
        install.install(&source).unwrap();
        assert!(install.installed());
        assert_eq!(
            fs::metadata(&install.image).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert!(
            fs::read_to_string(&install.desktop)
                .unwrap()
                .contains("Monoprint.AppImage\" %f")
        );
        fs::create_dir_all(&state).unwrap();
        fs::write(state.join("studio.json"), "draft").unwrap();
        assert!(install.install(&source).is_err());
        install.uninstall(false).unwrap();
        assert_eq!(
            fs::read_to_string(state.join("studio.json")).unwrap(),
            "draft"
        );
        assert!(source.exists());
        assert!(!install.image.exists());
    }

    #[test]
    fn does_not_overwrite_files_or_uninstall_an_edited_launcher() {
        let temp = tempfile::tempdir().unwrap();
        let install = Installation::new(
            temp.path(),
            &temp.path().join("data"),
            &temp.path().join("state"),
        )
        .unwrap();
        let source = temp.path().join("source");
        fs::write(&source, b"\x7fELF\x02\x01\x01\x00AI\x02fixture").unwrap();
        fs::create_dir_all(install.desktop.parent().unwrap()).unwrap();
        fs::write(&install.desktop, "keep").unwrap();
        assert!(install.install(&source).is_err());
        assert!(!install.image.exists());
        fs::remove_file(&install.desktop).unwrap();
        install.install(&source).unwrap();
        fs::write(&install.desktop, "custom").unwrap();
        assert!(install.uninstall(true).is_err());
        assert!(install.image.exists());
    }

    #[test]
    fn opt_in_data_removal_only_deletes_owned_files() {
        let temp = tempfile::tempdir().unwrap();
        let state = temp.path().join("state");
        let install = Installation::new(temp.path(), &temp.path().join("data"), &state).unwrap();
        let source = temp.path().join("source");
        fs::write(&source, b"\x7fELF\x02\x01\x01\x00AI\x02fixture").unwrap();
        install.install(&source).unwrap();
        fs::create_dir_all(&state).unwrap();
        for name in ["studio.json", "automatic-updates.json", "unrelated"] {
            fs::write(state.join(name), "keep").unwrap();
        }
        install.uninstall(true).unwrap();
        assert!(!state.join("studio.json").exists());
        assert!(state.join("unrelated").exists());
    }

    #[test]
    fn desktop_paths_escape_field_codes_and_reject_newlines() {
        assert_eq!(
            exec_path(Path::new("/home/a $b%/AppImage")).unwrap(),
            "\"/home/a \\\\$b%%/AppImage\""
        );
        assert!(exec_path(Path::new("/home/a\nExec=bad")).is_err());
    }

    #[test]
    fn queued_saves_do_not_recreate_a_deleted_draft() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("studio.json");
        let store = crate::document::SessionStore::new(Some(path.clone()));
        let draft = crate::document::Document::default();
        store.save(1, &draft).unwrap();
        store.set_saving_enabled(false);
        fs::remove_file(&path).unwrap();
        store.save(2, &draft).unwrap();
        assert!(!path.exists());
        store.set_saving_enabled(true);
        store.save(2, &draft).unwrap();
        assert!(path.exists());
    }
}
