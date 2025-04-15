use chessing::{bitboard::{BitBoard, BitInt}, game::{action::{Action, ActionRecord}, zobrist::ZobristTable, Board, Team}};

use crate::eval::MATERIAL;

use super::{is_capture, SearchInfo, TtEntry};

// [team][sq][sq]
pub type History = Vec<Vec<Vec<i32>>>;

// [team][piece][sq][team][piece][sq]
pub type ContinuationHistory = Vec<Vec<Vec<Vec<Vec<Vec<i32>>>>>>;

#[derive(Clone, Debug, Copy)]
pub struct ScoredAction(pub Action, pub i32);

pub fn mvv_lva<T: BitInt>(
    board: &mut Board<T>, 
    action: Action,
) -> i32 {
    let attacker_type = board.state.mailbox[action.from as usize] - 1;
    let victim_type = board.state.mailbox[action.to as usize] - 1;

    let attacker_value = MATERIAL[attacker_type as usize];
    let victim_value = MATERIAL[victim_type as usize];

    1000 + (victim_value - attacker_value)
}   

pub fn update_history(history: &mut History, team: Team, action: Action, bonus: i32) {
    let from = action.from as usize;
    let to = action.to as usize;
    let clamped_bonus = bonus.clamp(-300, 300);

    history[team.index()][from][to]
        += clamped_bonus - history[team.index()][from][to] * clamped_bonus.abs() / 300;
}

pub fn update_conthist(conthist: &mut ContinuationHistory, prio: Team, previous: Action, team: Team, action: Action, bonus: i32) {
    let prio_piece = previous.piece as usize;
    let prio_to = previous.to as usize;

    let piece = action.piece as usize;
    let to = action.to as usize;
    let clamped_bonus = bonus.clamp(-300, 300);

    conthist[prio.index()][prio_piece][prio_to][team.index()][piece][to]
        += clamped_bonus - conthist[prio.index()][prio_piece][prio_to][team.index()][piece][to] * clamped_bonus.abs() / 300;
}

pub const HIGH_PRIORITY: i32 = 2i32.pow(28);
pub const MAX_KILLERS: usize = 2;

pub fn get_history<T: BitInt>(
    board: &mut Board<T>, 
    info: &mut SearchInfo,
    act: Action, 
    previous: Option<Action>,
    two_ply: Option<Action>,
    noisy: bool
) -> i32 {
    let team = board.state.moving_team;
    if noisy {
        info.capture_history[team.index()][act.from as usize][act.to as usize]
    } else {
        let mut history = info.history[team.index()][act.from as usize][act.to as usize];
        if let Some(previous) = previous {
            history += info.conthist[team.next().index()][previous.piece as usize][previous.to as usize][team.index()][act.piece as usize][act.to as usize] / 2;
        }
        if let Some(previous) = two_ply {
            history += info.conthist[team.index()][previous.piece as usize][previous.to as usize][team.index()][act.piece as usize][act.to as usize] / 2;
        }

        history
    }
}

pub fn score<T: BitInt>(
    board: &mut Board<T>, 
    info: &mut SearchInfo,
    ply: usize,
    act: Action, 
    opps: BitBoard<T>,
    previous: Option<Action>,
    two_ply: Option<Action>,
    found_best_move: Option<Action>
) -> i32 {
    if let Some(found_best_move) = found_best_move {
        if found_best_move == act {
            return HIGH_PRIORITY * 2;
        }
    }
    
    if is_capture(board, act, opps) {
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

pub fn sort_actions<T: BitInt>(
    board: &mut Board<T>, 
    info: &mut SearchInfo,
    ply: usize,
    opps: BitBoard<T>,
    actions: Vec<Action>,
    previous: Option<Action>,
    two_ply: Option<Action>,
    found_best_move: Option<Action>
) -> Vec<ScoredAction> {
    let mut scored = vec![];
    for act in actions {
        scored.push(ScoredAction(act, score(board, info, ply, act, opps, previous, two_ply, found_best_move)))
    }

    scored.sort_by(|a, b| b.1.cmp(&a.1));

    scored
}