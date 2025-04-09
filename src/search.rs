use std::{cmp::Ordering, i32};

use chessing::{bitboard::{BitBoard, BitInt}, game::{action::Action, Board, GameState, Team}, uci::{respond::Info, Uci}};

use crate::{eval::{eval, MATERIAL}, util::current_time_millis};

pub struct SearchInfo {
    pub root_depth: i32,
    pub best_move: Option<Action>,
    pub nodes: u64,
    pub score: i32
}

pub const MAX: i32 = 1_000_000;
pub const MIN: i32 = -1_000_000;

fn mvv_lva<T: BitInt>(board: &mut Board<T>, action: Action, opps: BitBoard<T>) -> i32 {
    if !is_capture(board, action, opps) {
        return -10000;
    }

    let attacker_type = board.state.mailbox[action.from as usize] - 1;
    let victim_type = board.state.mailbox[action.to as usize] - 1;

    let attacker_value = MATERIAL[attacker_type as usize];
    let victim_value = MATERIAL[victim_type as usize];

    victim_value - attacker_value
}   

fn is_capture<T: BitInt>(board: &mut Board<T>, action: Action, opps: BitBoard<T>) -> bool {
    let to_idx = action.to as usize;
    if board.state.mailbox[to_idx] == 0 {
        return false;
    }

    return BitBoard::index(action.to).and(opps).is_set();
}

pub fn search<T: BitInt>(
    board: &mut Board<T>, 
    info: &mut SearchInfo,
    depth: i32,
    ply: i32,
    mut alpha: i32, 
    beta: i32, 
) -> i32 {
    if depth == 0 {
        return eval(board);
    }

    let mut legal_actions = board.list_legal_actions();
    let opps = board.state.opposite_team();

    match board.game_state(&legal_actions) {
        GameState::Win(Team::White) => {
            return MIN + ply;
        }
        GameState::Win(Team::Black) => {
            return MIN + ply;
        }
        GameState::Draw => {
            return 0;
        }
        GameState::Ongoing => {
            // continue evaluation
        }
    }

    legal_actions.sort_by(|&a, &b| {
        mvv_lva(board, b, opps).cmp(&mvv_lva(board, a, opps))
    });

    let mut max = i32::MIN;
    let mut best_move: Option<Action> = None;

    for act in legal_actions {
        let history = board.play(act);

        info.nodes += 1;

        let score = -search(board, info, depth - 1, ply + 1, -beta, -alpha);
        board.state.restore(history);

        if score > max {
            max = score;
            best_move = Some(act);

            if score > alpha {
                alpha = score;
            }
        }

        if score >= beta {
            break;
        }
    }

    if depth == info.root_depth {
        info.best_move = best_move;
    }

    max
}

pub fn iterative_deepening<T: BitInt>(uci: &Uci, board: &mut Board<T>, max_time: u64) -> SearchInfo {
    let mut info = SearchInfo {
        root_depth: 0,
        best_move: None,
        nodes: 0,
        score: 0
    };

    for depth in 1..100 {
        let start = current_time_millis();

        info.root_depth = depth;
        let score = search(board, &mut info, depth, 0, MIN, MAX);
        info.score = score;

        let end = current_time_millis();

        let mut time = (end - start) as u64;
        if time == 0 { time = 1; }

        uci.info(Info {
            depth: Some(depth as u32),
            score_cp: Some(info.score),
            time: Some(time),
            nodes: Some(info.nodes),
            nps: Some(info.nodes / time * 1000),
            pv: info.best_move.map(|action| vec![ board.display_uci_action(action) ]),
            ..Default::default()
        });

        if time > max_time {
            break;   
        }
    }

    info
}