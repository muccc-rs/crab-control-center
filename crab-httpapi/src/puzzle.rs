use std::collections::{BinaryHeap, HashMap};
use std::time::Instant;

// Keep in sync with rfc-quiz/src/main.rs but we do not want to share dependencies with that
// project and factoring it is awfully complex.
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)] // for validation and keeping the structure in sync
pub struct Puzzle {
    pr_number: u32,
    markdown: String,
    html: String,
    judgement: Judgement,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, utoipa::ToSchema, serde::Serialize, serde::Deserialize,
)]
pub enum Judgement {
    Merge,
    Closed,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    utoipa::ToSchema,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct PuzzleKey(#[serde(with = "hex")] [u8; 16]);

pub struct Puzzles {
    by_id: HashMap<PuzzleKey, usize>,
    inner: Vec<Puzzle>,
    timeout: BinaryHeap<(Instant, PuzzleKey)>,
}

impl Puzzles {
    pub fn new() -> Self {
        Self::from_file(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rfc-quiz/puzzles.json"
        ))
    }

    pub fn from_file(file: &str) -> Self {
        let mut inner = vec![];

        if let Err(io) = Self::try_to_fill(&mut inner, file) {
            log::error!("Failed to load puzzles from {}: {}", file, io);
        }

        Self {
            by_id: HashMap::new(),
            inner,
            timeout: BinaryHeap::new(),
        }
    }

    pub fn generate(&mut self, timeout: Instant) -> PuzzleKey {
        use rand::Rng as _;
        let mut rng = rand::rng();

        let mut key = PuzzleKey([0; 16]);
        rng.fill(&mut key.0);

        if self.inner.len() == 0 {
            log::warn!("No puzzles available to generate.");
            // Unsolvable puzzle state, we have no puzzles.
            return key;
        }

        let uni = rand::distr::Uniform::new(0, self.inner.len()).unwrap();
        let idx = rng.sample(uni);

        self.by_id.insert(key.clone(), idx);
        self.timeout.push((timeout, key.clone()));

        log::info!(
            "Generated puzzle {:?} for {idx} with timeout at {:?}",
            key,
            timeout
        );

        key
    }

    pub fn next_timeout(&self) -> Option<Instant> {
        self.timeout.peek().map(|(when, _)| *when)
    }

    pub fn reap(&mut self, now: Instant) {
        while let Some((when, key)) = self.timeout.peek() {
            if *when > now {
                break;
            }

            log::info!("Reaping unsolved puzzle {:?} at {:?}", key, now);
            self.by_id.remove(key);
            self.timeout.pop();
        }
    }

    pub fn solve(&mut self, key: &PuzzleKey, solve: Judgement) -> (bool, Option<Judgement>) {
        if let Some(idx) = self.by_id.remove(key) {
            let judge = self.inner.get(idx).map(|puzzle| puzzle.judgement);
            let okay = judge.map_or(false, |judge| judge == solve);
            (okay, judge)
        } else {
            (false, None)
        }
    }

    pub fn get_html(&self, key: &PuzzleKey) -> Option<&str> {
        self.by_id
            .get(key)
            .and_then(|&idx| self.inner.get(idx))
            .map(|puzzle| puzzle.html.as_str())
    }

    fn try_to_fill(inner: &mut Vec<Puzzle>, file: &str) -> std::io::Result<()> {
        let data = std::fs::File::open(file)?;
        let puzzles: Vec<Puzzle> = serde_json::from_reader(data)?;
        log::info!("Loaded {} puzzles from {}", puzzles.len(), file);
        inner.extend(puzzles);
        Ok(())
    }
}
