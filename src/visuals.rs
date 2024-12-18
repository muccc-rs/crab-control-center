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
            viewport: egui::ViewportBuilder::default().with_inner_size([960., 540.]),
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
            ui.heading("Crab Visualization");
            let available_size = ui.available_size();
            let rect = egui::Rect::from_min_size(ui.min_rect().min, available_size);
            let channels = self.channels.lock().unwrap();
            if channels.bottom_back || channels.bottom_front {
                egui::Image::new(egui::include_image!("../vis/bottom.png")).paint_at(ui, rect);
            }
            if channels.spikes_left {
                egui::Image::new(egui::include_image!("../vis/spikes_left.png")).paint_at(ui, rect);
            }
            if channels.spikes_mid {
                egui::Image::new(egui::include_image!("../vis/spikes_mid.png")).paint_at(ui, rect);
            }
            if channels.spikes_right {
                egui::Image::new(egui::include_image!("../vis/spikes_right.png"))
                    .paint_at(ui, rect);
            }

            if channels.eyes {
                egui::Image::new(egui::include_image!("../vis/eyes.png")).paint_at(ui, rect);
            }
            if channels.pupil_top {
                egui::Image::new(egui::include_image!("../vis/pupil_top.png")).paint_at(ui, rect);
            }
            if channels.pupil_down {
                egui::Image::new(egui::include_image!("../vis/pupil_down.png")).paint_at(ui, rect);
            }

            if channels.mouth_top {
                egui::Image::new(egui::include_image!("../vis/mouth_top.png")).paint_at(ui, rect);
            }
            if channels.mouth_mid {
                egui::Image::new(egui::include_image!("../vis/mouth_mid.png")).paint_at(ui, rect);
            }
            if channels.mouth_bottom {
                egui::Image::new(egui::include_image!("../vis/mouth_bottom.png"))
                    .paint_at(ui, rect);
            }
        });
    }
}
