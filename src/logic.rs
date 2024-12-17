/// All emotions a rustacean can feel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emotion {
    Happy,
    Sad,
    Surprised,
    Angered,
    Neutral,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Channels {
    // Outline
    pub bottom_front: bool,
    pub bottom_back: bool,
    pub spikes_left: bool,
    pub spikes_mid: bool,
    pub spikes_right: bool,

    // Eyes
    pub eyes: bool,
    pub pupil_top: bool,
    pub pupil_down: bool,

    // Mouth
    pub mouth_mid: bool,
    pub mouth_top: bool,
    pub mouth_bottom: bool,
}

#[derive(Debug, Default, Clone)]
pub struct LogicInputs {
    pub emotion: Option<Emotion>,
    pub dc_ok: bool,
    pub pressure_fullscale: u16,
}

#[derive(Debug, Default, Clone)]
pub struct LogicOutputs {
    pub channels: Channels,
    pub indicator_fault: bool,
    pub indicator_refill_air: bool, 
}

#[derive(Debug, Default)]
pub struct Logic {
    inp: LogicInputs,
    out: LogicOutputs,
}

impl Logic {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inputs_mut(&mut self) -> &mut LogicInputs {
        &mut self.inp
    }

    pub fn outputs(&self) -> &LogicOutputs {
        &self.out
    }
}

impl Logic {
    pub fn run(&mut self, now: std::time::Instant) {
        self.out.channels = Channels {
            bottom_front: true,
            bottom_back: true,
            spikes_left: true,
            spikes_mid: true,
            spikes_right: true,

            eyes: true,
            pupil_top: true,
            pupil_down: false,

            mouth_top: false,
            mouth_mid: true,
            mouth_bottom: true,
        }
    }
}
