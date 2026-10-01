//! UCI option validation and protocol discovery, committed atomically to the engine.

use gwaymaegyi_search::{Behavior, Engine, Parameter, SkillLevel};

#[derive(Debug)]
pub(super) struct ProtocolOptions {
    pub overhead: u64,
    pub ponder: bool,
    elo: u16,
    limited: bool,
}
impl Default for ProtocolOptions {
    fn default() -> Self {
        Self {
            overhead: 20,
            ponder: false,
            elo: 1500,
            limited: false,
        }
    }
}
impl ProtocolOptions {
    pub(super) fn set(
        &mut self,
        engine: &mut Engine,
        name: &str,
        value: &str,
    ) -> Result<(), String> {
        let mut options = engine.options();
        let mut elo = self.elo;
        let mut limited = self.limited;
        let mut overhead = self.overhead;
        let mut ponder = self.ponder;
        match name.to_ascii_lowercase().as_str() {
            "hash" => options
                .set_hash_mib(value.parse().map_err(|_| "invalid hash size")?)
                .map_err(|error| error.to_string())?,
            "multipv" => options
                .set_multi_pv(value.parse().map_err(|_| "invalid MultiPV")?)
                .map_err(|error| error.to_string())?,
            "mode" => options.set_mode(
                value
                    .parse()
                    .map_err(|error: gwaymaegyi_search::EngineError| error.to_string())?,
            ),
            "uci_chess960" => {
                options.set_chess960(value.parse().map_err(|_| "invalid Chess960 flag")?);
            }
            "uci_limitstrength" => limited = value.parse().map_err(|_| "invalid strength flag")?,
            "uci_elo" => {
                elo = value.parse().map_err(|_| "invalid Elo target")?;
                if !(500..=3000).contains(&elo) {
                    return Err("Elo must be 500 through 3000".into());
                }
            }
            "skill_level" => {
                let level = SkillLevel::new(value.parse().map_err(|_| "invalid skill level")?)
                    .map_err(|error| error.to_string())?;
                limited = level != SkillLevel::FULL;
                if let Some(target) = level.nominal_elo() {
                    elo = target;
                }
            }
            "seed" => options.set_seed(value.parse().map_err(|_| "invalid seed")?),
            "move overhead" => {
                overhead = value.parse().map_err(|_| "invalid move overhead")?;
                if overhead > 5000 {
                    return Err("move overhead must be 0 through 5000".into());
                }
            }
            "ponder" => {
                ponder = value.parse().map_err(|_| "invalid ponder flag")?;
            }
            _ => {
                let mut tuning = options.tuning();
                if let Some(key) = name.strip_prefix("Behavior ") {
                    tuning.set_behavior(
                        Behavior::parse(key).map_err(|error| error.to_string())?,
                        value.parse().map_err(|_| "invalid behavior flag")?,
                    );
                } else {
                    tuning
                        .set(
                            Parameter::parse(name).map_err(|error| error.to_string())?,
                            value.parse().map_err(|_| "invalid parameter value")?,
                        )
                        .map_err(|error| error.to_string())?;
                }
                options.set_tuning(tuning);
            }
        }
        options
            .set_elo(if limited { elo } else { 0 })
            .map_err(|error| error.to_string())?;
        engine
            .configure(options)
            .map_err(|error| error.to_string())?;
        self.elo = elo;
        self.limited = limited;
        self.overhead = overhead;
        self.ponder = ponder;
        Ok(())
    }
}

pub(super) const IDENTIFICATION: &str = concat!(
    "id name gwaymaegyi ",
    env!("CARGO_PKG_VERSION"),
    "\nid author codewiththiha\n\
option name Hash type spin default 8 min 1 max 64\n\
option name SyzygyPath type string default\n\
option name MultiPV type spin default 1 min 1 max 5\n\
option name Mode type combo default balanced var balanced var aggressive var human-like var analysis\n\
option name UCI_Chess960 type check default false\n\
option name UCI_LimitStrength type check default false\n\
option name UCI_Elo type spin default 1500 min 500 max 3000\n\
option name Skill_Level type spin default 21 min 1 max 21\n\
option name Seed type string default 19\n\
option name Move Overhead type spin default 20 min 0 max 5000\n\
option name Ponder type check default false\n\
info string Elo targets are uncalibrated resource and error-tolerance presets\nuciok"
);

#[cfg(test)]
mod tests {
    use super::ProtocolOptions;
    use gwaymaegyi_search::{Engine, Strength};
    use std::error::Error;
    #[test]
    fn skill_options_commit_the_mapped_strength() -> Result<(), Box<dyn Error>> {
        let mut engine = Engine::new()?;
        let mut options = ProtocolOptions::default();
        options.set(&mut engine, "Skill_Level", "10")?;
        assert_eq!(engine.options().strength(), Strength::Approximate(1800));
        assert!(options.set(&mut engine, "Skill_Level", "22").is_err());
        assert_eq!(engine.options().strength(), Strength::Approximate(1800));
        options.set(&mut engine, "Skill_Level", "21")?;
        assert_eq!(engine.options().strength(), Strength::Full);
        Ok(())
    }
}
