use std::i32;

use chessing::{bitboard::BitInt, game::{action::Action, Board, GameState, Team}, uci::{respond::Info, Uci}};

use crate::util::current_time_millis;

pub struct SearchInfo {
    pub root_depth: i32,
    pub best_move: Option<Action>,
    pub nodes: u64,
    pub score: i32
}

fn team_to_move<T: BitInt>(board: &mut Board<T>) -> i32 {
    match board.state.moving_team {
        Team::White => 1,
        Team::Black => -1
    }
}

pub const PAWN: i32 = 100;
pub const KNIGHT: i32 = 305;
pub const BISHOP: i32 = 333;
pub const ROOK: i32 = 563;
pub const QUEEN: i32 = 950;

fn eval<T: BitInt>(board: &mut Board<T>) -> i32 {
    let mut score = 0;

    let pawns = board.state.pieces[0];
    let knights = board.state.pieces[1];
    let bishops = board.state.pieces[2];
    let rooks = board.state.pieces[3];
    let queens = board.state.pieces[4];

    let white = board.state.white;
    let black = board.state.black;

    let white_pawns = pawns.and(white).count() as i32;
    let black_pawns = pawns.and(black).count() as i32;

    let white_knights = knights.and(white).count() as i32;
    let black_knights = knights.and(black).count() as i32;

    let white_bishops = bishops.and(white).count() as i32;
    let black_bishops = bishops.and(black).count() as i32;

    let white_rooks = rooks.and(white).count() as i32;
    let black_rooks = rooks.and(black).count() as i32;

    let white_queens = queens.and(white).count() as i32;
    let black_queens = queens.and(black).count() as i32;

    score += (white_pawns - black_pawns) * PAWN;
    score += (white_knights - black_knights) * KNIGHT;
    score += (white_bishops - black_bishops) * BISHOP;
    score += (white_rooks - black_rooks) * ROOK;
    score += (white_queens - black_queens) * QUEEN;

    score *= team_to_move(board);

    score
}

pub const MAX: i32 = 1_000_000;
pub const MIN: i32 = -1_000_000;

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

    let legal_actions = board.list_legal_actions();

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