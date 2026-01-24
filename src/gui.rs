use eframe::egui;
use std::sync::Arc;
use std::thread;

use crate::{CopyOptions, CopyEngine, SharedProgress, ProgressState};

pub fn run_gui() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        &format!("RBCP version {}", crate::VERSION),
        options,
        Box::new(|cc| Ok(Box::new(RbcpApp::new(cc)))),
    )
}

struct RbcpApp {
    // Inputs
    source: String,
    destination: String,
    copy_options: CopyOptions,
    
    // State
    progress: SharedProgress,
    engine_thread: Option<thread::JoinHandle<()>>,
    show_log: bool,
    show_options: bool,
    
    // Log buffer for display
    log_buffer: String,
}

impl RbcpApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            source: String::new(),
            destination: String::new(),
            copy_options: CopyOptions::default(),
            progress: SharedProgress::new(),
            engine_thread: None,
            show_log: true,
            show_options: false,
            log_buffer: String::new(),
        }
    }

    fn start_copy(&mut self) {
        if self.source.is_empty() || self.destination.is_empty() {
            // TODO: Show error
            return;
        }

        let mut options = self.copy_options.clone();
        options.source = self.source.clone();
        options.destination = self.destination.clone();
        options.show_progress = true; // Force progress for GUI

        let progress = self.progress.clone();
        progress.reset();
        self.log_buffer.clear();

        self.engine_thread = Some(thread::spawn(move || {
            let engine = CopyEngine::new(options, Arc::new(progress));
            let _ = engine.run();
        }));
    }

    fn cancel_copy(&mut self) {
        self.progress.cancel();
    }

    fn toggle_pause(&mut self) {
        self.progress.toggle_pause();
    }
}

impl eframe::App for RbcpApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll progress updates
        let info = self.progress.get_info();
        
        // Poll logs
        let new_logs = self.progress.take_logs();
        for log in new_logs {
            self.log_buffer.push_str(&log);
            self.log_buffer.push('\n');
        }

        // Request repaint if running to animate progress bar
        if info.state == ProgressState::Scanning || info.state == ProgressState::Copying {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Title
            ui.heading(format!("RBCP version {}", crate::VERSION));
            ui.separator();

            // Source
            ui.horizontal(|ui| {
                ui.label("Source path:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("options").clicked() {
                        self.show_options = !self.show_options;
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.source).hint_text("Source directory"));
                if ui.button("browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.source = path.to_string_lossy().to_string();
                    }
                }
            });

            // Destination
            ui.label("Destination path:");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.destination).hint_text("Destination directory"));
                if ui.button("browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.destination = path.to_string_lossy().to_string();
                    }
                }
            });

            ui.add_space(10.0);

            // Progress
            ui.label("Progress:");
            let pct = info.percentage() / 100.0;
            let progress_bar = egui::ProgressBar::new(pct)
                .show_percentage()
                .animate(info.state == ProgressState::Scanning);
            
            ui.add(progress_bar);

            // Status text
            ui.horizontal(|ui| {
                if info.state == ProgressState::Scanning {
                    ui.label(format!("Scanning... {} files found", info.files_total));
                } else if info.files_total > 0 {
                    ui.label(format!("{} of {} objects", info.files_done, info.files_total));
                } else {
                    ui.label("Ready");
                }
            });
            
            // Current file
            ui.label(egui::RichText::new(&info.current_file).weak().size(12.0));

            ui.add_space(10.0);
            ui.separator();

            // Controls
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.show_log, "Show log").changed() {
                    // Toggle logic handled by state
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // OK/Minimize (Start)
                    let is_running = matches!(info.state, ProgressState::Scanning | ProgressState::Copying | ProgressState::Paused);
                    
                    if is_running {
                         if ui.button("Minimize to tray").clicked() {
                             // TODO: Implement tray
                             ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                         }
                    } else {
                        if ui.button("Start Copy").clicked() {
                            self.start_copy();
                        }
                    }

                    // Pause/Continue
                    if is_running {
                        let label = if self.progress.is_paused() { "Continue" } else { "Pause" };
                        if ui.button(label).clicked() {
                            self.toggle_pause();
                        }
                    } else {
                        ui.add_enabled(false, egui::Button::new("Pause/Continue"));
                    }

                    // Cancel
                    if is_running {
                        if ui.button("Cancel").clicked() {
                            self.cancel_copy();
                        }
                    } else {
                        ui.add_enabled(false, egui::Button::new("Cancel"));
                    }
                });
            });

            // Log Area
            if self.show_log {
                ui.separator();
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true) // Auto scroll to bottom
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.log_buffer)
                                .desired_width(f32::INFINITY)
                                .desired_rows(10)
                                .font(egui::TextStyle::Monospace)
                                .interactive(false) // Read-only
                        );
                    });
            }
        });

        // Options Window
        if self.show_options {
            egui::Window::new("Options")
                .open(&mut self.show_options)
                .show(ctx, |ui| {
                    ui.checkbox(&mut self.copy_options.recursive, "Recursive (/S)");
                    ui.checkbox(&mut self.copy_options.include_empty, "Include Empty (/E)");
                    ui.checkbox(&mut self.copy_options.mirror, "Mirror (/MIR)");
                    ui.checkbox(&mut self.copy_options.purge, "Purge (/PURGE)");
                    ui.checkbox(&mut self.copy_options.move_files, "Move Files (/MOV)");
                    ui.checkbox(&mut self.copy_options.restartable, "Restartable (/Z)");
                    ui.checkbox(&mut self.copy_options.shred_files, "Secure Delete (/SHRED)");
                    
                    ui.horizontal(|ui| {
                        ui.label("Threads:");
                        ui.add(egui::DragValue::new(&mut self.copy_options.threads).range(1..=128));
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Retries:");
                        ui.add(egui::DragValue::new(&mut self.copy_options.retries));
                    });
                });
        }
    }
}
