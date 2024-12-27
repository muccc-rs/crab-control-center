use crate::logic;

type Responder<T> = tokio::sync::oneshot::Sender<T>;

pub const EMOTION_RESET_TIMER_SECS: u64 = 60;

#[derive(Default, Clone, Debug)]
pub struct EmotionContainer(std::sync::Arc<tokio::sync::Mutex<logic::Emotion>>);

impl EmotionContainer {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn blocking_set(&mut self, emotion: logic::Emotion) {
        *self.0.blocking_lock() = emotion;
    }

    pub async fn set(&mut self, emotion: logic::Emotion) {
        *self.0.lock().await = emotion;
    }

    pub fn blocking_get(&self) -> logic::Emotion {
        *self.0.blocking_lock()
    }

    pub async fn get(&self) -> logic::Emotion {
        *self.0.lock().await
    }
}

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
    pub emotion: EmotionContainer,
    rx: tokio::sync::mpsc::Receiver<EmotionCommand>,
}

impl EmotionManager {
    pub fn new(emotion: EmotionContainer, rx: tokio::sync::mpsc::Receiver<EmotionCommand>) -> Self {
        Self { emotion, rx }
    }

    pub fn run(mut self) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    val = self.rx.recv() => {
                        match val {
                            Some(EmotionCommand::Get { resp }) => {
                                let _ = resp.send(self.emotion.get().await);
                            }
                            Some(EmotionCommand::Set { emotion, resp }) => {
                                self.emotion.set(emotion).await;
                                let _ = resp.send(());
                            }
                            None => return,
                        }
                    },
                    _ = tokio::time::sleep(std::time::Duration::from_secs(EMOTION_RESET_TIMER_SECS)) => {
                        self.emotion.set(logic::Emotion::default()).await;
                    }
                }
            }
        })
    }
}
