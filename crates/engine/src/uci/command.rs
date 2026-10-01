use super::go::Go;
use gwaymaegyi_core::START_FEN;

#[derive(Debug)]
pub(super) enum Command {
    Uci,
    Ready,
    NewGame,
    Option { name: String, value: String },
    Position { fen: String, moves: Vec<String> },
    Go(Go),
    Stop,
    PonderHit,
    Quit,
    EndInput,
    Invalid(String),
    Ignore,
}
impl Command {
    pub(super) fn parse(line: &str) -> Self {
        match Self::decode(line) {
            Ok(command) => command,
            Err(error) => Self::Invalid(error),
        }
    }
    fn decode(line: &str) -> Result<Self, String> {
        let fields: Vec<_> = line.split_whitespace().collect();
        let Some(first) = fields.first() else {
            return Ok(Self::Ignore);
        };
        match *first {
            "uci" => Ok(Self::Uci),
            "isready" => Ok(Self::Ready),
            "ucinewgame" => Ok(Self::NewGame),
            "quit" => Ok(Self::Quit),
            "stop" => Ok(Self::Stop),
            "ponderhit" => Ok(Self::PonderHit),
            "setoption" => Self::option(&fields),
            "position" => Self::position(&fields),
            "go" => Go::parse(&fields[1..]).map(Self::Go),
            _ => Ok(Self::Ignore),
        }
    }
    fn option(fields: &[&str]) -> Result<Self, String> {
        if fields.get(1) != Some(&"name") {
            return Err("setoption requires name".into());
        }
        let separator = fields
            .iter()
            .position(|&field| field == "value")
            .unwrap_or(fields.len());
        if separator <= 2 {
            return Err("setoption requires an option name".into());
        }
        let name = fields[2..separator].join(" ");
        let value = fields.get(separator + 1..).unwrap_or_default().join(" ");
        Ok(Self::Option { name, value })
    }
    fn position(fields: &[&str]) -> Result<Self, String> {
        let (fen, offset) = match fields.get(1) {
            Some(&"startpos") => (START_FEN.to_owned(), 2),
            Some(&"fen") if fields.len() >= 8 => (fields[2..8].join(" "), 8),
            _ => return Err("position requires startpos or a complete FEN".into()),
        };
        let moves = if fields.len() == offset {
            Vec::new()
        } else if fields.get(offset) == Some(&"moves") {
            fields[offset + 1..]
                .iter()
                .map(|item| (*item).to_owned())
                .collect()
        } else {
            return Err("unexpected position suffix".into());
        };
        Ok(Self::Position { fen, moves })
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use gwaymaegyi_search::MAX_DEPTH;
    #[test]
    fn parsing_handles_limits_and_bad_commands() {
        assert!(matches!(
            Command::parse("position fen not a fen"),
            Command::Invalid(_)
        ));
        assert!(matches!(Command::parse("go depth 0"), Command::Invalid(_)));
        assert!(matches!(Command::parse("go nodes -1"), Command::Invalid(_)));
        assert!(
            matches!(Command::parse("go depth 10"),Command::Go(go) if go.limits.depth==10 && go.budget_ms(0,20).is_none())
        );
        assert!(
            matches!(Command::parse("go wtime 18446744073709551615"),Command::Go(go) if go.budget_ms(0,20).is_some_and(|time|time<=86_400_000))
        );
        assert!(
            matches!(Command::parse("go infinite"),Command::Go(go) if go.limits.depth==MAX_DEPTH)
        );
    }
}
