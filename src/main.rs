use eframe::egui;
use xcap::Monitor;
use std::thread;
use chrono::{Local, Duration as ChronoDuration};
use image::ImageFormat;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use std::fs::create_dir_all;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(600.0, 200.0)),
        ..Default::default()
    };
    
    eframe::run_native(
        "Overwatch", 
        options, 
        Box::new(|_cc| Box::new(MyApp::new()))
    );

    Ok(())
}

struct MyApp {
    interval_hours: u64,
    interval_minutes: u64,
    interval_seconds: u64,
    duration_hours: u64,
    duration_minutes: u64,
    duration_seconds: u64,
    is_running: bool,
    use_custom_name: bool,
    custom_name: String,
    screenshot_counter: Arc<AtomicU64>,  // Use atomic counter for thread-safe updates
    total_screenshots: u64,
    dark_mode: bool,  // Track whether dark mode is active
}

impl MyApp {
    fn new() -> Self {
        MyApp {
            interval_hours: 0,
            interval_minutes: 0,
            interval_seconds: 0,
            duration_hours: 0,
            duration_minutes: 0,
            duration_seconds: 0,
            is_running: false,
            use_custom_name: false,
            custom_name: String::new(),
            screenshot_counter: Arc::new(AtomicU64::new(0)),
            total_screenshots: 0,
            dark_mode: true,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Initialize the visuals according to the current mode
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        // Main panel for input and Start button
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Title on the left
                ui.heading("Overwatch");

                // Button on the far right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button_label = if self.dark_mode { "Light Mode" } else { "Dark Mode" };
                    if ui.button(button_label).clicked() {
                        self.dark_mode = !self.dark_mode;
                        if self.dark_mode {
                            ctx.set_visuals(egui::Visuals::dark()); // Switch to dark mode
                        } else {
                            ctx.set_visuals(egui::Visuals::light()); // Switch to light mode
                        }
                    }
                });
            });

            // Interval input (hours, minutes, seconds)
            ui.horizontal(|ui| {
                ui.label("Interval between screenshots:");

                // Align interval inputs to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::DragValue::new(&mut self.interval_seconds).prefix("seconds: ").speed(1));
                    ui.add(egui::DragValue::new(&mut self.interval_minutes).prefix("minutes: ").speed(1));
                    ui.add(egui::DragValue::new(&mut self.interval_hours).prefix("hours: ").speed(1));
                });
            });

            // Duration input (hours, minutes, seconds)
            ui.horizontal(|ui| {
                ui.label("Total duration:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::DragValue::new(&mut self.duration_seconds).prefix("seconds: ").speed(1));
                    ui.add(egui::DragValue::new(&mut self.duration_minutes).prefix("minutes: ").speed(1));
                    ui.add(egui::DragValue::new(&mut self.duration_hours).prefix("hours: ").speed(1));
                });
            });

            // Custom name checkbox/input in the same row
            ui.horizontal(|ui| {
                // Left-aligned checkbox
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.checkbox(&mut self.use_custom_name, "Use custom name for screenshots");
                });

                // Right-aligned custom name input with placeholder behavior
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.use_custom_name {
                        if self.custom_name.is_empty() {
                            // Display hint text in gray when the custom name is empty
                            ui.visuals_mut().override_text_color = Some(egui::Color32::GRAY);
                            let hint = "Enter custom name...";
                            let response = ui.text_edit_singleline(&mut self.custom_name);
                            if response.changed() && self.custom_name.is_empty() {
                                // Manually copy the hint into the field to simulate placeholder behavior
                                self.custom_name = hint.to_string();
                            }
                            ui.visuals_mut().override_text_color = None; // Reset the color after
                        } else {
                            // Normal input field
                            ui.text_edit_singleline(&mut self.custom_name);
                        }
                    }
                });
            });

            // Add the "Start" button on its own line at the bottom of the main panel
            ui.add_space(10.0); // Add some space before the Start button
            ui.horizontal_centered(|ui| {
                // Center the Start button without stretching it
                if ui.add_sized(egui::vec2(100.0, 30.0), egui::Button::new("Start")).clicked() && !self.is_running {
                    self.is_running = true;
                    self.screenshot_counter.store(0, Ordering::Relaxed); // Reset the counter

                    // Calculate interval and duration from input
                    let interval = Duration::from_secs(
                        self.interval_hours * 3600 +
                        self.interval_minutes * 60 +
                        self.interval_seconds
                    );

                    let duration = ChronoDuration::seconds(
                        (self.duration_hours * 3600 +
                         self.duration_minutes * 60 +
                         self.duration_seconds) as i64
                    );

                    // Calculate the total number of expected screenshots
                    self.total_screenshots = (duration.num_seconds() as u64) / interval.as_secs();

                    // Start the screenshot process with the appropriate naming scheme
                    let name_prefix = if self.use_custom_name && !self.custom_name.is_empty() {
                        self.custom_name.clone()
                    } else {
                        Local::now().format("%Y%m%d_%H%M%S").to_string() // Ensure there's no semicolon here
                    };

                    let folder_path = create_screenshot_folder(&name_prefix).expect("Failed to create folder");

                    let counter = Arc::clone(&self.screenshot_counter); // Clone the atomic counter for the thread
                    thread::spawn(move || {
                        start_screenshot_process(interval, duration, folder_path, counter);
                    });
                }
            });
        });

        // Bottom panel for Screenshot count
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Align the screenshot count to the far right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let current_count = self.screenshot_counter.load(Ordering::Relaxed); // Read the atomic counter
                    ui.label(format!(
                        "Screenshots taken: {}/{}",
                        current_count, self.total_screenshots
                    ));
                });
            });
        });

        ctx.request_repaint(); // Continuously request a repaint to update the UI
    }
}

fn create_screenshot_folder(name_prefix: &str) -> Result<PathBuf, Box<dyn Error>> {
    let base_folder = "screenshots"; // Base folder to store all sessions
    let unique_folder_name = format!("{}_{}", name_prefix, Local::now().format("%H%M%S"));
    let folder_path = PathBuf::from(base_folder).join(unique_folder_name);

    create_dir_all(&folder_path)?; // Create the directory, including parent folders if necessary
    Ok(folder_path)
}

fn start_screenshot_process(interval: Duration, duration: ChronoDuration, folder_path: PathBuf, counter: Arc<AtomicU64>) {
    let start_time = Local::now();
    let monitors = Monitor::all().unwrap(); // Get all monitors

    while Local::now() - start_time < duration {
        let loop_start = Instant::now(); // Mark the start of the loop

        for monitor in &monitors {
            // Capture screenshot from each monitor
            let image = monitor.capture_image().unwrap();

            // Create a filename with either the custom name or timestamp
            let filename = folder_path.join(format!("{}_{}.png", monitor.name(), Local::now().format("%Y%m%d_%H%M%S")));

            // Write the screenshot to a PNG file
            image.save_with_format(&filename, ImageFormat::Png).unwrap();
        }

        // Increment the screenshot count (thread-safe)
        counter.fetch_add(1, Ordering::Relaxed);

        // Calculate how long the screenshot process took
        let elapsed_time = loop_start.elapsed();
        
        // Adjust sleep time to account for processing time
        if elapsed_time < interval {
            thread::sleep(interval - elapsed_time);
        }
    }
}