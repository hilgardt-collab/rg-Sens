//! Configuration management

mod defaults;
mod settings;

pub use defaults::{DefaultsConfig, GeneralDefaults};
pub use settings::{AppConfig, GridConfig, PanelConfig, PanelConfigV2, WindowConfig};

/// Write `content` to `path` atomically: write to a temp file in the same
/// directory, fsync, then rename over the destination. A crash or full disk
/// mid-write leaves either the old file or the new one — never a truncated
/// config that the next save would clobber with defaults.
pub(crate) fn write_atomic(path: &std::path::Path, content: &str) -> anyhow::Result<()> {
    use std::io::Write;

    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent directory: {}", path.display()))?;
    std::fs::create_dir_all(dir)?;

    let tmp_path = path.with_extension("json.tmp");
    {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    Ok(())
}
