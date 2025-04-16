use std::{cmp::Ordering, i32, vec};

use chessing::{bitboard::{BitBoard, BitInt}, game::{action::{Action, ActionRecord}, zobrist::ZobristTable, Board, GameState, Team}, uci::{respond::Info, Uci}};
use ordering::{get_history, mvv_lva, sort_actions, update_conthist, update_history, ContinuationHistory, History, ScoredAction, MAX_KILLERS};

use crate::{eval::{eval, MATERIAL, ROOK}, util::current_time_millis};

mod ordering;

#[derive(Clone, Debug, Copy)]
pub enum Bounds {
    Exact,
    Lower,
    Upper
}

#[derive(Clone, Debug)]
pub struct TtEntry {
    pub hash: u64,
    pub best_move: Option<Action>,
    pub score: i32,
    pub depth: i32,
    pub bounds: Bounds
}

pub struct SearchInfo {
    pub root_depth: i32,
    pub best_move: Option<Action>,
    pub history: History,
    pub capture_history: History,
    pub conthist: ContinuationHistory,
    pub killers: Vec<Vec<Option<Action>>>,
    pub pv_table: Vec<Vec<ActionRecord>>,
    pub zobrist: ZobristTable,
    pub hashes: Vec<u64>,
    pub mobility: Vec<Option<(usize, Team)>>,
    pub tt: Vec<Option<TtEntry>>,
    pub tt_size: u64,
    pub nodes: u64,
    pub score: i32,
    pub abort: bool,
    pub time_to_abort: u128
}

pub const MAX: i32 = 1_000_000;
pub const MIN: i32 = -1_000_000;

fn set_or_push<T>(vec: &mut Vec<T>, index: usize, item: T) {
    if vec.len() > index {
        vec[index] = item;
    } else if vec.len() == index {
        vec.push(item);
    }
}

// Generalize "noisiness"
// Checks if the amount of pieces of a given team/type are changed
fn is_noisy_general<T: BitInt, const N: usize>(board: &mut Board<T, N>, action: Action) -> bool {
    let white = board.state.white.count();
    let black = board.state.black.count();
    let pieces = board.state.pieces.map(|piece| piece.count());

    let history = board.play(action);

    if white != board.state.white.count() || black != board.state.black.count() {
        board.restore(history);
        return true;
    }

    let new_pieces = board.state.pieces.map(|piece| piece.count());

    board.restore(history);
    new_pieces != pieces
}

// Chess-specific "noisiness"
fn is_noisy_chess<T: BitInt, const N: usize>(board: &mut Board<T, N>, action: Action) -> bool {
    if action.piece == 0 && action.info >= 1 {
        // Pawn Promotion or En Passant
        return true;
    }

    return BitBoard::index(action.to).and(board.state.opposite_team()).is_set();
}

fn is_noisy<T: BitInt, const N: usize>(board: &mut Board<T, N>, action: Action) -> bool {
    // For chess, `is_noisy_chess` is idential to `is_noisy_general`
    // However, for some variants this may not be the case
    // is_noisy_general(board, action)
    is_noisy_chess(board, action)
}

pub fn quiescence<T: BitInt, const N: usize>(
    board: &mut Board<T, N>, 
    info: &mut SearchInfo,
    ply: usize,
    mut alpha: i32, 
    beta: i32, 
) -> i32 {
    let stand_pat = eval(board, info, ply);
    let mut best = stand_pat;

    if stand_pat >= beta {
        return stand_pat;
    }

    if stand_pat > alpha {
        alpha = stand_pat;
    }

    let actions = board.list_actions();
    info.mobility[ply] = Some((actions.len(), board.state.moving_team));

    let mut captures = Vec::with_capacity(actions.len());

    for act in actions {
        if is_noisy(board, act) {
            captures.push(act);
        }
    }
    captures.sort_by(|&a, &b| {
        mvv_lva(board, b).cmp(&mvv_lva(board, a))
    });

    for act in captures {
        let state = board.play(act);
        let is_legal = board.game.rules.is_legal(board);

        if !is_legal {
            board.restore(state);
            continue;
        }

        info.nodes += 1;

        let score = -quiescence(board, info, ply + 1, -beta, -alpha);
        board.restore(state);

        if score > best {
            best = score;
            if score > alpha {
                alpha = score;
            }
        }

        if score >= beta {
            break;
        }
    }

    best
}

