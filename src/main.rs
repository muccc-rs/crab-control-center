use profirust::dp;
use profirust::fdl;
use profirust::phy;

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
        .watchdog_timeout(profirust::time::Duration::from_secs(10));

    let sleep_time = std::time::Duration::from_millis(60);

    (parameters, sleep_time)
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_micros()
        .init();

    println!("WAGO 750-343 Remote I/O Station Example");

    let mut dp_master = dp::DpMaster::new(vec![]);

    // Options generated by `gsdtool` using "wagob757.gsd"
    let options = profirust::dp::PeripheralOptions {
        // "WAGO 750-343" by "WAGO Kontakttechnik GmbH"
        ident_number: 0xb757,

        // Global Parameters:
        //   - DP-Watchdog-Base...............: 10 ms
        //   - Restart on K-Bus Failure.......: POWER ON RESET
        //   - Device Diagnosis...............: enabled
        //   - Process Data Representation....: MOTOROLA (MSB-LSB)
        //   - Response to PROFIBUS DP Failure: Output image is cleared
        //   - Response to K-Bus Failure......: PROFIBUS communication stops
        //
        // Selected Modules:
        //   [0] 750-343 No PI Channel
        //   [1] 750-504  4 DO/24 V DC/0.5 A
        //       - Terminal is physically....: plugged
        //       - Substitude Value Channel 1: 0
        //       - Substitude Value Channel 2: 0
        //       - Substitude Value Channel 3: 0
        //       - Substitude Value Channel 4: 0
        //   [2] 750-504  4 DO/24 V DC/0.5 A
        //       - Terminal is physically....: plugged
        //       - Substitude Value Channel 1: 0
        //       - Substitude Value Channel 2: 0
        //       - Substitude Value Channel 3: 0
        //       - Substitude Value Channel 4: 0
        //   [3] 750-504  4 DO/24 V DC/0.5 A
        //       - Terminal is physically....: plugged
        //       - Substitude Value Channel 1: 0
        //       - Substitude Value Channel 2: 0
        //       - Substitude Value Channel 3: 0
        //       - Substitude Value Channel 4: 0
        //   [4] 750-504  4 DO/24 V DC/0.5 A
        //       - Terminal is physically....: plugged
        //       - Substitude Value Channel 1: 0
        //       - Substitude Value Channel 2: 0
        //       - Substitude Value Channel 3: 0
        //       - Substitude Value Channel 4: 0
        //   [5] 750-504  4 DO/24 V DC/0.5 A
        //       - Terminal is physically....: plugged
        //       - Substitude Value Channel 1: 0
        //       - Substitude Value Channel 2: 0
        //       - Substitude Value Channel 3: 0
        //       - Substitude Value Channel 4: 0
        //   [6] 750-402  4 DI/24 V DC/3.0 ms
        //       - Terminal is physically: plugged
        //   [7] 750-466  2 AI/4-20 mA/SE
        //       - Terminal is physically: plugged
        //       - Diagnosis Channel 1...: enabled
        //       - Diagnosis Channel 2...: disabled
        user_parameters: Some(&[
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0xc3, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x80, 0x2b, 0x00, 0x21, 0x02, 0x00, 0x21, 0x02, 0x00,
            0x21, 0x02, 0x00, 0x21, 0x02, 0x00, 0x21, 0x02, 0x00, 0x21, 0x01, 0x00, 0x24, 0x50,
            0x11, 0x06,
        ]),
        config: Some(&[0x00, 0x20, 0x20, 0x20, 0x20, 0x20, 0x10, 0x51]),

        // Set max_tsdr depending on baudrate and assert
        // that a supported baudrate is used.
        max_tsdr: match BAUDRATE {
            profirust::Baudrate::B9600 => 60,
            profirust::Baudrate::B19200 => 60,
            profirust::Baudrate::B93750 => 60,
            profirust::Baudrate::B187500 => 60,
            profirust::Baudrate::B500000 => 100,
            profirust::Baudrate::B1500000 => 150,
            profirust::Baudrate::B3000000 => 250,
            profirust::Baudrate::B6000000 => 350,
            profirust::Baudrate::B12000000 => 550,
            b => panic!("Peripheral \"WAGO 750-343\" does not support baudrate {b:?}!"),
        },

        fail_safe: true,
        ..Default::default()
    };
    let mut buffer_inputs = [0u8; 5];
    let mut buffer_outputs = [0u8; 5];
    let mut buffer_diagnostics = [0u8; 64];
    let io_handle = dp_master.add(
        dp::Peripheral::new(
            IO_STATION_ADDRESS,
            options,
            &mut buffer_inputs[..],
            &mut buffer_outputs[..],
        )
        .with_diag_buffer(&mut buffer_diagnostics[..]),
    );

    let (parameters, sleep_time) = bus_parameters();

    let mut fdl = fdl::FdlActiveStation::new(parameters.build_verified(&dp_master));

    println!("Connecting to the bus...");
    let mut phy = phy::SerialPortPhy::new(BUS_DEVICE, fdl.parameters().baudrate);

    let start = profirust::time::Instant::now();

    fdl.set_online();
    dp_master.enter_operate();
    loop {
        let now = profirust::time::Instant::now();
        fdl.poll(now, &mut phy, &mut dp_master);

        let events = dp_master.take_last_events();

        // Get mutable access the the peripheral here so we can interact with it.
        let remoteio = dp_master.get_mut(io_handle);

        if remoteio.is_running() && events.cycle_completed {
            // println!("Inputs: {:?}", remoteio.pi_i());
            let (pi_i, pi_q) = remoteio.pi_both();
            let dc_ok = process_image::tag!(pi_i, X, 0, 0);
            let pressure = (u32::from(pi_i[1]) << 8) + u32::from(pi_i[2]);

            println!("DC OK: {dc_ok} PRESSURE: {pressure}");

            process_image::process_image! {
                pub struct mut PiOutputs: 5 {
                    /// -KEC1-K1 DO1
                    pub bottom_f: (X, 0, 0),
                    /// -KEC1-K1 DO2
                    pub bottom_b: (X, 0, 1),
                    /// -KEC1-K1 DO3
                    pub pupil_down: (X, 0, 2),
                    /// -KEC1-K1 DO4
                    pub pupil_top: (X, 0, 3),
                    /// -KEC1-K2 DO1
                    pub eye: (X, 1, 0),
                    /// -KEC1-K2 DO2
                    pub mouth_mid: (X, 1, 1),
                    /// -KEC1-K2 DO3
                    pub mouth_bot: (X, 1, 2),
                    /// -KEC1-K2 DO4
                    pub mouth_top: (X, 1, 3),
                    /// -KEC1-K3 DO1
                    pub spikes_left: (X, 2, 0),
                    /// -KEC1-K3 DO2
                    pub spikes_mid: (X, 2, 1),
                    /// -KEC1-K3 DO3
                    pub spikes_right: (X, 2, 2),

                    pub ind_no_fault: (X, 4, 0),
                    pub ind_air_refill: (X, 4, 1),
                }
            }

            pi_q.fill(0x00);

            let mut pi_q = PiOutputs::try_from(pi_q).unwrap();

            // Only set outputs when DC 5V supply is okay
            if dc_ok {
                *pi_q.ind_no_fault() = true;

                *pi_q.bottom_f() = true;
                *pi_q.bottom_b() = true;
                *pi_q.eye() = true;

                *pi_q.spikes_mid() = true;
                *pi_q.spikes_right() = true;
                *pi_q.spikes_left() = true;

                let elapsed = (now - start).total_millis();

                if (usize::try_from(elapsed / 2000).unwrap() % 2) == 1 {
                    // Sad
                    *pi_q.pupil_down() = true;
                    *pi_q.mouth_top() = true;
                } else {
                    // Happy
                    *pi_q.pupil_top() = true;
                    *pi_q.mouth_mid() = true;
                    *pi_q.mouth_bot() = true;
                }
            }
        }

        std::thread::sleep(sleep_time);
    }
}
