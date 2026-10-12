//! Backline for Android. Step 1: proves the build, the screen and the sound work on a phone.

use eframe::egui;

#[derive(Default)]
struct App {
    note: String,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.heading("Backline");
                ui.add_space(20.0);
                if ui.add(egui::Button::new("TEST SOUND").min_size(egui::vec2(200.0, 60.0))).clicked() {
                    self.note = match beep() {
                        Ok(()) => "Sound works".to_string(),
                        Err(e) => format!("No sound: {}", e),
                    };
                }
                ui.add_space(10.0);
                ui.label(&self.note);
            });
        });
    }
}

/// Half a second of A440, only when the button is pressed.
fn beep() -> Result<(), String> {
    std::thread::spawn(|| {
        let Ok((_stream, handle)) = rodio::OutputStream::try_default() else { return };
        let Ok(sink) = rodio::Sink::try_new(&handle) else { return };
        let n = 22050;
        let v: Vec<f32> = (0..n).map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 44100.0).sin() * 0.3).collect();
        sink.append(rodio::buffer::SamplesBuffer::new(1, 44100, v));
        sink.sleep_until_end();
    });
    Ok(())
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(move |b| {
            b.with_android_app(app);
        })),
        ..Default::default()
    };
    let _ = eframe::run_native("Backline", options, Box::new(|_cc| Ok(Box::new(App::default()))));
}
