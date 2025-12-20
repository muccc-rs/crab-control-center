#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub message: String,
}

pub(crate) async fn handle_alerts(mut rx: tokio::sync::mpsc::Receiver<crate::alert::Alert>) {
    let client = reqwest::Client::new();

    let ntfy_topic = std::env::var("CRAB_NTFY_TOPIC").ok();

    'alerts: while let Some(alert) = rx.recv().await {
        log::warn!("Alert: {}", alert.message);

        if let Some(ntfy_topic) = ntfy_topic.as_deref() {
            for _ in 0..3 {
                let res = client
                    .post(format!("https://ntfy.sh/{ntfy_topic}"))
                    .header("Title", "Crab Alert!")
                    .header("Tags", "rotating_light")
                    .body(alert.message.clone())
                    .send()
                    .await;

                match res {
                    Ok(_) => continue 'alerts,
                    Err(_) => {
                        log::warn!("Failed to submit alert! Retrying...");
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }

            log::error!("Giving up on submitting alert! Dropping.");
        } else {
            log::info!("Not submitting alert because CRAB_NTFY_TOPIC is not set.");
        }
    }
}