fn zugzwang_unlikely<T: BitInt, const N: usize>(
    board: &mut Board<T, N>
) -> bool {
    let king = board.state.pieces[5];
    let pawns = board.state.pieces[0];
    let team = board.state.team_to_move();

    team != team.and(king.or(pawns))
    
}

pub fn search<T: BitInt, const N: usize>(
    board: &mut Board<T, N>, 
    info: &mut SearchInfo,
    depth: i32,
    ply: usize,
    mut alpha: i32, 
    beta: i32, 
    is_pv: bool
) -> i32 {
    if depth >= 4 && !info.abort {
        info.abort = current_time_millis() >= info.time_to_abort;
    }

    if info.abort { return 0; }
    //info.pv_table[ply] = vec![];

    if depth <= 0 {
        return quiescence(board, info, ply, alpha, beta);
    }

    let eval = eval(board, info, ply);
    if !is_pv && depth <= 3 {
        if eval - (100 * depth) >= beta {
            return eval;
        }
    }

    let hash = board.game.rules.hash(board, &info.zobrist);

    if info.hashes.contains(&hash) && ply > 0 {
        return 0;
    }

    let index = (hash % info.tt_size) as usize;

    let mut found_best_move: Option<Action> = None;

    let tt_hit = &info.tt[index];
    match tt_hit {
        Some(entry) => {
            if hash == entry.hash {
                let is_in_bounds = match entry.bounds {
                    Bounds::Exact => true,
                    Bounds::Lower => entry.score >= beta,
                    Bounds::Upper => entry.score < alpha
                };
    
                if entry.depth >= depth && is_in_bounds && !is_pv {
                    return entry.score;
                }
    
                found_best_move = entry.best_move;
            }
        }
        None => {}
    }

    let actions = board.list_actions();
    info.mobility[ply] = Some((actions.len(), board.state.moving_team));

    let mut legal_actions = vec![];

    for action in actions {
        let history = board.play(action);
        let is_legal = board.game.rules.is_legal(board);
        board.restore(history);
        if is_legal {
            legal_actions.push(action);
        }
    }

    let opps = board.state.opposite_team();

    match board.game_state(&legal_actions) {
        GameState::Win(Team::White) => {
            return MIN + ply as i32;
        }
        GameState::Win(Team::Black) => {
            return MIN + ply as i32;
        }
        GameState::Draw => {
            return 0;
        }
        GameState::Ongoing => {
            // continue evaluation
        }
    }

    let two_ply = match board.history.get(board.history.len().wrapping_sub(2)) {
        Some(&ActionRecord::Action(action)) => Some(action),
        _ => None
    };

    let previous = match board.history.last() {
        Some(&ActionRecord::Action(action)) => Some(action),
        _ => None
    };

    let null_last_move = match board.history.last() {
        Some(ActionRecord::Null()) => true,
        _ => false
    };
    
    let state = board.play_null();
    board.restore(state);

    if !is_pv && depth >= 3 && zugzwang_unlikely(board) && !null_last_move {
        let reduction = 3 + (depth / 5);
        let nm_depth = depth - reduction;

        let state = board.play_null();
        let is_legal = board.game.rules.is_legal(board);

        if is_legal {
            let null_score = -search(board, info, nm_depth, ply, -beta, -beta + 1, is_pv);
            board.restore(state);
    
            if null_score >= beta {
                return if null_score > MAX / 2 {
                    beta
                } else {
                    null_score
                }
            }
        } else {
            board.restore(state);
        }
    }
    
    info.hashes.push(hash);

    let scored_actions = sort_actions(board, info, ply, legal_actions, previous, two_ply, found_best_move);

    let mut best = MIN;
    let mut best_move: Option<Action> = None;

    let mut bounds = Bounds::Upper; // ALL-node: no move exceeded alpha

    let mut quiets: Vec<Action> = vec![];
    let mut noisies: Vec<Action> = vec![];

    for (index, &ScoredAction(act, _)) in scored_actions.iter().enumerate() {
        let is_tactical = is_noisy(board, act);
        let is_quiet = !is_tactical;
        let team = board.state.moving_team;

        if index > 3 + 2 * (depth * depth) as usize && is_quiet {
            continue;
        }

        let r = if index >= 3 {
            let mut r = if index >= 15 {
                3
            } else if index >= 8 {
                2
            } else {
                1
            };
            r += depth / 6;

            let history = get_history(board, info, act, previous, two_ply, is_tactical);
            r -= history.clamp(-600, 600) / 300;

            r = r.max(0);
            r
        } else {
            0
        };
        let lmr = r > 0;
        
        if depth != info.root_depth && is_quiet && (depth - r) <= 8 && eval + 300 + (75 * depth) <= alpha {
            continue;
        }

        let history = board.play(act);

        info.nodes += 1;

        let new_depth = depth - 1;
        let mut score: i32 = MIN; 
        
        if lmr {
            let reduced = new_depth - r;

            score = -search(board, info, reduced, ply + 1, -alpha - 1, -alpha, false);
            
            if score > alpha && reduced < new_depth {
                score = -search(board, info, new_depth, ply + 1, -alpha - 1, -alpha, false);
            }
        } else if !is_pv || index > 0 {
            score = -search(board, info, new_depth, ply + 1, -alpha - 1, -alpha, false);
        }
        
        if is_pv && (index == 0 || score > alpha) {
            score = -search(board, info, new_depth, ply + 1, -beta, -alpha, is_pv);
        }

        board.restore(history);

        if score > best {
            best = score;
            best_move = Some(act);
            if score > alpha {
                bounds = Bounds::Exact; // PV-node: move exceeded alpha but not beta
                alpha = score;

                if is_pv {
                    let ply = ply as usize;

                    match info.pv_table.get((ply + 1) as usize) {
                        Some(pv_moves) => {
                            for (i, pv) in pv_moves.clone().iter().enumerate() {
                                match pv {
                                    ActionRecord::Null() => {
                                        set_or_push(&mut info.pv_table[ply], i + 1, ActionRecord::Null());
                                        break;
                                    }
                                    &ActionRecord::Action(act) => {
                                        set_or_push(&mut info.pv_table[ply], i + 1, ActionRecord::Action(act));
                                    }
                                }
                            }
                        }
                        None => {}
                    }

                    set_or_push(&mut info.pv_table[ply], 0, ActionRecord::Action(act));
                }
            }
        }

        if score >= beta {
            bounds = Bounds::Lower; // CUT-node: beta-cutoff was performed

            if is_quiet {
                update_history(&mut info.history, team, act, depth * depth);
                for &quiet in &quiets {
                    update_history(&mut info.history, team, quiet, -depth * depth);
                }

                if let Some(previous) = previous {
                    update_conthist(&mut info.conthist, team.next(), previous, team, act, depth * depth);
                    for &quiet in &quiets {
                        update_conthist(&mut info.conthist, team.next(), previous, team, quiet, -depth * depth);
                    }
                }

                if let Some(previous) = two_ply {
                    update_conthist(&mut info.conthist, team, previous, team, act, depth * depth);
                    for &quiet in &quiets {
                        update_conthist(&mut info.conthist, team, previous, team, quiet, -depth * depth);
                    }
                }

                let first_killer = info.killers[0][ply];
                if first_killer != Some(act) {
                    for i in (1..MAX_KILLERS).rev() {
                        let previous = info.killers[i - 1][ply];
                        info.killers[i][ply] = previous;
                    }
                    info.killers[0][ply] = Some(act);
                }
            } else {
                update_history(&mut info.capture_history, team, act, depth * depth);
                for &noisy in &noisies {
                    update_history(&mut info.capture_history, team, noisy, -depth * depth);
                }
            }

            break;
        }

        if is_quiet {
            quiets.push(act);
        } else {
            noisies.push(act);
        }
    }
    
    if info.abort { return 0; }

    if depth == info.root_depth && best_move.is_some() {
        info.best_move = best_move;
    }

    info.tt[index] = Some(TtEntry { 
        hash, 
        best_move,
        depth,
        bounds,
        score: best
    });

    info.hashes.pop();

    best
}

