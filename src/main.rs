use std::{io, process, thread, time::Duration};

use chessing::{chess::Chess, game::{GameTemplate, Team}, uci::{parse::{GoOption, UciCommand, UciPosition}, respond::Info, Uci}};
use search::{create_search_info, iterative_deepening, search, SearchInfo};
use util::{current_time_millis, BENCH};

mod search;
mod util;
mod eval;

fn main() {
    let uci = Uci { log: true };
    let stdin = io::stdin();

    let chess = Chess::create::<u64, 6>();
    let mut board = chess.default();

    let mut info = create_search_info(&mut board);

    for line in stdin.lines() {
        let line = line.expect("Line is set");

        match uci.parse(&line) {
            UciCommand::Uci() => {
                uci.uciok();
            }
            UciCommand::Go { options } => {
                let mut soft_time = 0;
                let mut hard_time = 0;
                let team = board.state.moving_team;
                
                for option in options {
                    match option {
                        GoOption::BTime(time) => {
                            if team == Team::Black {
                                soft_time += time / 40;
                                hard_time += time / 9;
                            }
                        }
                        GoOption::BInc(inc) => {
                            soft_time += inc / 4;
                        }
                        GoOption::WTime(time) => {
                            if team == Team::White {
                                soft_time += time / 40;
                                hard_time += time / 9;
                            }
                        }
                        GoOption::WInc(inc) => {
                            if team == Team::White {
                                soft_time += inc / 4;
                            }
                        }
                        GoOption::MoveTime(time) => {
                            soft_time += time / 2;
                            hard_time += time;
                        }
                        _ => {}
                    }
                }

                if soft_time == 0 {
                    soft_time = 300;
                }

                iterative_deepening(&uci, &mut info, &mut board, search::SearchLimit::Time { soft: soft_time, hard: hard_time }, true);

                let action = info.best_move.expect("There's a best move, right?");
                let action_display = board.display_uci_action(action);

                uci.bestmove(&action_display);

                info.best_move = None;
            }
            UciCommand::Bench() => {
                let depth = 9;
                let mut total_nodes = 0;
                let start = current_time_millis();
                for (ind, pos) in BENCH.iter().enumerate() {
                    info.nodes = 0;
                    let mut board = chess.load(&pos);

                    iterative_deepening(&uci, &mut info, &mut board, search::SearchLimit::Depth(depth), false);
                    println!("[{}] NODES: {} | BEST: {}", ind, info.nodes, board.display_uci_action(info.best_move.expect("Must have a best move")));

                    total_nodes += info.nodes;
                }
                let end = current_time_millis();
                let time = (end - start) as u64;
                println!("[#] NODES: {} | TIME: {}ms | NPS: {}", total_nodes, time, total_nodes / time * 1000)
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

                info.hashes = vec![];

                for act in moves {
                    info.hashes.push(chess.rules.hash(&mut board, &info.zobrist));
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
                info = create_search_info(&mut board);
            }
            UciCommand::Unknown(cmd) => {
                // TODO
            }
        }
    }
}
