use std::{io, thread, time::Duration};

use chessing::{chess::Chess, game::{GameTemplate, Team}, uci::{parse::{GoOption, UciCommand, UciPosition}, respond::Info, Uci}};
use fastrand::choice;
use search::{iterative_deepening, search, SearchInfo};

mod search;
mod util;

fn main() {
    let uci = Uci;
    let stdin = io::stdin();

    let chess = Chess::create::<u64>();
    let mut board = chess.default();

    for line in stdin.lines() {
        let line = line.expect("Line is set");

        match uci.parse(&line) {
            UciCommand::Uci() => {
                uci.uciok();
            }
            UciCommand::Go { options } => {
                let mut max_time = None::<u64>;
                let team = board.state.moving_team;
                
                for option in options {
                    match option {
                        GoOption::BTime(time) => {
                            if team == Team::Black {
                                max_time = max_time.map(|el| el + (time / 300)).or(Some(time / 300));
                            }
                        }
                        GoOption::BInc(inc) => {
                            if team == Team::Black {
                                max_time = max_time.map(|el| el + (inc / 30)).or(Some(inc / 30));
                            }
                        }
                        GoOption::WTime(time) => {
                            if team == Team::White {
                                max_time = max_time.map(|el| el + (time / 300)).or(Some(time / 300));
                            }
                        }
                        GoOption::WInc(inc) => {
                            if team == Team::White {
                                max_time = max_time.map(|el| el + (inc / 30)).or(Some(inc / 30));
                            }
                        }
                        _ => {}
                    }
                }

                let max_time = max_time.unwrap_or(300);

                let info = iterative_deepening(&uci, &mut board, max_time);

                let action = info.best_move.expect("There's a best move, right?");
                let action_display = board.display_uci_action(action);

                uci.bestmove(&action_display);
            }
            UciCommand::IsReady() => {
                uci.readyok();
            }
            UciCommand::Position { position, moves } => {
                match position {
                    UciPosition::Fen(fen) => {
                        board = chess.load(&fen);
                    } 
                    UciPosition::Startpos => {
                        board = chess.default();
                    }
                }

                for act in moves {
                    board.play_action(&act);
                }
            }
            UciCommand::Quit() => {
                // TODO
            }
            UciCommand::Stop() => {
                // TODO
            }
            UciCommand::UciNewGame() => {
                // TODO
            }
            UciCommand::Unknown(cmd) => {
                // TODO
            }
        }
    }
}
