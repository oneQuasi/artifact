use chessing::{bitboard::{BitBoard, BitInt}, game::{Board, Team}};
use psqt::{BISHOP_EG, BISHOP_EG_WHITE, BISHOP_MG, BISHOP_MG_WHITE, EG_MATERIAL, EG_PSQT_BLACK, EG_PSQT_WHITE, KING_EG, KING_EG_WHITE, KING_MG, KING_MG_WHITE, KNIGHT_EG, KNIGHT_EG_WHITE, KNIGHT_MG, KNIGHT_MG_WHITE, MG_MATERIAL, MG_PSQT_BLACK, MG_PSQT_WHITE, PAWN_EG, PAWN_EG_WHITE, PAWN_MG, PAWN_MG_WHITE, PHASE_VALUE, QUEEN_EG, QUEEN_EG_WHITE, QUEEN_MG, QUEEN_MG_WHITE, ROOK_EG, ROOK_EG_WHITE, ROOK_MG, ROOK_MG_WHITE};

use crate::search::SearchInfo;

mod psqt;

pub fn team_to_move<T: BitInt, const N: usize>(board: &mut Board<T, N>) -> i32 {
    match board.state.moving_team {
        Team::White => 1,
        Team::Black => -1
    }
}

pub const MOBILITY: i32 = 2;

pub const MATERIAL: [ i32; 6 ] = MG_MATERIAL;

fn get_mobility_diff<T: BitInt, const N: usize>(
    board: &mut Board<T, N>,
    info: &mut SearchInfo,
    ply: usize
) -> i32 {
    let mut white_mobility = 0;
    let mut black_mobility = 0;

    for ply in (0..ply).rev() {
        if white_mobility > 0 && black_mobility > 0 { break; }
        match info.mobility[ply] {
            Some((mobility, team)) => {
                match team {
                    Team::White => {
                        if white_mobility == 0 { white_mobility = mobility };
                    }
                    Team::Black => {
                        if black_mobility == 0 { black_mobility = mobility; }
                    }
                }
            }
            None => {}
        }
    } 

    (white_mobility as i32) - (black_mobility as i32)
}

// For use in training neural nets on new variants
pub fn eval_primitive<T: BitInt, const N: usize>(
    board: &mut Board<T, N>,
    info: &mut SearchInfo,
    ply: usize
) -> i32 {
    let mut score = 0;

    score += 100 * board.state.white.count() as i32;
    score -= 100 * board.state.black.count() as i32;

    let mobility_bonus = get_mobility_diff(board, info, ply) * MOBILITY;
    score += mobility_bonus;

    score * team_to_move(board)
}

pub fn eval<T: BitInt, const N: usize>(
    board: &mut Board<T, N>,
    info: &mut SearchInfo,
    ply: usize
) -> i32 {
    let mut score = 0;

    let mut mg = 0;
    let mut eg = 0;

    let white = board.state.white;
    let black = board.state.black;

    let mut phase = 0;

    for piece_ind in 0..N {
        let piece = board.state.pieces[piece_ind];

        for sq in piece.and(white).iter() {
            phase += PHASE_VALUE[piece_ind];

            mg += MG_MATERIAL[piece_ind];
            eg += EG_MATERIAL[piece_ind];

            mg += MG_PSQT_WHITE[piece_ind][sq as usize];
            eg += EG_PSQT_WHITE[piece_ind][sq as usize];
        }

        for sq in piece.and(black).iter() {
            phase += PHASE_VALUE[piece_ind];
            
            mg -= MG_MATERIAL[piece_ind];
            eg -= EG_MATERIAL[piece_ind];

            mg -= MG_PSQT_BLACK[piece_ind][sq as usize];
            eg -= EG_PSQT_BLACK[piece_ind][sq as usize];
        }
    }
    
    let mg_phase = phase.min(24);
    let eg_phase = 24 - mg_phase;
    let pesto_score = ((mg_phase * mg) + (eg_phase * eg)) / 24;

    score += pesto_score;

    let mobility_bonus = MOBILITY * get_mobility_diff(board, info, ply);
    score += mobility_bonus;

    score * team_to_move(board)
}