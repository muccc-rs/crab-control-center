use emotionmanager::EmotionCommand;

mod emotionmanager;
#[cfg(feature = "fieldbus")]
mod fieldbus;
mod graphql;
mod httpapi;
mod logic;
mod timers;
#[cfg(feature = "visuals")]
mod visuals;

fn main() {
    let (emotion_tx, emotion_rx) = tokio::sync::mpsc::channel::<EmotionCommand>(32);

    let emotioncontainer = emotionmanager::EmotionContainer::new();
    let emotionmanager = emotionmanager::EmotionManager::new(emotioncontainer.clone(), emotion_rx);

    let trigger_fan = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let fault_reset = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let graphql_context = graphql::Context::default();

    std::thread::spawn({
        let app_state = httpapi::AppState {
            emotion_ch_tx: emotion_tx.clone(),
            fault_reset: fault_reset.clone(),
            trigger_fan: trigger_fan.clone(),
        };
        let graphql_context = graphql_context.clone();
        move || {
            httpapi::run_http_server(app_state, emotionmanager, graphql_context);
        }
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
            loop {
                #[cfg(feature = "fieldbus")]
                if let Some(fieldbus) = &mut fieldbus {
                    let mut graphql_context = graphql_context.inner.blocking_write();

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
                        // -KEC1-K5 DO3
                        *tag_mut!(piq, X, 4, 2) = logic.outputs().run_fan;

                        // -KEC1-K6 DI1
                        logic.inputs_mut().dc_ok = tag!(pii, X, 1, 0);
                        // -KEC1-K6 DI2
                        logic.inputs_mut().estop_active = tag!(pii, X, 1, 1);

                        // -KEC1-K7 AI1
                        logic.inputs_mut().pressure_fullscale = tag!(pii, W, 2).into();

                        graphql_context.pii.copy_from_slice(pii);
                        graphql_context.piq.copy_from_slice(piq);
                    });
                }
                #[cfg(not(feature = "fieldbus"))]
                {
                    // Some sane defaults when no actual hardware is present
                    logic.inputs_mut().dc_ok = true;
                    logic.inputs_mut().estop_active = false;
                    logic.inputs_mut().pressure_fullscale = 64;
                }

                #[cfg(feature = "visuals")]
                visuals.update_channels(&logic.outputs().channels);

                {
                    let inputs = logic.inputs_mut();

                    inputs.emotion = Some(emotioncontainer.blocking_get());
                    inputs.trigger_fan =
                        trigger_fan.swap(false, std::sync::atomic::Ordering::SeqCst);
                    inputs.reset_fault =
                        fault_reset.swap(false, std::sync::atomic::Ordering::SeqCst);
                }

                let now = std::time::Instant::now();
                logic.run(now);

                // Mirror the logic state into the graphql context so it can be queried remotely.
                {
                    let mut graphql_context = graphql_context.inner.blocking_write();

                    graphql_context.logic_image.clone_from(&logic);
                    graphql_context.now = now;
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    });

    #[cfg(feature = "visuals")]
    visuals.run();
    #[cfg(not(feature = "visuals"))]
    _main_loop_handle.join().unwrap();
}
