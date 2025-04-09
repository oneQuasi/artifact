use chessing::{bitboard::BitInt, game::{Board, Team}};

pub fn team_to_move<T: BitInt>(board: &mut Board<T>) -> i32 {
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

const MG_PAWN_TABLE: [i32; 64] = [
     0,   0,   0,   0,   0,   0,  0,   0,
    98, 134,  61,  95,  68, 126, 34, -11,
    -6,   7,  26,  31,  65,  56, 25, -20,
   -14,  13,   6,  21,  23,  12, 17, -23,
   -27,  -2,  -5,  12,  17,   6, 10, -25,
   -26,  -4,  -4, -10,   3,   3, 33, -12,
   -35,  -1, -20, -23, -15,  24, 38, -22,
     0,   0,   0,   0,   0,   0,  0,   0,
];

const MG_KNIGHT_TABLE: [i32; 64] = [
  -167, -89, -34, -49,  61, -97, -15, -107,
   -73, -41,  72,  36,  23,  62,   7,  -17,
   -47,  60,  37,  65,  84, 129,  73,   44,
    -9,  17,  19,  53,  37,  69,  18,   22,
   -13,   4,  16,  13,  28,  19,  21,   -8,
   -23,  -9,  12,  10,  19,  17,  25,  -16,
   -29, -53, -12,  -3,  -1,  18, -14,  -19,
  -105, -21, -58, -33, -17, -28, -19,  -23,
];

const MG_BISHOP_TABLE: [i32; 64] = [
   -29,   4, -82, -37, -25, -42,   7,  -8,
   -26,  16, -18, -13,  30,  59,  18, -47,
   -16,  37,  43,  40,  35,  50,  37,  -2,
    -4,   5,  19,  50,  37,  37,   7,  -2,
    -6,  13,  13,  26,  34,  12,  10,   4,
     0,  15,  15,  15,  14,  27,  18,  10,
     4,  15,  16,   0,   7,  21,  33,   1,
   -33,  -3, -14, -21, -13, -12, -39, -21,
];

const MG_ROOK_TABLE: [i32; 64] = [
    32,  42,  32,  51,  63,   9,  31,  43,
    27,  32,  58,  62,  80,  67,  26,  44,
    -5,  19,  26,  36,  17,  45,  61,  16,
   -24, -11,   7,  26,  24,  35,  -8, -20,
   -36, -26, -12,  -1,   9,  -7,   6, -23,
   -45, -25, -16, -17,   3,   0,  -5, -33,
   -44, -16, -20,  -9,  -1,  11,  -6, -71,
   -19, -13,   1,  17,  16,   7, -37, -26,
];

const MG_QUEEN_TABLE: [i32; 64] = [
   -28,   0,  29,  12,  59,  44,  43,  45,
   -24, -39,  -5,   1, -16,  57,  28,  54,
   -13, -17,   7,   8,  29,  56,  47,  57,
   -27, -27, -16, -16,  -1,  17,  -2,   1,
    -9, -26,  -9, -10,  -2,  -4,   3,  -3,
   -14,   2, -11,  -2,  -5,   2,  14,   5,
   -35,  -8,  11,   2,   8,  15,  -3,   1,
    -1, -18,  -9,  10, -15, -25, -31, -50,
];

const MG_KING_TABLE: [i32; 64] = [
   -65,  23,  16, -15, -56, -34,   2,  13,
    29,  -1, -20,  -7,  -8,  -4, -38, -29,
    -9,  24,   2, -16, -20,   6,  22, -22,
   -17, -20, -12, -27, -30, -25, -14, -36,
   -49,  -1, -27, -39, -46, -44, -33, -51,
   -14, -14, -22, -46, -44, -30, -15, -27,
     1,   7,  -8, -64, -43, -16,   9,   8,
   -15,  36,  12, -54,   8, -28,  24,  14,
];

const fn flip_sq(sq: u8) -> u8 {
    sq ^ 56
}

const fn flip(table: [i32; 64]) -> [i32; 64] {
    let mut flipped = [0; 64];
    let mut i = 0;
    while i < 64 {
        flipped[flip_sq(i as u8) as usize] = table[i];
        i += 1;
    }
    flipped
}

const MG_PAWN_TABLE_WHITE: [i32; 64] = flip(MG_PAWN_TABLE);
const MG_KNIGHT_TABLE_WHITE: [i32; 64] = flip(MG_KNIGHT_TABLE);
const MG_BISHOP_TABLE_WHITE: [i32; 64] = flip(MG_BISHOP_TABLE);
const MG_ROOK_TABLE_WHITE: [i32; 64] = flip(MG_ROOK_TABLE);
const MG_QUEEN_TABLE_WHITE: [i32; 64] = flip(MG_QUEEN_TABLE);
const MG_KING_TABLE_WHITE: [i32; 64] = flip(MG_KING_TABLE);

pub const MATERIAL: [ i32; 6 ] = [ PAWN, KNIGHT, BISHOP, ROOK, QUEEN, 0 ];

pub fn eval<T: BitInt>(board: &mut Board<T>) -> i32 {
    let mut score = 0;

    let pawns = board.state.pieces[0];
    let knights = board.state.pieces[1];
    let bishops = board.state.pieces[2];
    let rooks = board.state.pieces[3];
    let queens = board.state.pieces[4];
    let kings = board.state.pieces[5];

    let white = board.state.white;
    let black = board.state.black;

    let white_pawns = pawns.and(white);
    let black_pawns = pawns.and(black);

    let white_knights = knights.and(white);
    let black_knights = knights.and(black);

    let white_bishops = bishops.and(white);
    let black_bishops = bishops.and(black);

    let white_rooks = rooks.and(white);
    let black_rooks = rooks.and(black);

    let white_queens = queens.and(white);
    let black_queens = queens.and(black);

    
    let white_king = kings.and(white);
    let black_king = kings.and(black);

    for pawn in white_pawns.iter() {
        score += PAWN;
        score += MG_PAWN_TABLE_WHITE[pawn as usize];
    }

    for pawn in black_pawns.iter() {
        score -= PAWN;
        score -= MG_PAWN_TABLE[pawn as usize];
    }

    for knight in white_knights.iter() {
        score += KNIGHT;
        score += MG_KNIGHT_TABLE_WHITE[knight as usize];
    }

    for knight in black_knights.iter() {
        score -= KNIGHT;
        score -= MG_KNIGHT_TABLE[knight as usize];
    }

    for bishop in white_bishops.iter() {
        score += BISHOP;
        score += MG_BISHOP_TABLE_WHITE[bishop as usize];
    }

    for bishop in black_bishops.iter() {
        score -= BISHOP;
        score -= MG_BISHOP_TABLE[bishop as usize];
    }

    for rook in white_rooks.iter() {
        score += ROOK;
        score += MG_ROOK_TABLE_WHITE[rook as usize];
    }

    for rook in black_rooks.iter() {
        score -= ROOK;
        score -= MG_ROOK_TABLE[rook as usize];
    }

    for queen in white_queens.iter() {
        score += QUEEN;
        score += MG_QUEEN_TABLE_WHITE[queen as usize];
    }

    for queen in black_queens.iter() {
        score -= QUEEN;
        score -= MG_QUEEN_TABLE[queen as usize];
    }

    for king in white_king.iter() {
        score += MG_KING_TABLE_WHITE[king as usize];
    }

    for king in black_king.iter() {
        score -= MG_KING_TABLE[king as usize];
    }

    // Continue for knight, bishop, rook, queen, king

    score *= team_to_move(board);

    score
}