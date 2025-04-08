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

fn eval<T: BitInt>(board: &mut Board<T>) -> i32 {
    0
}

pub fn search<T: BitInt>(board: &mut Board<T>, info: &mut SearchInfo, depth: i32) -> i32 {
    if depth == 0 {
        return eval(board);
    }

    let mut max = i32::MIN;
    let mut best_move: Option<Action> = None;

    let legal_actions = board.list_legal_actions();

    match board.game_state(&legal_actions) {
        GameState::Win(Team::White) => {
            return i32::MAX * team_to_move(board);
        }
        GameState::Win(Team::Black) => {
            return i32::MIN * team_to_move(board);
        }
        GameState::Draw => {
            return 0;
        }
        GameState::Ongoing => {
            // continue evaluation
        }
    }

    for act in legal_actions {
        let history = board.play(act);

        info.nodes += 1;

        let score = -search(board, info, depth - 1);
        board.state.restore(history);

        if score > max {
            max = score;
            best_move = Some(act);
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
        let score = search(board, &mut info, depth);
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