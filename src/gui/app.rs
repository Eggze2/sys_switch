//! GUI application using egui.

use anyhow::Result;
use eframe::egui;

use crate::models::BootEntry;
use crate::platform::{get_boot_manager, BootManager};

/// Main application state.
struct SysSwitchApp {
    entries: Vec<BootEntry>,
    selected: Option<usize>,
    log: String,
    manager: Box<dyn BootManager>,
    error_message: Option<String>,
    reboot_pending: bool,
}

impl SysSwitchApp {
    /// Create a new application instance.
    fn new() -> Self {
        let manager = get_boot_manager();
        let mut app = Self {
            entries: Vec::new(),
            selected: None,
            log: String::new(),
            manager,
            error_message: None,
            reboot_pending: false,
        };
        app.refresh();
        app
    }

    /// Refresh the boot entry list.
    fn refresh(&mut self) {
        self.error_message = None;
        self.reboot_pending = false;

        if !self.manager.available() {
            self.error_message = Some(
                "No supported boot manager found. Please install required tools or run as admin/root."
                    .to_string(),
            );
            return;
        }

        match self.manager.list_entries() {
            Ok(entries) => {
                self.log_line(&format!("Found {} boot entries", entries.len()));
                self.entries = entries;
                self.selected = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to get boot entries: {}", e));
            }
        }
    }

    /// Apply the selected boot entry.
    fn apply(&mut self) {
        let Some(idx) = self.selected else {
            self.error_message = Some("Please select a boot entry".to_string());
            return;
        };

        let Some(entry) = self.entries.get(idx) else {
            return;
        };

        match self.manager.set_next(&entry.id) {
            Ok(msg) => {
                self.log_line(&msg);
                self.refresh();
            }
            Err(e) => {
                self.log_line(&format!("Error: {}", e));
                self.error_message = Some(format!("{}", e));
            }
        }
    }

    /// Reboot the system.
    fn reboot(&mut self) {
        match self.manager.reboot_now() {
            Ok(()) => {}
            Err(e) => {
                self.log_line(&format!("Error: {}", e));
                self.error_message = Some(format!("{}", e));
            }
        }
    }

    /// Add a line to the log.
    fn log_line(&mut self, text: &str) {
        if !self.log.is_empty() {
            self.log.push('\n');
        }
        self.log.push_str(text);
    }
}

impl eframe::App for SysSwitchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Next Boot Selector");
            ui.add_space(8.0);

            // Error message
            if let Some(ref err) = self.error_message {
                ui.colored_label(egui::Color32::RED, err);
                ui.add_space(4.0);
            }

            // Platform info
            #[cfg(target_os = "linux")]
            ui.label("Platform: Linux");
            #[cfg(target_os = "windows")]
            ui.label("Platform: Windows");

            ui.add_space(8.0);

            // Boot entry list
            ui.group(|ui| {
                ui.set_min_height(200.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, entry) in self.entries.iter().enumerate() {
                        let label = format!(
                            "{} [{}]{}{}",
                            entry.description,
                            entry.id,
                            if entry.is_current { " (Current)" } else { "" },
                            if entry.is_next { " (Next)" } else { "" }
                        );

                        if ui
                            .selectable_label(self.selected == Some(i), &label)
                            .clicked()
                        {
                            self.selected = Some(i);
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Refresh").clicked() {
                    self.refresh();
                }
                if ui.button("Set as Next Boot").clicked() {
                    self.apply();
                }

                let reboot_label = if self.reboot_pending {
                    "Confirm Reboot"
                } else {
                    "Reboot Now"
                };

                if ui.button(reboot_label).clicked() {
                    if self.reboot_pending {
                        // Second click - actually reboot
                        self.reboot();
                    } else {
                        // First click - show confirmation
                        self.reboot_pending = true;
                        self.error_message =
                            Some("Click 'Confirm Reboot' to reboot now.".to_string());
                    }
                }
            });

            ui.add_space(8.0);

            // Log area
            ui.label("Log");
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.log.as_str())
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    );
                });
        });
    }
}

/// Run the GUI application.
pub fn run_gui() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 480.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Next Boot Selector",
        options,
        Box::new(|_cc| Ok(Box::new(SysSwitchApp::new()))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