pub fn create_search_info<T: BitInt, const N: usize>(board: &mut Board<T, N>) -> SearchInfo {
    let squares = (board.game.bounds.rows * board.game.bounds.cols) as usize;
    let pieces = board.game.pieces.len() as usize;

    SearchInfo {
        root_depth: 0,
        best_move: None,
        capture_history: vec![ vec![ vec![ 0; squares ]; squares ]; 2 ],
        history: vec![ vec![ vec![ 0; squares ]; squares ]; 2 ],
        conthist: vec![ vec![ vec![ vec![ vec![ vec![ 0; squares ]; pieces ]; 2 ]; squares ]; pieces ]; 2 ],
        pv_table: vec![],
        hashes: vec![],
        killers: vec![],
        mobility: vec![ None; 100 ],
        zobrist: board.game.rules.gen_zobrist(board, 64),
        tt_size: 1_000_000,
        tt: vec![ None; 1_000_000 ],
        nodes: 0,
        score: 0,
        abort: false,
        time_to_abort: u128::MAX
    }
}

pub fn aspiration<T: BitInt, const N: usize>(info: &mut SearchInfo, board: &mut Board<T, N>, depth: i32) -> i32 {
    let max_window_size = ROOK;
    let mut delta = 30;
    let (mut alpha, mut beta) = if depth >= 5 {
        (info.score - delta, info.score + delta)
    } else {
        (MIN, MAX)
    };

    loop {
        let score = search(board, info, depth, 0, alpha, beta, true);
        if info.abort {
            return 0;
        }

        if score <= alpha && score > MIN {
            alpha = (score - delta).max(MIN);
        } else if score >= beta && score < MAX {
            beta = (score + delta).min(MAX);
        } else {
            return score;
        }

        delta *= 2;
        if delta >= max_window_size {
            delta = MAX;
        }
    }
}

