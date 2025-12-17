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
    let (pressure_limits_tx, mut pressure_limits_rx) = tokio::sync::mpsc::channel(8);

    let emotioncontainer = emotionmanager::EmotionContainer::new();
    let emotionmanager = emotionmanager::EmotionManager::new(emotioncontainer.clone(), emotion_rx);

    let trigger_fan = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let trigger_sleep = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let fault_reset = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let graphql_context = graphql::Context::default();

    std::thread::spawn({
        let app_state = httpapi::AppState {
            emotion_ch_tx: emotion_tx.clone(),
            fault_reset: fault_reset.clone(),
            trigger_fan: trigger_fan.clone(),
            trigger_sleep: trigger_sleep.clone(),
            pressure_limits_tx,
        };
        let graphql_context = graphql_context.clone();
        move || {
            let graphql_router = graphql::axum_router(graphql_context);
            httpapi::run_http_server(app_state, emotionmanager, graphql_router);
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
                let start = std::time::Instant::now();

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

                        // -KEC1-K4 DO1
                        *tag_mut!(piq, X, 3, 0) = logic.outputs().channels.right_claw;
                        // -KEC1-K4 DO2
                        *tag_mut!(piq, X, 3, 1) = logic.outputs().channels.left_claw;

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
                    inputs.trigger_sleep =
                        trigger_sleep.swap(false, std::sync::atomic::Ordering::SeqCst);
                    inputs.reset_fault =
                        fault_reset.swap(false, std::sync::atomic::Ordering::SeqCst);

                    if let Ok(limits) = pressure_limits_rx.try_recv() {
                        if let Some(low_low) = limits.low_low {
                            log::info!("Updating LOWLOW pressure limit to {low_low:.3} mbar");
                            inputs.pressure_limits.low_low = low_low;
                        }
                        if let Some(low) = limits.low {
                            log::info!("Updating LOW pressure limit to {low:.3} mbar");
                            inputs.pressure_limits.low = low;
                        }
                        if let Some(high) = limits.high {
                            log::info!("Updating HIGH pressure limit to {high:.3} mbar");
                            inputs.pressure_limits.high = high;
                        }
                        if let Some(high_high) = limits.high_high {
                            log::info!("Updating HIGHHIGH pressure limit to {high_high:.3} mbar");
                            inputs.pressure_limits.high_high = high_high;
                        }
                    }
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

                let cycletime = std::time::Instant::now() - start;
                metrics::histogram!("crab_logic_cycletime_seconds").record(cycletime.as_secs_f64());
            }
        }
    });

    #[cfg(feature = "visuals")]
    visuals.run();
    #[cfg(not(feature = "visuals"))]
    _main_loop_handle.join().unwrap();
}
