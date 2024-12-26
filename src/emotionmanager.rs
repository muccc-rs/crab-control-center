use crate::logic;

type Responder<T> = tokio::sync::oneshot::Sender<T>;

#[derive(Debug)]
pub enum EmotionCommand {
    Get {
        resp: Responder<logic::Emotion>,
    },
    Set {
        emotion: logic::Emotion,
        resp: Responder<()>,
    },
}

#[derive(Debug)]
pub struct EmotionManager {
    pub emotion: logic::Emotion,
    rx: tokio::sync::mpsc::Receiver<EmotionCommand>,
}

impl EmotionManager {
    pub fn new(rx: tokio::sync::mpsc::Receiver<EmotionCommand>) -> Self {
        Self {
            emotion: logic::Emotion::Neutral,
            rx,
        }
    }

    pub fn run(mut self) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn(async move {
            while let Some(command) = self.rx.recv().await {
                match command {
                    EmotionCommand::Get { resp } => {
                        let _ = resp.send(self.emotion);
                    }
                    EmotionCommand::Set { emotion, resp } => {
                        self.emotion = emotion;
                        let _ = resp.send(());
                    }
                }
            }
        })
    }
}
