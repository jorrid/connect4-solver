extern crate test;

use std::simd::u64x4;

// BitBoard represents the stones of a single player with bits.
// Layout:
// 5 14 23 32 41 50 59
// 4 13 22 31 40 49 58
// 3 12 21 30 39 48 57
// 2 11 20 29 38 47 56
// 1 10 19 28 37 46 55
// 0  9 18 27 36 45 54
// Additionally bit 63 is used when caching.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BitBoard {
    board: u64,
}

const HOR_STRIDE: u64 = 9;
const VER_STRIDE: u64 = 1;
const DIAG_DOWN_STRIDE: u64 = HOR_STRIDE - 1;
const DIAG_UP_STRIDE: u64 = HOR_STRIDE + 1;

const COL0: u64 = 0x3f;
const COL1: u64 = COL0 << HOR_STRIDE;
const COL2: u64 = COL1 << HOR_STRIDE;
const COL3: u64 = COL2 << HOR_STRIDE;
const COL4: u64 = COL3 << HOR_STRIDE;
const COL5: u64 = COL4 << HOR_STRIDE;
const COL6: u64 = COL5 << HOR_STRIDE;

const ROW0: u64 = 0x40201008040201;
const ROW1: u64 = ROW0 << VER_STRIDE;
const ROW2: u64 = ROW1 << VER_STRIDE;
const ROW3: u64 = ROW2 << VER_STRIDE;
const ROW4: u64 = ROW3 << VER_STRIDE;
const ROW5: u64 = ROW4 << VER_STRIDE;

const VALID_PLACES: u64 = COL0 * ROW0;

fn stridex4() -> u64x4 {
    return u64x4::from([HOR_STRIDE, VER_STRIDE, DIAG_DOWN_STRIDE, DIAG_UP_STRIDE]);
}

fn and4r(x: u64, stride: u64) -> u64 {
    let and2 = x & (x >> stride);
    return and2 & (and2 >> (2 * stride));
}

fn and4rx4(x: u64x4, stride: u64x4) -> u64x4 {
    let and2 = x & (x >> stride);
    return and2 & (and2 >> (u64x4::splat(2) * stride));
}

fn or4l(x: u64, stride: u64) -> u64 {
    let or2 = x | (x << stride);
    return or2 | (or2 << (2 * stride));
}

fn or4r(x: u64, stride: u64) -> u64 {
    let or2 = x | (x >> stride);
    return or2 | (or2 >> (2 * stride));
}

fn or4lx4(x: u64x4, stride: u64x4) -> u64x4 {
    let or2 = x | (x << stride);
    return or2 | (or2 << (u64x4::splat(2) * stride));
}

fn comb34(x: u64, stride: u64) -> u64 {
    let and2 = x & (x >> stride);
    let xor2 = x ^ (x >> stride);
    // Check for patterns x[i]=1, x[i+s]=1 and either x[i+2s]=1 or x[i+3s]=1
    let b21 = and2 & (xor2 >> (2 * stride));
    // Check for patterns x[i+2s]=1, x[i+3s]=1 and either x[i]=1 or x[i+s]=1
    let b12 = xor2 & (and2 >> (2 * stride));
    return b21 | b12;
}

fn comb34x4(x: u64x4, stride: u64x4) -> u64x4 {
    let and2 = x & (x >> stride);
    let xor2 = x ^ (x >> stride);
    let b21 = and2 & (xor2 >> (u64x4::splat(2) * stride));
    let b12 = xor2 & (and2 >> (u64x4::splat(2) * stride));
    return b21 | b12;
}

pub fn empty() -> BitBoard {
    return BitBoard { board: 0 };
}

// for testing/benchmarking purposes.
pub fn random(seed: u64) -> BitBoard {
    return BitBoard { board: seed };
}

impl BitBoard {
    pub fn raw(self) -> u64 {
        return self.board;
    }

    pub fn empty(self) -> bool {
        return self.board == 0;
    }

    pub fn flip(mut self, col: u64, row: u64) -> BitBoard {
        self.board ^= 1 << (col * HOR_STRIDE + row * VER_STRIDE);
        return self;
    }

    pub fn is_set(self, col: u64, row: u64) -> bool {
        return self.board & 1 << (col * HOR_STRIDE + row * VER_STRIDE) != 0;
    }

