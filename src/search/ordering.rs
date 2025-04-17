use chessing::{bitboard::{BitBoard, BitInt}, game::{action::{Action, ActionRecord}, zobrist::ZobristTable, Board, Team}};

use crate::eval::MATERIAL;

use super::{is_noisy, SearchInfo, TtEntry};

// [team][sq][sq]
pub type History = Vec<Vec<Vec<i32>>>;

// [team][piece][sq][team][piece][sq]
pub type ContinuationHistory = Vec<Vec<Vec<Vec<Vec<Vec<i32>>>>>>;

#[derive(Clone, Debug, Copy)]
pub struct ScoredAction(pub Action, pub i32);

pub fn mvv_lva<T: BitInt, const N: usize>(
    board: &mut Board<T, N>, 
    action: Action,
) -> i32 {
    let mut score = 1000;
    if action.piece == 0 && action.info >= 3 {
        // Pawn Promotion
        score += MATERIAL[(action.info - 2) as usize] - MATERIAL[0];
    }

    if let Some(victim_type) = board.piece_at(action.to) {
        if let Some(attacker_type) = board.piece_at(action.from) {
            let attacker_value = MATERIAL[attacker_type as usize];
            let victim_value = MATERIAL[victim_type as usize];

            score += victim_value - attacker_value;
        }
    }

    score
}   

pub const MAX_HISTORY: i32 = 300;
pub const MIN_HISTORY: i32 = -MAX_HISTORY;

pub fn history_bonus(depth: i32) -> i32 {
    depth * depth
}

pub fn update_history(history: &mut History, team: Team, action: Action, bonus: i32) {
    let from = action.from as usize;
    let to = action.to as usize;
    let clamped_bonus = bonus.clamp(MIN_HISTORY, MAX_HISTORY);

    history[team.index()][from][to]
        += clamped_bonus - history[team.index()][from][to] * clamped_bonus.abs() / MAX_HISTORY;
}

pub fn update_conthist(conthist: &mut ContinuationHistory, prio: Team, previous: Action, team: Team, action: Action, bonus: i32) {
    let prio_piece = previous.piece as usize;
    let prio_to = previous.to as usize;

    let piece = action.piece as usize;
    let to = action.to as usize;
    let clamped_bonus = bonus.clamp(MIN_HISTORY, MAX_HISTORY);

    conthist[prio.index()][prio_piece][prio_to][team.index()][piece][to]
        += clamped_bonus - conthist[prio.index()][prio_piece][prio_to][team.index()][piece][to] * clamped_bonus.abs() / MAX_HISTORY;
}

pub const HIGH_PRIORITY: i32 = 2i32.pow(28);
pub const MAX_KILLERS: usize = 2;

pub fn get_history<T: BitInt, const N: usize>(
    board: &mut Board<T, N>, 
    info: &mut SearchInfo,
    act: Action, 
    previous: Option<Action>,
    two_ply: Option<Action>,
    noisy: bool
) -> i32 {
    let to = act.to as usize;
    let from = act.from as usize;
    let piece = act.piece as usize;

    let team = board.state.moving_team;

    if noisy {
        info.capture_history[team.index()][from][to]
    } else {
        let mut history = info.history[team.index()][from][to];
        if let Some(previous) = previous {
            history += info.conthist[team.next().index()][previous.piece as usize][previous.to as usize][team.index()][piece][to] / 2;
        }
        if let Some(previous) = two_ply {
            history += info.conthist[team.index()][previous.piece as usize][previous.to as usize][team.index()][piece][to] / 2;
        }

        history
    }
}

pub fn score<T: BitInt, const N: usize>(
    board: &mut Board<T, N>, 
    info: &mut SearchInfo,
    ply: usize,
    act: Action, 
    previous: Option<Action>,
    two_ply: Option<Action>,
    found_best_move: Option<Action>
) -> i32 {
    if let Some(found_best_move) = found_best_move {
        if found_best_move == act {
            return HIGH_PRIORITY * 2;
        }
    }
    
    if is_noisy(board, act) {
        return HIGH_PRIORITY + mvv_lva(board, act) + get_history(board, info, act, previous, two_ply, true);
    }

    let mut score = get_history(board, info, act, previous, two_ply, false);

    for i in 0..MAX_KILLERS {
        let killer = info.killers[i][ply];
        if killer == Some(act) {
            score += 100 - (50 * (i as i32));
        }
    }

    score
}

pub fn qs_score<T: BitInt, const N: usize>(
    board: &mut Board<T, N>, 
    info: &mut SearchInfo,
    act: Action
) -> i32 {
    let mut score = 0;
    let to = act.to as usize;
    let from = act.from as usize;
    let piece = act.piece as usize;

    let team = board.state.moving_team;

    score += mvv_lva(board, act);
    score += info.capture_history[team.index()][from][to];

    score
}

pub fn sort_actions<T: BitInt, const N: usize>(
    board: &mut Board<T, N>, 
    info: &mut SearchInfo,
    ply: usize,
    actions: Vec<Action>,
    previous: Option<Action>,
    two_ply: Option<Action>,
    found_best_move: Option<Action>
) -> Vec<ScoredAction> {
    let mut scored = vec![];
    for act in actions {
        scored.push(ScoredAction(act, score(board, info, ply, act, previous, two_ply, found_best_move)))
    }

    scored.sort_by(|a, b| b.1.cmp(&a.1));

    scored
}

pub fn sort_qs_actions<T: BitInt, const N: usize>(
    board: &mut Board<T, N>, 
    info: &mut SearchInfo,
    actions: Vec<Action>
) -> Vec<ScoredAction> {
    let mut scored = vec![];
    for act in actions {
        scored.push(ScoredAction(act, mvv_lva(board, act)))
    }

    scored.sort_by(|a, b| b.1.cmp(&a.1));

    scored
}