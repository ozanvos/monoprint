//! Public Foundation APIs for container storage and persistent user-selected files.
use anyhow::Result;
use std::path::Path;

pub fn font_bookmark(path: &Path) -> Result<Option<Vec<u8>>> {
    #[cfg(all(target_os = "macos", feature = "app-store"))]
    {
        use objc2_foundation::NSURLBookmarkCreationOptions as Options;
        let url = native::file_url(path)?;
        let data = url
            .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
                Options::WithSecurityScope | Options::SecurityScopeAllowOnlyReadAccess,
                None,
                None,
            )
            .map_err(|error| anyhow::anyhow!("Could not retain access to this font: {error}"))?;
        Ok(Some(data.to_vec()))
    }
    #[cfg(not(all(target_os = "macos", feature = "app-store")))]
    {
        let _ = path;
        Ok(None)
    }
}

pub fn with_file_access<T>(
    path: &Path,
    bookmark: Option<&[u8]>,
    read: impl FnOnce(&Path) -> Result<T>,
) -> Result<T> {
    #[cfg(all(target_os = "macos", feature = "app-store"))]
    {
        use objc2_foundation::{NSData, NSURL, NSURLBookmarkResolutionOptions as Options};
        let url = if let Some(bookmark) = bookmark {
            anyhow::ensure!(
                bookmark.len() <= 64 * 1024,
                "Saved font access is invalid. Choose the font again."
            );
            // No stale-data pointer is needed; Foundation resolves moved files too.
            unsafe {
                NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                    &NSData::with_bytes(bookmark),
                    Options::WithSecurityScope | Options::WithoutUI,
                    None,
                    std::ptr::null_mut(),
                )
            }
            .map_err(|error| {
                anyhow::anyhow!("Custom font is unavailable. Choose the font again: {error}")
            })?
        } else {
            native::file_url(path)?
        };
        // Balance every successful access, including error paths and background reads.
        let access = native::Access::new(url);
        if bookmark.is_some() {
            anyhow::ensure!(
                access.started,
                "Access to the custom font expired. Choose the font again."
            );
        }
        let resolved = access
            .url
            .path()
            .ok_or_else(|| anyhow::anyhow!("Choose a local file."))?;
        read(Path::new(&resolved.to_string()))
    }
    #[cfg(not(all(target_os = "macos", feature = "app-store")))]
    {
        let _ = bookmark;
        read(path)
    }
}

#[cfg(all(target_os = "macos", feature = "app-store"))]
pub fn home_directory() -> std::path::PathBuf {
    objc2_foundation::NSHomeDirectory().to_string().into()
}

#[cfg(all(target_os = "macos", feature = "app-store"))]
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    use objc2_foundation::{NSData, NSDataWritingOptions};
    // Foundation handles safe-save access granted to the native Save panel URL.
    let url = native::file_url(path)?;
    NSData::with_bytes(bytes)
        .writeToURL_options_error(&url, NSDataWritingOptions::Atomic)
        .map_err(|error| anyhow::anyhow!("Could not save {}: {error}", path.display()))
}

#[cfg(all(target_os = "macos", feature = "app-store"))]
mod native {
    use super::*;
    use objc2::rc::Retained;
    use objc2_foundation::{NSString, NSURL};
    pub fn file_url(path: &Path) -> Result<Retained<NSURL>> {
        let path = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Choose a UTF-8 file path."))?;
        Ok(NSURL::fileURLWithPath(&NSString::from_str(path)))
    }
    pub struct Access {
        pub url: Retained<NSURL>,
        pub started: bool,
    }
    impl Access {
        pub fn new(url: Retained<NSURL>) -> Self {
            let started = unsafe { url.startAccessingSecurityScopedResource() };
            Self { url, started }
        }
    }
    impl Drop for Access {
        fn drop(&mut self) {
            if self.started {
                unsafe { self.url.stopAccessingSecurityScopedResource() };
            }
        }
    }
}
