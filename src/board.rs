extern crate test;

use bitboard;
use bitboard::BitBoard;

pub const FIRST_PLAYER: u64 = 0;
pub const SECOND_PLAYER: u64 = 1;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Board {
    // Stones for the player that will make the next move (this can be player 1 or 2).
    current: BitBoard,
    // Stones for the player that made the last move (if any).
    other: BitBoard,
}

pub fn empty() -> Board {
    return Board {
        current: bitboard::empty(),
        other: bitboard::empty(),
    };
}

impl Board {
    pub fn moves(self) -> BitBoard {
        return self.current.moves(self.other);
    }

    pub fn do_move(self, move_: BitBoard) -> Board {
        return Board {
            current: self.other,
            other: self.current.do_move(move_),
        };
    }

    pub fn can_win(self, moves: BitBoard) -> bool {
        return self.current.can_win(moves);
    }

    pub fn wins_involving(self, move_: BitBoard) -> u32 {
        return self.other.wins_involving(move_);
    }

    pub fn raw(self) -> (u64, u64) {
        return (self.current.raw(), self.other.raw());
    }

    pub fn safe_moves(self, moves: BitBoard) -> BitBoard {
        return self.other.safe_moves(moves);
    }

    pub fn mirror(self) -> Board {
        return Board {
            current: self.current.mirror(),
            other: self.other.mirror(),
        };
    }

    fn canonical_max(self) -> Board {
        return std::cmp::max(self, self.mirror());
    }

    fn canonical_lazy(self) -> Board {
        let current_mirrored = self.current.mirror();
        if current_mirrored > self.current {
            return self;
        }
        let other_mirrored = self.other.mirror();
        if self.current == current_mirrored && other_mirrored > self.other {
            return self;
        }
        return Board {
            current: current_mirrored,
            other: other_mirrored,
        };
    }

    pub fn canonical(self) -> Board {
        return self.canonical_max();
    }

    pub fn with_color_less(self) -> Board {
        return Board {
            current: self.current.add_color_less(self.other),
            other: self.other.add_color_less(self.current),
        };
    }

    // Returns 0 if the first player is to move, 1 if the second player is to move.
    // This is slow, though it could be faster with a population count.
    pub fn player(self) -> u64 {
        let mut stones = 0;
        for row in (0..6).rev() {
            for col in 0..7 {
                if self.current.is_set(col, row) || self.other.is_set(col, row) {
                    stones += 1;
                }
            }
        }
        return stones & 1;
    }

    fn print_custom(self, current_token: char, other_token: char) {
        for row in (0..6).rev() {
            for col in 0..7 {
                let c = match (self.current.is_set(col, row), self.other.is_set(col, row)) {
                    (false, false) => '.',
                    (true, false) => current_token,
                    (false, true) => other_token,
                    (true, true) => '*',
                };
                print! {"{}", c};
            }
            println!();
        }
    }

    // print always prints an X for the first player, and an O for the second player.
    pub fn print(self) {
        let player = self.player();
        if player == FIRST_PLAYER {
            self.print_custom('X', 'O');
        } else {
            self.print_custom('O', 'X');
        }
    }
}

mod tests {
    use super::*;
    const ITERATION_COUNT: u64 = 10;

    #[bench]
    fn bench_canonical_max(b: &mut test::Bencher) {
        b.iter(|| {
            for i in 0..ITERATION_COUNT {
                for j in 0..ITERATION_COUNT {
                    let board = Board {
                        current: bitboard::random(i),
                        other: bitboard::random(j),
                    };
                    test::black_box(board.canonical_max());
                }
            }
        });
    }

    #[bench]
    fn bench_canonical_lazy(b: &mut test::Bencher) {
        b.iter(|| {
            for i in 0..ITERATION_COUNT {
                for j in 0..ITERATION_COUNT {
                    let board = Board {
                        current: bitboard::random(i),
                        other: bitboard::random(j),
                    };
                    test::black_box(board.canonical_lazy());
                }
            }
        });
    }
}
