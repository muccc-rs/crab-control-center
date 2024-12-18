use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Visuals {
    inner: Arc<Mutex<crate::logic::Channels>>,
}

impl Visuals {
    pub fn new() -> Self {
        let inner: Arc<Mutex<crate::logic::Channels>> = Default::default();

        Self { inner }
    }

    pub fn run(&self) {
        let channels = self.inner.clone();

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
            ..Default::default()
        };

        eframe::run_native(
            "crab control center",
            options,
            Box::new(|cc| {
                // This gives us image support:
                egui_extras::install_image_loaders(&cc.egui_ctx);

                Ok(Box::new(CrabVisualization { channels }))
            }),
        )
        .unwrap();
    }

    pub fn update_channels(&self, channels: &crate::logic::Channels) {
        let mut inner = self.inner.lock().unwrap();
        *inner = channels.clone();
    }
}

struct CrabVisualization {
    channels: Arc<Mutex<crate::logic::Channels>>,
}

impl eframe::App for CrabVisualization {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
        });
    }
}
