//! Protocol-safe scores, principal variations, best moves, and ponder hints.

use gwaymaegyi_search::SearchReport;

pub(super) fn best(report: &SearchReport, chess960: bool, ponder: bool) -> String {
    let hint = if ponder {
        report
            .variations
            .iter()
            .find(|line| line.moves.first().copied() == report.best_move)
            .and_then(|line| line.moves.get(1))
    } else {
        None
    };
    let suffix = hint.map_or_else(String::new, |chess_move| {
        format!(" ponder {}", chess_move.to_uci(chess960))
    });
    format!(
        "bestmove {}{suffix}",
        report
            .best_move
            .map_or_else(|| "0000".into(), |chess_move| chess_move.to_uci(chess960))
    )
}

pub(super) fn info(report: &SearchReport, chess960: bool, elapsed: u128, count: u8) -> Vec<String> {
    let nps = u128::from(report.nodes).saturating_mul(1000) / elapsed.max(1);
    if report.variations.is_empty() {
        let score = report.score_cp.map_or_else(String::new, |value| {
            report.mate_in().map_or_else(
                || format!(" score cp {value}"),
                |mate| format!(" score mate {mate}"),
            )
        });
        return vec![format!(
            "info depth {} seldepth {}{score} nodes {} nps {nps} time {elapsed}",
            report.depth, report.selective_depth, report.nodes
        )];
    }
    let chosen = report
        .variations
        .iter()
        .find(|line| line.moves.first().copied() == report.best_move);
    chosen.into_iter().chain(report.variations.iter().filter(|line| line.moves.first().copied()!=report.best_move))
        .take(usize::from(count)).enumerate().map(|(index,line)| {
            let score=line.mate_in().map_or_else(||format!("cp {}",line.score_cp),|mate|format!("mate {mate}"));
            let pv=line.moves.iter().map(|chess_move|chess_move.to_uci(chess960)).collect::<Vec<_>>().join(" ");
            format!("info depth {} seldepth {} multipv {} score {score} nodes {} nps {nps} time {elapsed} pv {pv}",report.depth,report.selective_depth,index+1,report.nodes)
        }).collect()
}
