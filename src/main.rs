use profirust::dp;
use profirust::fdl;
use profirust::phy;

mod fieldbus;
mod logic;
mod timers;

// I/O Station Parameters
const IO_STATION_ADDRESS: u8 = 8;

// Bus Parameters
const MASTER_ADDRESS: u8 = 3;
const BUS_DEVICE: &'static str = "/dev/ttyUSB0";
const BAUDRATE: profirust::Baudrate = profirust::Baudrate::B19200;

fn bus_parameters() -> (fdl::ParametersBuilder, std::time::Duration) {
    let mut parameters = fdl::ParametersBuilder::new(MASTER_ADDRESS, BAUDRATE);
    parameters
        // We use a rather large T_slot time because USB-RS485 converters
        // can induce large delays at times.
        .slot_bits(576)
        .watchdog_timeout(profirust::time::Duration::from_secs(60));

    let sleep_time = std::time::Duration::from_millis(10);

    (parameters, sleep_time)
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_micros()
        .init();

    let mut fieldbus = fieldbus::Fieldbus::new();
    let mut logic = logic::Logic::new();

    fieldbus.enter_state(fieldbus::OperatingState::Operate);

    loop {
        fieldbus.with_process_images(|pii, piq| {
            use process_image::{tag, tag_mut};

            // -KEC1-K1 DO1
            *tag_mut!(piq, X, 0, 0) = logic.outputs().channels.bottom_front;
            // -KEC1-K1 DO2
            *tag_mut!(piq, X, 0, 1) = logic.outputs().channels.bottom_back;
            // -KEC1-K1 DO3
            *tag_mut!(piq, X, 0, 2) = logic.outputs().channels.pupil_down;
            // -KEC1-K1 DO4
            *tag_mut!(piq, X, 0, 2) = logic.outputs().channels.pupil_top;

            // -KEC1-K2 DO1
            *tag_mut!(piq, X, 1, 0) = logic.outputs().channels.eyes;
            // -KEC1-K2 DO2
            *tag_mut!(piq, X, 1, 1) = logic.outputs().channels.mouth_mid;
            // -KEC1-K2 DO3
            *tag_mut!(piq, X, 1, 1) = logic.outputs().channels.mouth_bottom;
            // -KEC1-K2 DO4
            *tag_mut!(piq, X, 1, 1) = logic.outputs().channels.mouth_top;

            // -KEC1-K3 DO1
            *tag_mut!(piq, X, 2, 0) = logic.outputs().channels.spikes_left;
            // -KEC1-K3 DO2
            *tag_mut!(piq, X, 2, 1) = logic.outputs().channels.spikes_mid;
            // -KEC1-K3 DO3
            *tag_mut!(piq, X, 2, 3) = logic.outputs().channels.spikes_right;

            // -KEC1-K5 DO1 (inverted!)
            *tag_mut!(piq, X, 4, 0) = !logic.outputs().indicator_fault;
            // -KEC1-K5 DO2
            *tag_mut!(piq, X, 4, 1) = logic.outputs().indicator_refill_air;

            // -KEC1-K6 DI1
            logic.inputs_mut().dc_ok = tag!(pii, X, 1, 0);

            // -KEC1-K7 AI1
            logic.inputs_mut().pressure_fullscale = tag!(pii, W, 2);
        });

        logic.run(std::time::Instant::now());

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
