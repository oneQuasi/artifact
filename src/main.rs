use std::{io, process, thread, time::Duration};

use chessing::{chess::Chess, game::{GameTemplate, Team}, uci::{parse::{GoOption, UciCommand, UciPosition}, respond::Info, Uci}};
use search::{iterative_deepening, search, SearchInfo};

mod search;
mod util;
mod eval;

fn main() {
    let uci = Uci;
    let stdin = io::stdin();

    let chess = Chess::create::<u64>();
    let mut board = chess.default();

    let mut hashes: Vec<u64> = vec![];
    let zobrist = board.game.processor.gen_zobrist(&mut board, 64);

    for line in stdin.lines() {
        let line = line.expect("Line is set");

        match uci.parse(&line) {
            UciCommand::Uci() => {
                uci.uciok();
            }
            UciCommand::Go { options } => {
                let mut soft_time = 0;
                let team = board.state.moving_team;
                
                for option in options {
                    match option {
                        GoOption::BTime(time) => {
                            if team == Team::Black {
                                soft_time += time / 300;
                            }
                        }
                        GoOption::BInc(inc) => {
                            soft_time += inc / 30;
                        }
                        GoOption::WTime(time) => {
                            if team == Team::White {
                                soft_time += time / 300;
                            }
                        }
                        GoOption::WInc(inc) => {
                            if team == Team::White {
                                soft_time += inc / 30;
                            }
                        }
                        GoOption::MoveTime(time) => {
                            soft_time += time / 10;
                        }
                        _ => {}
                    }
                }

                if soft_time == 0 {
                    soft_time = 300;
                }

                let zobrist = board.game.processor.gen_zobrist(&mut board, 64);
                let info = iterative_deepening(&uci, &mut board, soft_time, zobrist, hashes.clone());

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

                hashes = vec![];

                for act in moves {
                    hashes.push(chess.processor.hash(&mut board, &zobrist));

                    board.play_action(&act);
                }
            }
            UciCommand::Quit() => {
                process::exit(0x100);
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
