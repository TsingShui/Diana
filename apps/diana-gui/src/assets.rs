use std::{borrow::Cow, path::Path};

use gpui_kit::{AssetSource, Result, SharedString, assets::AllAssets};

/// Kit's icon catalog with Diana's project artwork under a stable asset path.
#[derive(Clone, Copy, Debug, Default)]
pub struct DianaAssets;

impl AssetSource for DianaAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == "diana.png" {
            return Ok(Some(Cow::Borrowed(include_bytes!(
                "../../../assets/diana.png"
            ))));
        }
        AllAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut assets = AllAssets.list(path)?;
        if Path::new("diana.png").starts_with(path) {
            assets.push("diana.png".into());
        }
        Ok(assets)
    }
}
