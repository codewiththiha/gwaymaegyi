//! Search-limit parsing and checked native clock allocation for go requests.

use gwaymaegyi_search::{MAX_DEPTH, SearchLimits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PlayState {
    Normal,
    Infinite,
    Ponder,
}

#[derive(Debug)]
pub(super) struct Go {
    pub limits: SearchLimits,
    pub state: PlayState,
    pub roots: Vec<String>,
    remaining: [Option<u64>; 2],
    increments: [u64; 2],
    move_time: Option<u64>,
    horizon: u64,
}
impl Go {
    pub(super) fn parse(fields: &[&str]) -> Result<Self, String> {
        let mut go = Self {
            limits: SearchLimits {
                depth: MAX_DEPTH,
                nodes: u64::MAX,
            },
            state: PlayState::Normal,
            roots: Vec::new(),
            remaining: [None; 2],
            increments: [0; 2],
            move_time: None,
            horizon: 25,
        };
        let mut index = 0;
        while let Some(&field) = fields.get(index) {
            index += 1;
            match field {
                "infinite" => {
                    if go.state != PlayState::Ponder {
                        go.state = PlayState::Infinite;
                    }
                    continue;
                }
                "ponder" => {
                    go.state = PlayState::Ponder;
                    continue;
                }
                "searchmoves" => {
                    while fields.get(index).is_some_and(|token| !Self::keyword(token)) {
                        go.roots.push(fields[index].to_owned());
                        index += 1;
                    }
                    if go.roots.is_empty() {
                        return Err("searchmoves requires a legal root move".into());
                    }
                    continue;
                }
                _ => {}
            }
            let value: u64 = fields
                .get(index)
                .ok_or_else(|| format!("{field} requires an integer"))?
                .parse()
                .map_err(|_| format!("invalid {field}"))?;
            index += 1;
            match field {
                "depth" => go.limits.depth = u8::try_from(value).map_err(|_| "invalid depth")?,
                "nodes" => go.limits.nodes = value,
                "mate" if value > 0 => {
                    go.limits.depth =
                        u8::try_from(value.saturating_mul(2).min(u64::from(MAX_DEPTH)))
                            .map_err(|_| "invalid mate limit")?;
                }
                "wtime" => go.remaining[0] = Some(value),
                "btime" => go.remaining[1] = Some(value),
                "winc" => go.increments[0] = value,
                "binc" => go.increments[1] = value,
                "movetime" => go.move_time = Some(value),
                "movestogo" if value > 0 => go.horizon = value,
                _ => return Err(format!("unsupported go field: {field}")),
            }
        }
        go.limits.validate().map_err(|error| error.to_string())?;
        Ok(go)
    }
    fn keyword(value: &str) -> bool {
        matches!(
            value,
            "depth"
                | "nodes"
                | "mate"
                | "wtime"
                | "btime"
                | "winc"
                | "binc"
                | "movetime"
                | "movestogo"
                | "infinite"
                | "ponder"
                | "searchmoves"
        )
    }
    /// Hard and soft budgets in milliseconds, or none when clock data is absent.
    /// Ponder budgets are computed here but start only after `ponderhit`.
    /// The reference formula: max is 80% of usable time, opt is 60% of
    /// per-move time plus the increment.
    pub(super) fn time_control(&self, side: usize, overhead: u64) -> Option<(u64, u64)> {
        if let Some(time) = self.move_time {
            let budget = time.saturating_sub(overhead).max(1);
            return Some((budget, budget));
        }
        let time = self.remaining[side]?;
        let usable = time.saturating_sub(overhead);
        let max = (usable / 10 * 8 + usable % 10 * 8 / 10).max(1);
        let per_move = u128::from(usable / 20) + u128::from(self.increments[side]);
        let opt = (per_move * 6 / 10).max(1).min(u128::from(max));
        // The cap precedes narrowing, including when time plus increment exceeds u64.
        let opt = u64::try_from(opt).unwrap_or(max);
        Some((max, opt))
    }

    pub(super) const fn adaptive_time(&self) -> bool {
        self.move_time.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::Go;

    #[test]
    fn ponder_clocks_are_deferred_and_movetime_stays_fixed() -> Result<(), String> {
        let ponder = Go::parse(&["ponder", "wtime", "20000", "winc", "1000"])?;
        assert_eq!(ponder.time_control(0, 20), Some((15_984, 1_199)));
        assert!(ponder.adaptive_time());

        let movetime = Go::parse(&["movetime", "5000"])?;
        assert_eq!(movetime.time_control(0, 20), Some((4_980, 4_980)));
        assert!(!movetime.adaptive_time());
        Ok(())
    }

    #[test]
    fn native_clock_budgets_have_no_one_day_ceiling() -> Result<(), String> {
        let fixed = Go::parse(&["movetime", "18446744073709551615"])?;
        assert_eq!(fixed.time_control(0, 0), Some((u64::MAX, u64::MAX)));
        let clock = Go::parse(&[
            "wtime",
            "18446744073709551615",
            "winc",
            "18446744073709551615",
        ])?;
        let (max, opt) = clock.time_control(0, 20).ok_or("missing large clock")?;
        assert_eq!(max, 14_757_395_258_967_641_276);
        assert!(opt > 86_400_000 && opt <= max);
        Ok(())
    }
}
