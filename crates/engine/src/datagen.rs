//! Seeded self-play training data generation with opening normalization and adjudication.

#![expect(
    clippy::missing_errors_doc,
    reason = "Datagen configuration and execution errors are described in plain prose."
)]

use std::{
    error::Error,
    fmt,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    thread,
};

use gwaymaegyi_core::{Board, Color, Game, MoveKind, Outcome, Promotion, START_FEN};
use gwaymaegyi_search::{
    Engine, EngineError, GameResult, MAX_DEPTH, Mode, Options, SearchLimits, TrainingRecord,
};

const MAX_GAME_BUFFER: usize = 5_000;

/// Configuration for a bounded self-play data generation run.
#[derive(Clone, Debug)]
pub struct DatagenConfig {
    pub positions: usize,
    pub threads: u8,
    pub seed: u64,
    pub chess960: bool,
    pub opt_nodes: u64,
    pub max_nodes: u64,
    pub max_depth: u8,
    pub openings: Vec<String>,
}

impl Default for DatagenConfig {
    fn default() -> Self {
        Self {
            positions: 64,
            threads: 1,
            seed: 19,
            chess960: false,
            opt_nodes: 5_000,
            max_nodes: 50_000,
            max_depth: 10,
            openings: Vec::new(),
        }
    }
}

/// Failures when validating or running self-play data generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatagenError {
    InvalidPositions,
    InvalidThreads,
    InvalidLimits,
    InvalidOpening(String),
    Engine(EngineError),
    WorkerFailure,
}

impl fmt::Display for DatagenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPositions => {
                formatter.write_str("datagen position count must be 1 through 100000")
            }
            Self::InvalidThreads => {
                formatter.write_str("datagen thread count must be 1 through 16")
            }
            Self::InvalidLimits => {
                formatter.write_str("datagen node and depth limits must be positive and valid")
            }
            Self::InvalidOpening(reason) => write!(formatter, "invalid opening FEN: {reason}"),
            Self::Engine(error) => error.fmt(formatter),
            Self::WorkerFailure => formatter.write_str("datagen worker failed"),
        }
    }
}

impl Error for DatagenError {}

/// Strips BOM and CR/LF, returning `Ok(None)` for blank lines or canonical FEN on success.
pub fn normalize_opening_line(raw: &str) -> Result<Option<String>, DatagenError> {
    let without_bom = raw.strip_prefix('\u{FEFF}').unwrap_or(raw);
    let trimmed = without_bom.trim_matches(|ch: char| ch == '\r' || ch.is_whitespace());
    if trimmed.is_empty() {
        return Ok(None);
    }
    let board: Board = trimmed
        .parse()
        .map_err(|error: gwaymaegyi_core::FenError| {
            DatagenError::InvalidOpening(error.to_string())
        })?;
    Ok(Some(board.to_string()))
}

/// Computes the early-game `MultiPV` randomization threshold (0..=100) for `game_ply`.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Clamped threshold is always within 0.0..=100.0."
)]
pub fn multipv_threshold(game_ply: usize) -> u32 {
    if game_ply >= 30 {
        return 0;
    }
    let exponent = i32::try_from(game_ply).unwrap_or(0) - 5;
    let raw = (80.0 * 0.9_f64.powi(exponent)).round().clamp(0.0, 100.0);
    raw as u32
}

#[derive(Clone, Copy, Debug)]
struct SplitMix64(u64);

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }

    const fn next_u64(&mut self) -> u64 {
        let mut z = self.0;
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Modulo bounds the random index to the target slice length."
    )]
    const fn next_index(&mut self, len: usize) -> usize {
        if len <= 1 {
            0
        } else {
            (self.next_u64() as usize) % len
        }
    }
}

