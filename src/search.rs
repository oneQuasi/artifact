use std::{cmp::Ordering, i32};

use chessing::{bitboard::{BitBoard, BitInt}, game::{action::{Action, ActionRecord}, zobrist::ZobristTable, Board, GameState, Team}, uci::{respond::Info, Uci}};

use crate::{eval::{eval, MATERIAL}, util::current_time_millis};


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
    pub zobrist: ZobristTable,
    pub hashes: Vec<u64>,
    pub tt: Vec<Option<TtEntry>>,
    pub tt_size: u64,
    pub nodes: u64,
    pub score: i32
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

pub const HIGH_PRIORITY: i32 = 2i32.pow(28);

fn score<T: BitInt>(
    board: &mut Board<T>, 
    info: &mut SearchInfo,
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

    info.history[board.state.moving_team.index()][act.from as usize][act.to as usize]
}

fn sort_actions<T: BitInt>(
    board: &mut Board<T>, 
    info: &mut SearchInfo,
    opps: BitBoard<T>,
    actions: Vec<Action>,
    found_best_move: Option<Action>
) -> Vec<ScoredAction> {
    let mut scored = vec![];
    for act in actions {
        scored.push(ScoredAction(act, score(board, info, act, opps, found_best_move)))
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
    mut alpha: i32, 
    beta: i32, 
) -> i32 {
    let stand_pat = eval(board);
    let mut best = stand_pat;

    if stand_pat >= beta {
        return stand_pat;
    }

    if stand_pat > alpha {
        alpha = stand_pat;
    }

    let actions = board.list_actions();
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

        let score = -quiescence(board, info, -beta, -alpha);
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
    ply: i32,
    mut alpha: i32, 
    beta: i32, 
) -> i32 {
    if depth <= 0 {
        return quiescence(board, info, alpha, beta);
    }
    
    let eval = eval(board);
    if depth <= 3 && eval - (100 * depth) >= beta {
        return eval;
    }

    let hash = board.game.processor.hash(board, &info.zobrist);
    let index = (hash % info.tt_size) as usize;

    let mut found_best_move: Option<Action> = None;

    let tt_hit = &info.tt[index];
    if let Some(entry) = tt_hit {
        if hash == entry.hash {
            let is_in_bounds = match entry.bounds {
                Bounds::Exact => true,
                Bounds::Lower => entry.score >= beta,
                Bounds::Upper => entry.score < alpha
            };

            if entry.depth >= depth && is_in_bounds {
                return entry.score;
            }

            found_best_move = entry.best_move;
        }
    }

    let legal_actions = board.list_legal_actions();
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

    if info.hashes.contains(&hash) && ply > 0 {
        return 0;
    }

    let null_last_move = match board.state.history.last() {
        Some(ActionRecord::Null()) => true,
        _ => false};
    
    let king = board.state.pieces[5].and(board.state.team_to_move());
    let history = board.play_null();
    let in_check = board.list_captures(king).and(king).is_set();
    board.state.restore(history);

    if depth >= 3 && zugzwang_unlikely(board) && !null_last_move && !in_check {
        let reduction = 3 + (depth / 5);
        let nm_depth = depth - reduction;

        let history = board.play_null();
        let null_score = -search(board, info, nm_depth, ply, -beta, -beta + 1);
        board.state.restore(history);

        if null_score >= beta {
            return if null_score > MAX / 2 {
                beta
            } else {
                null_score
            }
        }
    }
    
    info.hashes.push(hash);

    let scored_actions = sort_actions(board, info, opps, legal_actions, found_best_move);

    let mut best = i32::MIN;
    let mut best_move: Option<Action> = None;

    let mut bounds = Bounds::Upper; // ALL-node: no move exceeded alpha

    let pv_node = beta - alpha > 1;

    for (index, &ScoredAction(act, _)) in scored_actions.iter().enumerate() {
        let history = board.play(act);

        info.nodes += 1;

        let lmr = index >= 3;

        let new_depth = depth - 1;
        let mut score: i32 = MIN; 
        
        if lmr {
            let reduced = new_depth - 1;

            score = -search(board, info, reduced, ply + 1, -alpha - 1, -alpha);
            
            if score > alpha && reduced < new_depth {
                score = -search(board, info, new_depth, ply + 1, -alpha - 1, -alpha);
            }
        } else if !pv_node || index > 0 {
            score = -search(board, info, new_depth, ply + 1, -alpha - 1, -alpha)
        }
        
        if pv_node && (index == 0 || score > alpha) {
            score = -search(board, info, new_depth, ply + 1, -beta, -alpha)
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

            if !is_capture(board, act, opps) {
                info.history[board.state.moving_team.index()][act.from as usize][act.to as usize] += depth * depth;
            }

            break;
        }
    }
    
    if depth == info.root_depth {
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
        zobrist: board.game.processor.gen_zobrist(board, 64),
        tt_size: 1_000_000,
        tt: vec![ None; 1_000_000 ],
        nodes: 0,
        score: 0
    }
}

pub fn iterative_deepening<T: BitInt>(uci: &Uci, info: &mut SearchInfo, board: &mut Board<T>, soft_time: u64) {
    let start = current_time_millis();
    
    for depth in 1..100 {
        info.root_depth = depth;
        let score = search(board, info, depth, 0, MIN, MAX);
        info.score = score;

        let current_time = current_time_millis();

        let mut time = (current_time - start) as u64;
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

        if time > soft_time {
            break;   
        }
    }
}