    // Returns true if more than 1 bit is set.
    pub fn more_than_1(self) -> bool {
        return self.board & (self.board - 1) != 0;
    }

    // Moves returns a bitboard where position i is 1 if a stone can be placed there.
    // It requires the boards of both players to compute.
    pub fn moves(self, other: BitBoard) -> BitBoard {
        let placed_stones = self.board | other.board;
        return BitBoard {
            board: (placed_stones + ROW0) & VALID_PLACES,
        };
    }

    // non_losing_moves must be called on the opponents bitboard with the moves of the
    // current player. Returns a subset of `moves` that doesn't result in an
    // unavoidable win for the opponent.
    pub fn non_losing_moves(self, moves: BitBoard) -> BitBoard {
        // Note: opp_almost_wins might contain bits in invalid places.
        let opp_almost_wins = self.almost_wins().raw();
        // Direct wins are moves that if the opponent plays them, they win.
        let opp_direct_wins = BitBoard {
            board: opp_almost_wins & moves.raw(),
        };
        if opp_direct_wins.more_than_1() {
            // The opponent can always play the other move, so we'll lose.
            return BitBoard { board: 0 };
        }
        // Find all moves that if we would play them, would create an opportunity
        // for the opponent to win.
        let opp_indirect_wins = (opp_almost_wins & VALID_PLACES) >> 1;
        if !opp_direct_wins.empty() {
            // If the opponent can make 1 winning move, we have to play that one.
            // But that might result in an indirect win.
            return BitBoard {
                board: opp_direct_wins.raw() & !opp_indirect_wins,
            };
        }
        // The opponent doesn't have a direct winning move.
        // Just make sure we don't play an indirect opponent win move.
        return BitBoard {
            board: moves.raw() & !opp_indirect_wins,
        };
    }

    pub fn mirror(mut self) -> BitBoard {
        let c04 = self.board & (COL0 | COL4);
        let c135 = self.board & (COL1 | COL3 | COL5);
        let c26 = self.board & (COL2 | COL6);
        self.board = (c04 << (2 * HOR_STRIDE)) | c135 | (c26 >> (2 * HOR_STRIDE));
        // Current layout: 2 1 0 3 6 5 4

        let c012 = self.board & (COL0 | COL1 | COL2);
        let c3 = self.board & COL3;
        let c456 = self.board & (COL4 | COL5 | COL6);
        self.board = (c012 << (4 * HOR_STRIDE)) | c3 | (c456 >> (4 * HOR_STRIDE));

        return self;
    }

    fn won_no_simd(self) -> bool {
        let h4 = and4r(self.board, HOR_STRIDE); // Horizontal -
        let v4 = and4r(self.board, VER_STRIDE); // Vertical |
        let dd4 = and4r(self.board, DIAG_DOWN_STRIDE); // Diagonal \
        let du4 = and4r(self.board, DIAG_UP_STRIDE); // Diagonal /
        return h4 | v4 | dd4 | du4 != 0;
    }

    fn won_simd(self) -> bool {
        return and4rx4(u64x4::splat(self.board), stridex4()) != u64x4::splat(0);
    }

    pub fn won(self) -> bool {
        return self.won_no_simd();
    }

    fn wins_no_simd(self) -> BitBoard {
        let ph = or4l(and4r(self.board, HOR_STRIDE), HOR_STRIDE);
        let pv = or4l(and4r(self.board, VER_STRIDE), VER_STRIDE);
        let pdd = or4l(and4r(self.board, DIAG_DOWN_STRIDE), DIAG_DOWN_STRIDE);
        let pdu = or4l(and4r(self.board, DIAG_UP_STRIDE), DIAG_UP_STRIDE);
        return BitBoard {
            board: ph | pv | pdd | pdu,
        };
    }

    fn wins_simd(self) -> BitBoard {
        let result = or4lx4(and4rx4(u64x4::splat(self.board), stridex4()), stridex4());
        return BitBoard {
            board: result[0] | result[1] | result[2] | result[3],
        };
    }

    pub fn wins(self) -> BitBoard {
        return self.wins_simd();
    }

