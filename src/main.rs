use emotionmanager::EmotionCommand;

mod emotionmanager;
#[cfg(feature = "fieldbus")]
mod fieldbus;
mod httpapi;
mod logic;
mod timers;
#[cfg(feature = "visuals")]
mod visuals;

fn main() {
    let (emotion_tx, emotion_rx) = tokio::sync::mpsc::channel::<EmotionCommand>(32);

    let emotionmanager = emotionmanager::EmotionManager::new(emotion_rx);

    let emotion_tx_http = emotion_tx.clone();
    std::thread::spawn(move || {
        httpapi::run_http_server(emotion_tx_http, emotionmanager);
    });

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_micros()
        .init();

    #[cfg(feature = "fieldbus")]
    let mut fieldbus = if std::env::var("FAKE_CRAB")
        .map(|v| v.parse::<bool>().unwrap())
        .unwrap_or_default()
    {
        // Fake crab gets no fieldbus
        None
    } else {
        Some(fieldbus::Fieldbus::new())
    };
    #[cfg(feature = "visuals")]
    let visuals = visuals::Visuals::new();
    let mut logic = logic::Logic::new();

    #[cfg(feature = "fieldbus")]
    if let Some(fieldbus) = &mut fieldbus {
        fieldbus.enter_state(fieldbus::OperatingState::Operate);
    }

    let _main_loop_handle = std::thread::spawn({
        #[cfg(feature = "visuals")]
        let visuals = visuals.clone();
        move || {
            let start = std::time::Instant::now();

            loop {
                #[cfg(feature = "fieldbus")]
                if let Some(fieldbus) = &mut fieldbus {
                    fieldbus.with_process_images(|pii, piq| {
                        use process_image::{tag, tag_mut};

                        // -KEC1-K1 DO1
                        *tag_mut!(piq, X, 0, 0) = logic.outputs().channels.bottom_front;
                        // -KEC1-K1 DO2
                        *tag_mut!(piq, X, 0, 1) = logic.outputs().channels.bottom_back;
                        // -KEC1-K1 DO3
                        *tag_mut!(piq, X, 0, 2) = logic.outputs().channels.pupil_down;
                        // -KEC1-K1 DO4
                        *tag_mut!(piq, X, 0, 3) = logic.outputs().channels.pupil_top;

                        // -KEC1-K2 DO1
                        *tag_mut!(piq, X, 1, 0) = logic.outputs().channels.eyes;
                        // -KEC1-K2 DO2
                        *tag_mut!(piq, X, 1, 1) = logic.outputs().channels.mouth_mid;
                        // -KEC1-K2 DO3
                        *tag_mut!(piq, X, 1, 2) = logic.outputs().channels.mouth_bottom;
                        // -KEC1-K2 DO4
                        *tag_mut!(piq, X, 1, 3) = logic.outputs().channels.mouth_top;

                        // -KEC1-K3 DO1
                        *tag_mut!(piq, X, 2, 0) = logic.outputs().channels.spikes_left;
                        // -KEC1-K3 DO2
                        *tag_mut!(piq, X, 2, 1) = logic.outputs().channels.spikes_mid;
                        // -KEC1-K3 DO3
                        *tag_mut!(piq, X, 2, 2) = logic.outputs().channels.spikes_right;

                        // -KEC1-K5 DO1 (inverted!)
                        *tag_mut!(piq, X, 4, 0) = !logic.outputs().indicator_fault;
                        // -KEC1-K5 DO2
                        *tag_mut!(piq, X, 4, 1) = logic.outputs().indicator_refill_air;

                        // -KEC1-K6 DI1
                        logic.inputs_mut().dc_ok = tag!(pii, X, 1, 0);

                        // -KEC1-K7 AI1
                        logic.inputs_mut().pressure_fullscale = tag!(pii, W, 2);
                    });
                }

                #[cfg(feature = "visuals")]
                visuals.update_channels(&logic.outputs().channels);

                let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                let cmd = EmotionCommand::Get { resp: resp_tx };
                emotion_tx
                    .blocking_send(cmd)
                    .expect("failed to send emotion command");

                logic.inputs_mut().emotion =
                    Some(resp_rx.blocking_recv().expect("failed to receive emotion"));

                logic.run(std::time::Instant::now());

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    });

    #[cfg(feature = "visuals")]
    visuals.run();
    #[cfg(not(feature = "visuals"))]
    _main_loop_handle.join().unwrap();
}