pub fn iterative_deepening<T: BitInt, const N: usize>(uci: &Uci, info: &mut SearchInfo, board: &mut Board<T, N>, soft_time: u64, hard_time: u64) {
    let start = current_time_millis();
    info.time_to_abort = start + hard_time as u128;
    info.abort = false;
    info.nodes = 0;
    info.killers = vec![ vec![ None; 100 ]; MAX_KILLERS ];

    for depth in 1..100 {
        info.root_depth = depth;
        info.pv_table = vec![ vec![]; 100 ];

        let score = aspiration(info, board, depth);
        if info.abort {
            break;
        }

        info.score = score;

        let current_time = current_time_millis();

        // PV Tables are still bugged, so temporarily disabling them.
        /*let history = restore_perfectly(board);
        let past_moves = board.history.clone();
        let team = board.state.moving_team.clone();

        let mut pv_acts: Vec<String> = vec![];
        for act in info.pv_table[0].clone() {
            if let ActionRecord::Action(act) = act {
                if board.state.mailbox[act.from as usize] == 0 {
                    // Invalid PV end early
                    break;
                }

                pv_acts.push(board.display_uci_action(act));
                board.play(act);
            }
        }

        board.state.restore(history);
        board.history = past_moves;
        board.state.moving_team = team;*/

        let mut time = (current_time - start) as u64;
        if time == 0 { time = 1; }

        uci.info(Info {
            depth: Some(depth as u32),
            score_cp: Some(info.score),
            time: Some(time),
            nodes: Some(info.nodes),
            nps: Some(info.nodes / time * 1000),
            pv: info.best_move.map(|el| vec![ board.display_uci_action(el) ]), //Some(pv_acts),
            ..Default::default()
        });

        if time > soft_time {
            break;   
        }
    }
}