use gwaymaegyi_search::Engine;

#[derive(Debug)]
pub(super) struct ProtocolOptions {
    pub overhead: u64,
    elo: u16,
    limited: bool,
}
impl Default for ProtocolOptions {
    fn default() -> Self {
        Self {
            overhead: 20,
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
            "seed" => options.set_seed(value.parse().map_err(|_| "invalid seed")?),
            "move overhead" => {
                overhead = value.parse().map_err(|_| "invalid move overhead")?;
                if overhead > 5000 {
                    return Err("move overhead must be 0 through 5000".into());
                }
            }
            "ponder" => {
                let _: bool = value.parse().map_err(|_| "invalid ponder flag")?;
            }
            _ => return Err(format!("unknown option: {name}")),
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
        Ok(())
    }
}

pub(super) const IDENTIFICATION: &str = "id name gwaymaegyi 0.1.0\nid author codewiththiha\n\
option name Hash type spin default 8 min 1 max 64\n\
option name MultiPV type spin default 1 min 1 max 5\n\
option name Mode type combo default balanced var balanced var aggressive var human-like var analysis\n\
option name UCI_Chess960 type check default false\n\
option name UCI_LimitStrength type check default false\n\
option name UCI_Elo type spin default 1500 min 500 max 3000\n\
option name Seed type string default 19\n\
option name Move Overhead type spin default 20 min 0 max 5000\n\
option name Ponder type check default false\n\
info string Elo targets are uncalibrated resource and error-tolerance presets\nuciok";
