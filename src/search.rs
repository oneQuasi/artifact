use std::{cmp::Ordering, i32, vec};

use chessing::{bitboard::{BitBoard, BitInt}, game::{action::{restore_perfectly, Action, ActionRecord, HistoryState}, zobrist::ZobristTable, Board, GameState, Team}, uci::{respond::Info, Uci}};

use crate::{eval::{eval, MATERIAL, ROOK}, util::current_time_millis};


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

#[derive(Clone, Debug, Copy)]
pub struct ScoredAction(pub Action, pub i32);
pub struct SearchInfo {
    pub root_depth: i32,
    pub best_move: Option<Action>,
    pub history: Vec<Vec<Vec<i32>>>,
    pub killers: Vec<Vec<Option<Action>>>,
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

fn mvv_lva<T: BitInt>(
    board: &mut Board<T>, 
    action: Action,
) -> i32 {
    let attacker_type = board.state.mailbox[action.from as usize] - 1;
    let victim_type = board.state.mailbox[action.to as usize] - 1;

    let attacker_value = MATERIAL[attacker_type as usize];
    let victim_value = MATERIAL[victim_type as usize];

    1000 + (victim_value - attacker_value)
}   

fn update(info: &mut SearchInfo, team: Team, action: Action, bonus: i32) {
    let from = action.from as usize;
    let to = action.to as usize;
    let clamped_bonus = bonus.clamp(-300, 300);

    info.history[team.index()][from][to]
        += clamped_bonus - info.history[team.index()][from][to] * clamped_bonus.abs() / 300;
}

pub const HIGH_PRIORITY: i32 = 2i32.pow(28);
pub const MAX_KILLERS: usize = 2;

fn score<T: BitInt>(
    board: &mut Board<T>, 
    info: &mut SearchInfo,
    ply: usize,
    act: Action, 
    opps: BitBoard<T>,
    found_best_move: Option<Action>
) -> i32 {
    if let Some(found_best_move) = found_best_move {
        if found_best_move == act {
            return HIGH_PRIORITY * 2;
        }
    }

    if is_capture(board, act, opps) {
        return HIGH_PRIORITY + mvv_lva(board, act);
    }

    let mut score = info.history[board.state.moving_team.index()][act.from as usize][act.to as usize];

    for i in 0..MAX_KILLERS {
        let killer = info.killers[i][ply];
        if killer == Some(act) {
            score += 100 - (50 * (i as i32));
        }
    }

    score
}

fn sort_actions<T: BitInt>(
    board: &mut Board<T>, 
    info: &mut SearchInfo,
    ply: usize,
    opps: BitBoard<T>,
    actions: Vec<Action>,
    found_best_move: Option<Action>
) -> Vec<ScoredAction> {
    let mut scored = vec![];
    for act in actions {
        scored.push(ScoredAction(act, score(board, info, ply, act, opps, found_best_move)))
    }

    scored.sort_by(|a, b| b.1.cmp(&a.1));

    scored
}

fn is_capture<T: BitInt>(board: &mut Board<T>, action: Action, opps: BitBoard<T>) -> bool {
    let to_idx = action.to as usize;
    if board.state.mailbox[to_idx] == 0 {
        return false;
    }

    return BitBoard::index(action.to).and(opps).is_set();
}

pub fn quiescence<T: BitInt>(
    board: &mut Board<T>, 
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

    let opps = board.state.opposite_team();
    let mut captures = Vec::with_capacity(actions.len());

    for act in actions {
        if is_capture(board, act, opps) {
            captures.push(act);
        }
    }
    captures.sort_by(|&a, &b| {
        mvv_lva(board, b).cmp(&mvv_lva(board, a))
    });

    for act in captures {
        let history = board.play(act);
        let is_legal = board.game.processor.is_legal(board);

        if !is_legal {
            board.state.restore(history);
            continue;
        }

        info.nodes += 1;

        let score = -quiescence(board, info, ply + 1, -beta, -alpha);
        board.state.restore(history);

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

fn zugzwang_unlikely<T: BitInt>(
    board: &mut Board<T>
) -> bool {
    let king = board.state.pieces[5];
    let pawns = board.state.pieces[0];
    let team = board.state.team_to_move();

    team != team.and(king.or(pawns))
    
}

pub fn search<T: BitInt>(
    board: &mut Board<T>, 
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

    if depth <= 0 {
        return quiescence(board, info, ply, alpha, beta);
    }

    let eval = eval(board, info, ply);
    if depth <= 3 {
        if eval - (100 * depth) >= beta {
            return eval;
        }
    }

    let hash = board.game.processor.hash(board, &info.zobrist);

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
    
                if entry.depth >= depth && is_in_bounds {
                    if depth == info.root_depth {
                        info.best_move = entry.best_move;
                    }
    
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
        let is_legal = board.game.processor.is_legal(board);
        board.state.restore(history);
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

    let null_last_move = match board.state.history.last() {
        Some(ActionRecord::Null()) => true,
        _ => false};
    
    let history = board.play_null();
    board.state.restore(history);

    if depth >= 3 && zugzwang_unlikely(board) && !null_last_move {
        let reduction = 3 + (depth / 5);
        let nm_depth = depth - reduction;

        let history = board.play_null();
        let is_legal = board.game.processor.is_legal(board);

        if is_legal {
            let null_score = -search(board, info, nm_depth, ply, -beta, -beta + 1, is_pv);
            board.state.restore(history);
    
            if null_score >= beta {
                return if null_score > MAX / 2 {
                    beta
                } else {
                    null_score
                }
            }
        } else {
            board.state.restore(history);
        }
    }
    
    info.hashes.push(hash);

    let scored_actions = sort_actions(board, info, ply, opps, legal_actions, found_best_move);

    let mut best = MIN;
    let mut best_move: Option<Action> = None;

    let mut bounds = Bounds::Upper; // ALL-node: no move exceeded alpha

    let mut quiets: Vec<Action> = vec![];

    for (index, &ScoredAction(act, _)) in scored_actions.iter().enumerate() {
        let is_tactical = is_capture(board, act, opps);
        let is_quiet = !is_tactical;

        let lmr = index >= 3;
        let r = if lmr {
            if index >= 12 {
                3
            } else if index >= 6 {
                2
            } else {
                1
            }
        } else {
            0
        };
        
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

        board.state.restore(history);

        if score > best {
            best = score;
            best_move = Some(act);
            if score > alpha {
                bounds = Bounds::Exact; // PV-node: move exceeded alpha but not beta
                alpha = score;
            }
        }

        if score >= beta {
            bounds = Bounds::Lower; // CUT-node: beta-cutoff was performed

            if is_quiet {
                let team = board.state.moving_team;
                update(info, team, act, depth * depth);
                for quiet in quiets {
                    update(info, team, quiet, depth * -depth);
                }

                let first_killer = info.killers[0][ply];
                if first_killer != Some(act) {
                    for i in (1..MAX_KILLERS).rev() {
                        let previous = info.killers[i - 1][ply];
                        info.killers[i][ply] = previous;
                    }
                    info.killers[0][ply] = Some(act);
                }
            }

            break;
        }

        quiets.push(act);
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

pub fn create_search_info<T: BitInt>(board: &mut Board<T>) -> SearchInfo {
    let squares = (board.game.bounds.rows * board.game.bounds.cols) as usize;

    SearchInfo {
        root_depth: 0,
        best_move: None,
        history: vec![ vec![ vec![ 0; squares ]; squares ]; 2 ],
        hashes: vec![],
        killers: vec![],
        mobility: vec![ None; 100 ],
        zobrist: board.game.processor.gen_zobrist(board, 64),
        tt_size: 1_000_000,
        tt: vec![ None; 1_000_000 ],
        nodes: 0,
        score: 0,
        abort: false,
        time_to_abort: u128::MAX
    }
}

pub fn aspiration<T: BitInt>(info: &mut SearchInfo, board: &mut Board<T>, depth: i32) -> i32 {
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

pub fn iterative_deepening<T: BitInt>(uci: &Uci, info: &mut SearchInfo, board: &mut Board<T>, soft_time: u64, hard_time: u64) {
    let start = current_time_millis();
    info.time_to_abort = start + hard_time as u128;
    info.abort = false;
    info.nodes = 0;
    info.killers = vec![ vec![ None; 100 ]; MAX_KILLERS ];

    for depth in 1..100 {
        info.root_depth = depth;

        let score = aspiration(info, board, depth);
        if info.abort {
            break;
        }

        info.score = score;

        let current_time = current_time_millis();

        let history = restore_perfectly(board);
        let past_moves = board.state.history.clone();
        let team = board.state.moving_team.clone();

        board.state.restore(history);
        board.state.history = past_moves;
        board.state.moving_team = team;

        let mut time = (current_time - start) as u64;
        if time == 0 { time = 1; }

        uci.info(Info {
            depth: Some(depth as u32),
            score_cp: Some(info.score),
            time: Some(time),
            nodes: Some(info.nodes),
            nps: Some(info.nodes / time * 1000),
            pv: info.best_move.map(|el| vec![ board.display_uci_action(el) ]),
            ..Default::default()
        });

        if time > soft_time {
            break;   
        }
    }
}