    // Note: almost_wins might leak bits out of the bitboard.
    fn almost_wins_no_simd(self) -> BitBoard {
        let h = or4l(comb34(self.board, HOR_STRIDE), HOR_STRIDE);
        let v = or4l(comb34(self.board, VER_STRIDE), VER_STRIDE);
        let dd = or4l(comb34(self.board, DIAG_DOWN_STRIDE), DIAG_DOWN_STRIDE);
        let du = or4l(comb34(self.board, DIAG_UP_STRIDE), DIAG_UP_STRIDE);
        return BitBoard {
            board: h | v | dd | du,
        };
    }

    // Like almost_wins(), but with a SIMD implementation.
    fn almost_wins_simd(self) -> BitBoard {
        let result = or4lx4(comb34x4(u64x4::splat(self.board), stridex4()), stridex4());
        return BitBoard {
            board: result[0] | result[1] | result[2] | result[3],
        };
    }

    pub fn almost_wins(self) -> BitBoard {
        return self.almost_wins_simd();
    }

    // Given a bitboard of the opponent, computes the number of possible four-in-a-rows
    // involving move_.
    pub fn wins_involving(self, move_: BitBoard) -> u32 {
        let m = move_.board;
        let p = !self.board & VALID_PLACES;
        let mut k: u32 = 0;
        k += (and4r(p, HOR_STRIDE) & or4r(m, HOR_STRIDE)).count_ones();
        k += (and4r(p, VER_STRIDE) & or4r(m, VER_STRIDE)).count_ones();
        k += (and4r(p, DIAG_DOWN_STRIDE) & or4r(m, DIAG_DOWN_STRIDE)).count_ones();
        k += (and4r(p, DIAG_UP_STRIDE) & or4r(m, DIAG_UP_STRIDE)).count_ones();
        return k;
    }

    // Returns true if we can make a winning move (any column).
    // Requires the result of Board::moves() to compute.
    pub fn can_win(self, moves: BitBoard) -> bool {
        return self.almost_wins().board & moves.board != 0;
    }

    // If this bitboard is the result of moves(), this returns a single move
    // for a selected column.
    pub fn for_column(mut self, column: u64) -> BitBoard {
        self.board &= COL0 << (column * HOR_STRIDE);
        return self;
    }

    pub fn do_move(self, move_: BitBoard) -> BitBoard {
        return BitBoard {
            board: self.board | move_.board,
        };
    }

    // Returns a bitboard that adds opponent's checkers which cannot be part of
    // a four-in-a-row.
    pub fn add_color_less(self, other: BitBoard) -> BitBoard {
        // Get all positions for other that could still be part of a four-in-a-row.
        // (Leaks bits.)
        let other_potential = BitBoard {
            board: !self.board & VALID_PLACES,
        }
        .wins();
        // Get all other checkers that cannot be part of a four-in-a-row.
        let other_blocked = other.board & !other_potential.board;
        return BitBoard {
            board: self.board | other_blocked,
        };
    }
}

mod tests {
    use super::*;
    const ITERATION_COUNT: u64 = 100;

    #[bench]
    fn bench_won_no_simd(b: &mut test::Bencher) {
        b.iter(|| {
            for i in 0..ITERATION_COUNT {
                // `i` is not a valid board, but it shouldn't matter for benchmarking.
                test::black_box(BitBoard { board: i }.won_no_simd());
            }
        });
    }

    #[bench]
    fn bench_won_simd(b: &mut test::Bencher) {
        b.iter(|| {
            for i in 0..ITERATION_COUNT {
                test::black_box(BitBoard { board: i }.won_simd());
            }
        });
    }

    #[bench]
    fn bench_wins_no_simd(b: &mut test::Bencher) {
        b.iter(|| {
            for i in 0..ITERATION_COUNT {
                test::black_box(BitBoard { board: i }.wins_no_simd());
            }
        });
    }

    #[bench]
    fn bench_wins_simd(b: &mut test::Bencher) {
        b.iter(|| {
            for i in 0..ITERATION_COUNT {
                test::black_box(BitBoard { board: i }.wins_simd());
            }
        });
    }

    #[bench]
    fn bench_almost_wins_no_simd(b: &mut test::Bencher) {
        b.iter(|| {
            for i in 0..ITERATION_COUNT {
                test::black_box(BitBoard { board: i }.almost_wins_no_simd());
            }
        });
    }

    #[bench]
    fn bench_almost_wins_simd(b: &mut test::Bencher) {
        b.iter(|| {
            for i in 0..ITERATION_COUNT {
                test::black_box(BitBoard { board: i }.almost_wins_simd());
            }
        });
    }
}
