use native_dialog::{self, FileDialog};
use std::path::PathBuf;

pub fn ask_open() -> Result<Option<PathBuf>, String> {
    match FileDialog::new().add_filter("Photosquid Project", &["photosquid"]).show_open_single_file() {
        Ok(selection) => Ok(selection),
        Err(_) => Err("Failed to ask user to open a file".into()),
    }
}

pub fn ask_save() -> Result<Option<PathBuf>, String> {
    match FileDialog::new().add_filter("Photosquid Project", &["photosquid"]).show_save_single_file() {
        Ok(selection) => Ok(selection),
        Err(_) => Err("Failed to ask user to save a file".into()),
    }
}