/// Runs seeded self-play games and collects up to `config.positions` quiet training records.
pub fn generate_training_data(
    config: &DatagenConfig,
    cancel: &AtomicBool,
) -> Result<Vec<TrainingRecord>, DatagenError> {
    if !(1..=100_000).contains(&config.positions) {
        return Err(DatagenError::InvalidPositions);
    }
    if !(1..=16).contains(&config.threads) {
        return Err(DatagenError::InvalidThreads);
    }
    if config.opt_nodes == 0
        || config.max_nodes < config.opt_nodes
        || config.max_depth == 0
        || config.max_depth > MAX_DEPTH
    {
        return Err(DatagenError::InvalidLimits);
    }
    let mut openings = Vec::new();
    for raw in &config.openings {
        if let Some(fen) = normalize_opening_line(raw)? {
            openings.push(fen);
        }
    }
    let next_game = AtomicUsize::new(0);
    let total_saved = AtomicUsize::new(0);
    let worker_count = usize::from(config.threads).min(config.positions);

    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            let openings_ref = &openings;
            let next_game_ref = &next_game;
            let saved_ref = &total_saved;
            handles.push(scope.spawn(move || {
                let mut engine = Engine::new().map_err(DatagenError::Engine)?;
                let mut base = Options::default();
                base.set_mode(Mode::Aggressive);
                base.set_chess960(config.chess960);
                base.set_hash_mib(1).map_err(DatagenError::Engine)?;
                engine.configure(base).map_err(DatagenError::Engine)?;
                let mut local_batches = Vec::new();

                while !cancel.load(Ordering::Relaxed)
                    && saved_ref.load(Ordering::Relaxed) < config.positions
                {
                    let game_id = next_game_ref.fetch_add(1, Ordering::Relaxed);
                    if game_id > config.positions.saturating_mul(64).max(256) {
                        break;
                    }
                    let records =
                        play_single_game(&mut engine, config, openings_ref, game_id, cancel)?;
                    if !records.is_empty() {
                        saved_ref.fetch_add(records.len(), Ordering::Relaxed);
                        local_batches.push((game_id, records));
                    }
                }
                Ok::<_, DatagenError>(local_batches)
            }));
        }
        let mut batches = Vec::new();
        let mut failure = None;
        for handle in handles {
            match handle.join() {
                Ok(Ok(worker_batches)) => batches.extend(worker_batches),
                Ok(Err(error)) => {
                    if failure.is_none() {
                        failure = Some(error);
                    }
                }
                Err(_) => {
                    if failure.is_none() {
                        failure = Some(DatagenError::WorkerFailure);
                    }
                }
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        batches.sort_by_key(|(game_id, _)| *game_id);
        let mut out = Vec::with_capacity(config.positions);
        for (_, batch) in batches {
            for record in batch {
                if out.len() < config.positions {
                    out.push(record);
                }
            }
        }
        Ok(out)
    })
}

fn play_random_opening(
    game: &mut Game,
    rng: &mut SplitMix64,
    plies: usize,
    chess960: bool,
) -> Option<Vec<String>> {
    let mut played_uci = Vec::with_capacity(128);
    for _ in 0..plies {
        let moves = game.board().legal_moves();
        if moves.is_empty() || game.outcome() != Outcome::Ongoing {
            return None;
        }
        let chosen = moves[rng.next_index(moves.len())];
        let uci = chosen.to_uci(chess960);
        game.play_uci(&uci, chess960).ok()?;
        played_uci.push(uci);
    }
    (game.outcome() == Outcome::Ongoing).then_some(played_uci)
}

