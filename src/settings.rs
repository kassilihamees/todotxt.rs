//! Preferences shared by the platform frontends.
use crate::{document::atomic_write, view::Sort};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub file: Option<PathBuf>,
    pub archive: Option<PathBuf>,
    pub sort: Sort,
    pub filter: String,
    pub presets: [String; 9],
    pub active_preset: usize,
    pub hide_future: bool,
    pub show_hidden: bool,
    pub grouping: bool,
    pub word_wrap: bool,
    pub status_bar: bool,
    pub case_sensitive: bool,
    pub intellisense_case: bool,
    pub add_creation: bool,
    pub auto_refresh: bool,
    pub auto_archive: bool,
    pub auto_archive_path: bool,
    pub ctrl_enter: bool,
    pub focus_list: bool,
    pub preserve_blank: bool,
    pub font_size: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            file: None,
            archive: None,
            sort: Sort::Alphabetical,
            filter: String::new(),
            presets: Default::default(),
            active_preset: 0,
            hide_future: true,
            show_hidden: false,
            grouping: true,
            word_wrap: false,
            status_bar: true,
            case_sensitive: false,
            intellisense_case: false,
            add_creation: false,
            auto_refresh: false,
            auto_archive: false,
            auto_archive_path: false,
            ctrl_enter: false,
            focus_list: true,
            preserve_blank: false,
            font_size: 12.0,
        }
    }
}
impl Settings {
    pub fn directory(custom: Option<PathBuf>) -> PathBuf {
        custom.unwrap_or_else(|| {
            directories::ProjectDirs::from("", "", "todotxt.rs")
                .map(|p| p.config_dir().to_owned())
                .unwrap_or_else(|| ".local".into())
        })
    }
    pub fn read(directory: &Path) -> io::Result<Self> {
        match fs::read(directory.join("settings.json")) {
            Ok(bytes) => {
                let mut settings: Self =
                    serde_json::from_slice(&bytes).map_err(io::Error::other)?;
                settings.font_size = settings.font_size.clamp(8.0, 30.0);
                settings.active_preset = settings.active_preset.min(9);
                Ok(settings)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }
    pub fn save(&self, directory: &Path) -> io::Result<()> {
        fs::create_dir_all(directory)?;
        atomic_write(
            &directory.join("settings.json"),
            &serde_json::to_vec_pretty(self).map_err(io::Error::other)?,
        )
    }
}
