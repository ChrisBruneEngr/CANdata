//! Math signal export / import commands
//!
//! Moves Math signals between machines as `.cvmath` files
//! (see `crate::library::math_signals` for the file format).

use crate::app::CanDataApp;
use crate::library::math_signals::{
    ConflictChoice, MATH_EXTENSION, MathSignal, export_math_signals, read_math_signals,
    save_math_signal_store,
};
use gpui::Context;

impl CanDataApp {
    /// Ask where to save, then export every Math signal to a `.cvmath` file.
    pub fn export_math_signals_file(&mut self, cx: &mut Context<Self>) {
        if self.math_store.signals.is_empty() {
            self.status_msg = "There are no Math signals to export".into();
            cx.notify();
            return;
        }
        let signals = self.math_store.signals.clone();
        let default_name = format!("math_signals.{}", MATH_EXTENSION);

        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title("Export Math Signals")
                .add_filter("CANdata Math Signals", &[MATH_EXTENSION])
                .set_file_name(&default_name)
                .save_file()
                .await
            else {
                return;
            };
            let dest = file.path().to_owned();
            let msg = match export_math_signals(&signals, &dest) {
                Ok(()) => format!("Exported {} Math signals to {}", signals.len(), dest.display()),
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

    /// Ask for a `.cvmath` file and add its Math signals. If some names are
    /// already used, the Math view asks what to do with them.
    pub fn import_math_signals_file(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title("Import Math Signals")
                .add_filter("CANdata Math Signals", &[MATH_EXTENSION])
                .pick_file()
                .await
            else {
                return;
            };
            let result = read_math_signals(file.path());
            if let Some(entity) = this.upgrade() {
                entity.update(cx, |app, cx| {
                    match result {
                        Ok(signals) if app.math_store.conflicts(&signals).is_empty() => {
                            app.apply_math_import(signals, ConflictChoice::Skip);
                        }
                        Ok(signals) => {
                            app.math.confirm_delete = false;
                            app.math.pending_import = Some(signals);
                        }
                        Err(e) => app.status_msg = format!("❌ Import failed: {}", e).into(),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Add imported Math signals, settling used names with `choice`.
    pub fn apply_math_import(&mut self, signals: Vec<MathSignal>, choice: ConflictChoice) {
        self.math.pending_import = None;
        let summary = self.math_store.import(signals, choice);
        self.status_msg = match save_math_signal_store(&self.math_store, self.config_file_path.as_deref()) {
            Ok(()) => summary.message().into(),
            Err(e) => format!("❌ {}", e).into(),
        };
        // A replaced signal may be in the editor or on the plot.
        self.math.reload_editor = true;
        if !summary.replaced.is_empty() {
            crate::ui::views::math_view::replot_if_math_plotted(self);
        }
    }
}
