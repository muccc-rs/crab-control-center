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

#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct PressureLimits {
    pub low_low: f64,
    pub low: f64,
    pub high: f64,
    pub high_high: f64,
}

impl Default for PressureLimits {
    fn default() -> Self {
        Self {
            low_low: 0.08,
            low: 0.25,
            high: 0.35,
            high_high: 0.4,
        }
    }
}

#[derive(Debug, Default, Clone, juniper::GraphQLObject)]
pub struct LogicInputs {
    pub emotion: Option<Emotion>,
    pub dc_ok: bool,
    pub pressure_fullscale: i32,
    pub estop_active: bool,
    pub trigger_fan: bool,
    pub reset_fault: bool,
    pub pressure_limits: PressureLimits,
}

#[derive(Debug, Default, Clone, juniper::GraphQLObject)]
pub struct LogicOutputs {
    pub channels: Channels,
    pub indicator_fault: bool,
    pub indicator_refill_air: bool,
    pub run_fan: bool,
}

#[derive(Debug, Default, Clone, juniper::GraphQLObject)]
#[graphql(name = "LogicState")]
#[graphql(context = crate::graphql::Context)]
pub struct Logic {
    #[graphql(ignore)]
    inp: LogicInputs,
    #[graphql(ignore)]
    out: LogicOutputs,

    blink: bool,
    t_blink: timers::BaseTimer<bool>,

    close_mouth: bool,
    t_close_mouth: timers::BaseTimer<bool>,

    t_fan: timers::BaseTimer<bool>,
    run_fan: bool,

    t_info: timers::BaseTimer<bool>,
    t_emotion: timers::BaseTimer<Option<Emotion>>,

    faulted: bool,
    reset_fault_last: bool,

    /// Pressure converted to engineering units
    ///
    /// None when no value is available
    pressure_mbar: Option<f64>,

    pressure_low_low: bool,
    pressure_low: bool,
    pressure_high: bool,
    pressure_high_high: bool,
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
        self.t_fan.run(now, self.out.run_fan);

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

        let reset_fault_edge = self.inp.reset_fault && !self.reset_fault_last;
        self.reset_fault_last = self.inp.reset_fault;

        // Disconnected pressure sensor
        let pressure_fault = self.inp.pressure_fullscale & 7 != 0;
        self.pressure_mbar = if !pressure_fault {
            Some(f64::from(self.inp.pressure_fullscale) * 250. / 65535.)
        } else {
            None
        };

        if let Some(pressure_mbar) = self.pressure_mbar {
            // LOWLOW and HIGHHIGH alarms are sticky and need to be cleared
            self.pressure_low_low = (self.pressure_low_low && !reset_fault_edge)
                || (pressure_mbar <= self.inp.pressure_limits.low_low);
            self.pressure_high_high = (self.pressure_high_high && !reset_fault_edge)
                || (pressure_mbar >= self.inp.pressure_limits.high_high);

            self.pressure_low = pressure_mbar <= self.inp.pressure_limits.low;
            self.pressure_high = pressure_mbar >= self.inp.pressure_limits.high;
        }

        // Maximum fan runtime
        let fan_overtime = self.out.run_fan && self.t_fan.timer(now, 60.secs());

        self.faulted = (self.faulted && !reset_fault_edge)
            || pressure_fault
            || fan_overtime
            || self.pressure_low_low
            || self.pressure_high_high
            || self.inp.estop_active
            || !self.inp.dc_ok;

        // Fan
        let fan_cooldown = !self.out.run_fan && self.t_fan.timer(now, 10.secs());
        let start_fan = (self.pressure_low || self.inp.trigger_fan) && fan_cooldown;
        self.run_fan = (self.run_fan || start_fan) && !self.pressure_high;
        let new_run_fan = self.run_fan && !self.faulted;
        if new_run_fan != self.out.run_fan {
            log::info!("FAN State: {new_run_fan}");
        }
        self.out.run_fan = new_run_fan;

        self.out.indicator_fault = self.faulted;
        self.out.indicator_refill_air = self.pressure_low;

        if self.t_info.timer(now, 60.secs()) {
            log::info!("Pressure: {}", self.inp.pressure_fullscale);
            self.t_info.trigger(now);
        }
    }
}