fn play_single_game(
    engine: &mut Engine,
    config: &DatagenConfig,
    openings: &[String],
    game_id: usize,
    cancel: &AtomicBool,
) -> Result<Vec<TrainingRecord>, DatagenError> {
    let game_seed = u64::try_from(game_id)
        .unwrap_or(0)
        .wrapping_mul(0xD1B5_4A32_D192_ED03);
    let mut rng = SplitMix64::new(config.seed.wrapping_add(game_seed));
    let start_fen = if openings.is_empty() {
        START_FEN
    } else {
        &openings[rng.next_index(openings.len())]
    };
    let start_board: Board = start_fen
        .parse()
        .map_err(|err: gwaymaegyi_core::FenError| DatagenError::InvalidOpening(err.to_string()))?;
    let mut game = Game::new(start_board);
    let base_plies = if openings.is_empty() { 8 } else { 4 };
    let random_plies = base_plies + usize::try_from(rng.next_u64() & 1).unwrap_or(0);
    let Some(mut played_uci) =
        play_random_opening(&mut game, &mut rng, random_plies, config.chess960)
    else {
        return Ok(Vec::new());
    };
    let verify = search_game_move(engine, start_fen, &played_uci, config, 1, cancel)?;
    let verify_cap = if config.opt_nodes <= 512 { 900 } else { 400 };
    if verify.0.abs() > verify_cap {
        return Ok(Vec::new());
    }

    let mut samples: Vec<(Board, i16)> = Vec::new();
    let mut result = GameResult::Draw;
    for step in 0..320_usize {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        match game.outcome() {
            Outcome::Checkmate { winner } => {
                result = match winner {
                    Color::White => GameResult::WhiteWin,
                    Color::Black => GameResult::BlackWin,
                };
                break;
            }
            Outcome::Draw(_) => {
                result = GameResult::Draw;
                break;
            }
            Outcome::Ongoing => {}
        }
        if !game.claimable_draws().is_empty() {
            result = GameResult::Draw;
            break;
        }
        let game_ply = random_plies + step;
        let threshold = multipv_threshold(game_ply);
        let roll = u32::try_from(rng.next_u64() % 100).unwrap_or(0) + 1;
        let multi_pv = if roll > threshold && threshold > 20 {
            2
        } else {
            1
        };
        let (side_score, chosen) =
            search_game_move(engine, start_fen, &played_uci, config, multi_pv, cancel)?;
        let Some(chess_move) = chosen else {
            break;
        };
        let board = *game.board();
        let white_score = match board.side_to_move() {
            Color::White => side_score,
            Color::Black => -side_score,
        };
        if white_score.abs() > 1_000 || (game_ply > 200 && white_score.abs() > 200) {
            result = if white_score > 0 {
                GameResult::WhiteWin
            } else {
                GameResult::BlackWin
            };
            break;
        }
        let is_capture = board.is_capture(chess_move);
        let is_queen_promo = chess_move.kind() == MoveKind::Promotion(Promotion::Queen);
        let in_check = board.in_check(board.side_to_move());
        let quiet_phase = threshold <= 20 || config.opt_nodes <= 512;
        if !is_capture
            && !is_queen_promo
            && !in_check
            && quiet_phase
            && samples.len() < MAX_GAME_BUFFER
        {
            let clamped = i16::try_from(white_score.clamp(-30_000, 30_000)).unwrap_or(0);
            samples.push((board, clamped));
        }
        let uci = chess_move.to_uci(config.chess960);
        if game.play_uci(&uci, config.chess960).is_err() {
            break;
        }
        played_uci.push(uci);
    }
    Ok(samples
        .into_iter()
        .map(|(board, score)| TrainingRecord::new(board, score, result))
        .collect())
}

fn search_game_move(
    engine: &mut Engine,
    start_fen: &str,
    played_uci: &[String],
    config: &DatagenConfig,
    multi_pv: u8,
    cancel: &AtomicBool,
) -> Result<(i32, Option<gwaymaegyi_core::Move>), DatagenError> {
    let mut options = engine.options();
    options
        .set_multi_pv(multi_pv)
        .map_err(DatagenError::Engine)?;
    engine.configure(options).map_err(DatagenError::Engine)?;
    let move_refs: Vec<_> = played_uci.iter().map(String::as_str).collect();
    engine
        .set_position(start_fen, &move_refs)
        .map_err(DatagenError::Engine)?;
    engine
        .start(SearchLimits {
            depth: config.max_depth,
            nodes: config.max_nodes,
        })
        .map_err(DatagenError::Engine)?;
    while engine.searching() {
        if cancel.load(Ordering::Relaxed) {
            engine.stop();
            break;
        }
        let report = engine.step(256).map_err(DatagenError::Engine)?;
        if report.nodes >= config.opt_nodes && report.depth >= 1 {
            engine.stop();
            break;
        }
    }
    let report = engine.report();
    if multi_pv > 1 && report.variations.len() >= 2 {
        let second = &report.variations[1];
        return Ok((second.score_cp, second.moves.first().copied()));
    }
    Ok((report.score_cp.unwrap_or(0), report.best_move))
}
