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
    show_confirmation: bool,
    dark_mode: bool,
    
    // Log buffer for display
    log_buffer: String,
}

impl RbcpApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            source: String::new(),
            destination: String::new(),
            copy_options: CopyOptions {
                recursive: true, // Default to recursive for GUI
                ..CopyOptions::default()
            },
            progress: SharedProgress::new(),
            engine_thread: None,
            show_log: true,
            show_options: false,
            show_confirmation: false, // New state
            dark_mode: true, // Default to dark mode
            log_buffer: String::new(),
        }
    }

    fn start_copy(&mut self) {
        if self.source.is_empty() || self.destination.is_empty() {
            // TODO: Show error
            return;
        }

        // Calculate effective destination
        let mut effective_dest = self.destination.clone();
        let source_path = std::path::Path::new(&self.source);
        if source_path.is_dir() {
            if let Some(folder_name) = source_path.file_name() {
                effective_dest = std::path::Path::new(&self.destination)
                    .join(folder_name)
                    .to_string_lossy()
                    .to_string();
            }
        }

        // Check if destination exists
        if std::path::Path::new(&effective_dest).exists() {
            self.show_confirmation = true;
            return;
        }

        self.run_copy_engine(effective_dest);
    }

    fn run_copy_engine(&mut self, destination: String) {
        let mut options = self.copy_options.clone();
        options.source = self.source.clone();
        options.destination = destination;
        options.show_progress = true; // Force progress for GUI
        
        // Default pattern if none specified
        if options.patterns.is_empty() {
            options.patterns.push("*".to_string());
        }

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
        // Apply theme
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

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

        // Define colors from mockup
        let purple_color = egui::Color32::from_rgb(160, 32, 240); // Purple
        // let border_color = egui::Color32::BLACK; // Default is fine for now

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            // Title and Theme Toggle
            ui.horizontal(|ui| {
                ui.heading(format!("RBCP version {}", crate::VERSION));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if self.dark_mode { "☀ Light Mode" } else { "🌙 Dark Mode" };
                    if ui.button(label).clicked() {
                        self.dark_mode = !self.dark_mode;
                    }
                });
            });
            ui.separator();

            // Source Section
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Source path:").color(purple_color));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("options").clicked() {
                        self.show_options = !self.show_options;
                    }
                });
            });
            
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.source).desired_width(ui.available_width() - 70.0));
                if ui.button("browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.source = path.to_string_lossy().to_string();
                    }
                }
            });

            // Destination Section
            ui.label(egui::RichText::new("Destination path:").color(purple_color));
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.destination).desired_width(ui.available_width() - 70.0));
                if ui.button("browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.destination = path.to_string_lossy().to_string();
                    }
                }
            });

            ui.add_space(5.0);

            // Progress Section
            ui.label(egui::RichText::new("Progress:").color(purple_color));
            
            let pct = if info.state == ProgressState::Idle { 0.0 } else { info.percentage() / 100.0 };
            let progress_text = if info.state == ProgressState::Idle { "".to_string() } else { format!("{:.0}%", info.percentage()) };
            
            // Invert text color for readability
            // On purple background (dark), we want white.
            // On empty background (dark grey in dark mode, white in light mode), we want white (dark mode) or black (light mode).
            // But the text spans both.
            // egui doesn't support mixed color text in progress bar easily.
            // We'll prioritize readability on the filled part (purple) which is usually the focus.
            // Purple is dark-ish. White text is good.
            // In light mode, empty bar is white. White text is invisible.
            // So: Dark Mode -> White text. Light Mode -> Black text (but might be hard to read on purple).
            // Let's try White for Dark Mode, and maybe Black for Light Mode.
            let text_color = if self.dark_mode { egui::Color32::WHITE } else { egui::Color32::BLACK };

            let progress_bar = egui::ProgressBar::new(pct)
                .text(egui::RichText::new(progress_text).color(text_color))
                .fill(purple_color)
                .animate(info.state == ProgressState::Scanning);
            
            ui.add(progress_bar);

            // Status Text
            ui.horizontal(|ui| {
                if info.state == ProgressState::Scanning {
                    ui.label(egui::RichText::new(format!("Scanning... {} files found", info.files_total)).color(purple_color));
                } else if info.files_total > 0 {
                    ui.label(egui::RichText::new(format!("{} of {} objects", info.files_done, info.files_total)).color(purple_color));
                } else if info.state == ProgressState::Idle || info.state == ProgressState::Completed {
                    ui.label("");
                } else {
                    ui.label(egui::RichText::new("0 of 0 objects").color(purple_color));
                }
                
                // Speed
                if info.speed > 0 && matches!(info.state, ProgressState::Copying) {
                    let speed_mb = info.speed as f64 / 1_048_576.0;
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{:.2} MB/s", speed_mb)).color(purple_color));
                    });
                }
            });
            
            // Current file path
            if info.state != ProgressState::Idle && info.state != ProgressState::Completed {
                ui.label(egui::RichText::new(&info.current_file).weak().size(12.0));
            } else {
                ui.label("");
            }

            ui.add_space(10.0);
            ui.separator();

            // Bottom Controls
            ui.horizontal(|ui| {
                // Show log checkbox (Left)
                if ui.checkbox(&mut self.show_log, "Show log").changed() {
                    if self.show_log {
                         ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([600.0, 500.0].into()));
                    } else {
                         ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([600.0, 280.0].into()));
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button_height = 30.0;
                    let button_width = 100.0;

                    // OK/Minimize (Rightmost)
                    let is_running = matches!(info.state, ProgressState::Scanning | ProgressState::Copying | ProgressState::Paused);
                    
                    if is_running {
                         if ui.add_sized([button_width, button_height], egui::Button::new("Minimize to tray")).clicked() {
                             ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                         }
                    } else {
                        if ui.add_sized([button_width, button_height], egui::Button::new("Start Copy")).clicked() {
                            self.start_copy();
                        }
                    }

                    // Pause/Continue
                    if is_running {
                        let label = if self.progress.is_paused() { "Continue" } else { "Pause" };
                        if ui.add_sized([button_width, button_height], egui::Button::new(label)).clicked() {
                            self.toggle_pause();
                        }
                    } else {
                        ui.add_enabled_ui(false, |ui| {
                            ui.add_sized([button_width, button_height], egui::Button::new("Pause/Continue"));
                        });
                    }

                    // Cancel
                    if is_running {
                        if ui.add_sized([button_width, button_height], egui::Button::new("Cancel")).clicked() {
                            self.cancel_copy();
                        }
                    } else {
                        ui.add_enabled_ui(false, |ui| {
                            ui.add_sized([button_width, button_height], egui::Button::new("Cancel"));
                        });
                    }
                });
            });

            // Log Area
            if self.show_log {
                ui.add_space(5.0);
                egui::Frame::canvas(ui.style()).stroke(egui::Stroke::new(1.0, egui::Color32::BLACK)).show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .max_height(150.0) // Limit height
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.log_buffer)
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Monospace)
                                    .frame(false) // No internal frame, we used external
                                    .interactive(false)
                            );
                        });
                });
            }
        });

        // Confirmation Modal
        if self.show_confirmation {
            egui::Window::new("Confirmation")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Destination directory already exists.");
                    ui.label("What would you like to do?");
                    ui.add_space(10.0);
                    
                    // Recalculate effective destination
                    let mut effective_dest = self.destination.clone();
                    let source_path = std::path::Path::new(&self.source);
                    if source_path.is_dir() {
                        if let Some(folder_name) = source_path.file_name() {
                            effective_dest = std::path::Path::new(&self.destination)
                                .join(folder_name)
                                .to_string_lossy()
                                .to_string();
                        }
                    }
                    
                    if ui.button("Overwrite All").clicked() {
                        self.copy_options.force_overwrite = true;
                        self.show_confirmation = false;
                        self.run_copy_engine(effective_dest.clone());
                    }
                    
                    if ui.button("Overwrite File (Update)").clicked() {
                        self.copy_options.force_overwrite = false;
                        self.show_confirmation = false;
                        self.run_copy_engine(effective_dest.clone());
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.show_confirmation = false;
                    }
                });
        }

        // Options Window
        if self.show_options {
            egui::Window::new("Options")
                .open(&mut self.show_options)
                .show(ctx, |ui| {
                    ui.checkbox(&mut self.copy_options.recursive, "Recursive (/S)");
                    if ui.checkbox(&mut self.copy_options.include_empty, "Include Empty (/E)").changed() {
                        if self.copy_options.include_empty {
                            self.copy_options.recursive = true;
                        }
                    }
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
