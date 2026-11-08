//! Library export / import commands
//!
//! Moves signal libraries between machines as `.cvlib` files
//! (see `crate::library::bundle` for the file format).

use crate::app::CanViewerApp;
use crate::library::bundle::{self, BUNDLE_EXTENSION};
use gpui::Context;

impl CanViewerApp {
    /// Ask where to save, then export the selected library to a `.cvlib` file.
    pub fn export_selected_library(&mut self, cx: &mut Context<Self>) {
        let library = match self
            .selected_library_id
            .as_deref()
            .and_then(|id| self.library_manager.find_library(id))
        {
            Some(lib) => lib.clone(),
            None => {
                self.status_msg = "Select a library to export".into();
                cx.notify();
                return;
            }
        };

        let default_name = format!(
            "{}.{}",
            crate::library::sanitize_filename(&library.name),
            BUNDLE_EXTENSION
        );

        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title("Export Library")
                .add_filter("can-viewer Library", &[BUNDLE_EXTENSION])
                .set_file_name(&default_name)
                .save_file()
                .await
            else {
                return;
            };
            let dest = file.path().to_owned();
            let msg = match bundle::export_library(&library, &dest) {
                Ok(()) => format!("Exported library '{}' to {}", library.name, dest.display()),
                Err(e) => format!("❌ Export failed: {}", e),
            };
            if let Some(entity) = this.upgrade() {
                entity.update(cx, |app, cx| {
                    app.status_msg = msg.into();
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Ask for a `.cvlib` file and add the libraries it contains.
    pub fn import_library_file(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title("Import Library")
                .add_filter("can-viewer Library", &[BUNDLE_EXTENSION])
                .pick_file()
                .await
            else {
                return;
            };
            let src = file.path().to_owned();
            if let Some(entity) = this.upgrade() {
                entity.update(cx, |app, cx| {
                    app.import_libraries_from(&src, cx);
                });
            }
        })
        .detach();
    }

    fn import_libraries_from(&mut self, src: &std::path::Path, cx: &mut Context<Self>) {
        let libraries_dir =
            crate::library::libraries_base_path(self.config_file_path.as_deref());
        let imported = match bundle::import_libraries(
            src,
            self.library_manager.libraries(),
            &libraries_dir,
        ) {
            Ok(libs) => libs,
            Err(e) => {
                self.status_msg = format!("❌ Import failed: {}", e).into();
                cx.notify();
                return;
            }
        };

        let mut names = Vec::new();
        let mut first_id = None;
        for library in imported {
            let (id, name) = (library.id.clone(), library.name.clone());
            match self.library_manager.add_library(library) {
                Ok(()) => {
                    first_id.get_or_insert(id);
                    names.push(name);
                }
                Err(e) => eprintln!("⚠️  Skipped imported library: {}", e),
            }
        }

        self.app_config.libraries = self.library_manager.libraries().to_vec();
        self.save_config(cx);
        if let Some(id) = first_id {
            self.selected_library_id = Some(id);
            self.selected_version_id = None;
        }
        self.status_msg = if names.is_empty() {
            "❌ Import failed: no libraries could be added".into()
        } else {
            format!("Imported library {}", names.join(", ")).into()
        };
        cx.notify();
    }
}
