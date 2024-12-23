use std::sync::{Arc, Mutex};

use profirust::dp;
use profirust::fdl;
use profirust::phy;

pub use dp::OperatingState;

// I/O Station Parameters
const IO_STATION_ADDRESS: u8 = 8;

// Bus Parameters
const MASTER_ADDRESS: u8 = 3;
const BUS_DEVICE: &'static str = "/dev/ttyUSB0";
const BAUDRATE: profirust::Baudrate = profirust::Baudrate::B19200;

// Sizes for the controller-side global input and output process images
pub const PIQ_SIZE: usize = 256;
pub const PII_SIZE: usize = 256;

fn bus_parameters() -> (fdl::ParametersBuilder, std::time::Duration) {
    let mut parameters = fdl::ParametersBuilder::new(MASTER_ADDRESS, BAUDRATE);
    parameters
        // We use a rather large T_slot time because USB-RS485 converters
        // can induce large delays at times.
        .slot_bits(576)
        .watchdog_timeout(profirust::time::Duration::from_secs(10));

    let sleep_time = std::time::Duration::from_millis(10);

    (parameters, sleep_time)
}

#[derive(Debug, Default)]
pub struct Fieldbus {
    inner: Arc<Mutex<FieldbusInner>>,
}

#[derive(Debug)]
struct FieldbusInner {
    state: OperatingState,
    is_online: bool,
    pii: [u8; PII_SIZE],
    piq: [u8; PIQ_SIZE],
}

impl Default for FieldbusInner {
    fn default() -> Self {
        Self {
            state: OperatingState::Stop,
            is_online: false,
            pii: [0u8; PII_SIZE],
            piq: [0u8; PIQ_SIZE],
        }
    }
}

impl Fieldbus {
    pub fn new() -> Self {
        let inner: Arc<Mutex<FieldbusInner>> = Default::default();
        std::thread::spawn({
            let inner = inner.clone();
            move || {
                fieldbus_task(inner);
            }
        });
        Self { inner }
    }

    pub fn enter_state(&mut self, state: OperatingState) {
        self.inner.lock().unwrap().state = state;
    }

    #[allow(unused)]
    pub fn is_online(&self) -> bool {
        self.inner.lock().unwrap().is_online
    }

    #[allow(unused)]
    pub fn update_process_images(&mut self, pii: &mut [u8; PII_SIZE], piq: &[u8; PIQ_SIZE]) {
        let mut data = self.inner.lock().unwrap();
        pii.copy_from_slice(&data.pii);
        data.piq.copy_from_slice(piq);
    }

    pub fn with_process_images<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&[u8; PII_SIZE], &mut [u8; PIQ_SIZE]) -> R,
    {
        let mut data = self.inner.lock().unwrap();
        let data = &mut *data;
        f(&data.pii, &mut data.piq)
    }
}

struct PeripheralInfo {
    pub handle: dp::PeripheralHandle,
    pub pii_offset: usize,
    pub piq_offset: usize,
    pub liveness_bit: (usize, u8),
}

fn fieldbus_task(fieldbus_data: Arc<Mutex<FieldbusInner>>) {
    let mut dp_master = dp::DpMaster::new(vec![]);
    let mut peripherals: Vec<PeripheralInfo> = Default::default();

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
    let handle = dp_master.add(
        dp::Peripheral::new(
            IO_STATION_ADDRESS,
            options,
            &mut buffer_inputs[..],
            &mut buffer_outputs[..],
        )
        .with_diag_buffer(&mut buffer_diagnostics[..]),
    );

    peripherals.push(PeripheralInfo {
        handle,
        pii_offset: 1,
        piq_offset: 0,
        liveness_bit: (0, 0),
    });

    let (parameters, sleep_time) = bus_parameters();
    let mut fdl = fdl::FdlActiveStation::new(parameters.build_verified(&dp_master));

    log::info!("Connecting to the bus...");
    let mut phy = phy::SerialPortPhy::new(BUS_DEVICE, fdl.parameters().baudrate);

    fdl.set_online();
    dp_master.enter_operate();
    loop {
        let now = profirust::time::Instant::now();
        fdl.poll(now, &mut phy, &mut dp_master);
        let events = dp_master.take_last_events();

        {
            let mut data = fieldbus_data.lock().unwrap();

            if dp_master.operating_state() != data.state {
                dp_master.enter_state(data.state);
            }

            data.is_online = fdl.is_in_ring();

            if events.cycle_completed {
                for peripheral_info in peripherals.iter() {
                    let peripheral = dp_master.get_mut(peripheral_info.handle);

                    {
                        let mut liveness_bit = process_image::tag_mut!(
                            &mut data.pii,
                            peripheral_info.liveness_bit.0,
                            peripheral_info.liveness_bit.1
                        );
                        *liveness_bit = peripheral.is_running();
                    }
                    {
                        let pii = peripheral.pi_i();
                        data.pii[peripheral_info.pii_offset..][..pii.len()].copy_from_slice(pii);
                    }
                    {
                        let piq = peripheral.pi_q_mut();
                        piq.copy_from_slice(&data.piq[peripheral_info.piq_offset..][..piq.len()]);
                    }
                }
            }
        }

        std::thread::sleep(sleep_time);
    }
}
