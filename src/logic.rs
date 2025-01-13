use crate::timers;
use timers::TimeExt;
use utoipa::ToSchema;

/// All emotions a rustacean can feel
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, ToSchema, serde::Deserialize, juniper::GraphQLEnum,
)]
pub enum Emotion {
    #[default]
    Happy,
    Sad,
    Surprised,
    Angered,
    Neutral,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, juniper::GraphQLObject)]
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

#[derive(Debug, Default, Clone, juniper::GraphQLObject)]
pub struct LogicInputs {
    pub emotion: Option<Emotion>,
    pub dc_ok: bool,
    pub pressure_fullscale: i32,
}

#[derive(Debug, Default, Clone, juniper::GraphQLObject)]
pub struct LogicOutputs {
    pub channels: Channels,
    pub indicator_fault: bool,
    pub indicator_refill_air: bool,
}

#[derive(Debug, Default, Clone, juniper::GraphQLObject)]
#[graphql(name = "LogicState")]
pub struct Logic {
    #[graphql(ignore)]
    inp: LogicInputs,
    #[graphql(ignore)]
    out: LogicOutputs,

    blink: bool,
    #[graphql(ignore)]
    t_blink: timers::BaseTimer<bool>,

    close_mouth: bool,
    #[graphql(ignore)]
    t_close_mouth: timers::BaseTimer<bool>,

    #[graphql(ignore)]
    t_info: timers::BaseTimer<bool>,
    #[graphql(ignore)]
    t_emotion: timers::BaseTimer<Option<Emotion>>,
}

impl Logic {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inputs(&self) -> &LogicInputs {
        &self.inp
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
        self.t_blink.run(now, self.blink);
        self.t_close_mouth.run(now, self.close_mouth);
        self.t_emotion.run(now, self.inp.emotion);

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
        };

        if !self.t_emotion.timer(now, 1.millis()) {
            log::info!("New Emotion: {:?}", self.inp.emotion);
        }

        match self.inp.emotion {
            Some(Emotion::Happy) => {
                self.out.channels.pupil_down = false;
                self.out.channels.pupil_top = true;
                self.out.channels.mouth_top = false;
                self.out.channels.mouth_mid = true;
                self.out.channels.mouth_bottom = true;
            }
            Some(Emotion::Sad) => {
                self.out.channels.pupil_down = true;
                self.out.channels.pupil_top = false;
                self.out.channels.mouth_top = true;
                self.out.channels.mouth_mid = false;
                self.out.channels.mouth_bottom = false;
            }
            Some(Emotion::Surprised) => {
                self.out.channels.pupil_down = false;
                self.out.channels.pupil_top = true;
                self.out.channels.mouth_top = true;
                self.out.channels.mouth_mid = false;
                self.out.channels.mouth_bottom = true;
            }
            Some(Emotion::Angered) => {
                self.out.channels.pupil_down = false;
                self.out.channels.pupil_top = true;
                self.out.channels.mouth_top = true;
                self.out.channels.mouth_mid = true;
                self.out.channels.mouth_bottom = false;
            }
            Some(Emotion::Neutral) => {
                self.out.channels.pupil_down = false;
                self.out.channels.pupil_top = true;
                self.out.channels.mouth_top = false;
                self.out.channels.mouth_mid = true;
                self.out.channels.mouth_bottom = false;
            }
            None => (),
        }

        self.blink = match self.blink {
            false if self.t_blink.timer(now, 3.secs()) => true,
            true if self.t_blink.timer(now, 300.millis()) => false,
            d => d,
        };

        if self.blink {
            self.out.channels.pupil_top = false;
            self.out.channels.pupil_down = true;
        } else {
            self.out.channels.pupil_top = true;
            self.out.channels.pupil_down = false;
        }

        self.close_mouth = match self.close_mouth {
            _ if !self.t_emotion.timer(now, 10.secs()) => false,
            false if self.t_close_mouth.timer(now, 10.secs()) => true,
            true if self.t_close_mouth.timer(now, 2.secs()) => false,
            d => d,
        };

        if self.close_mouth {
            self.out.channels.mouth_top = false;
            self.out.channels.mouth_bottom = false;
            self.out.channels.mouth_mid = true;
        }

        // Disconnected pressure sensor
        let pressure_fault = self.inp.pressure_fullscale & 7 != 0;
        let pressure_low = !pressure_fault && self.inp.pressure_fullscale < 16;

        self.out.indicator_fault = pressure_fault;
        self.out.indicator_refill_air = pressure_low;

        if self.t_info.timer(now, 60.secs()) {
            log::info!("Pressure: {}", self.inp.pressure_fullscale);
            self.t_info.trigger(now);
        }
    }
}
