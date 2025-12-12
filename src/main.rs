#![feature(test)]
// Nightly Rust is required because SIMD isn't stabilized yet.
#![feature(portable_simd)]

// Avoid musl's default allocator due to lackluster performance
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod bitboard;
mod board;
mod cache;

use board::Board;
use cache::Cache;
use std::fs::read_to_string;

const DEBUG_PRINT_DEPTH: u64 = 5;
const CACHE_DEPTH_SKIP: u64 = 2;
const MOVE_ORDERING_MAX_DEPTH: u64 = 20;

struct MinimaxState<'cache> {
    moves_examined: u64,
    cache: &'cache mut Cache,
    first_player_can_draw: bool,
}

impl<'cache> MinimaxState<'cache> {
    // For simplicity, this function returns true if the current player can
    // force a "succesful" outcome: win or maybe a draw, see below.
    // If `first_player_can_draw` is true, then drawing is considered a
    // succesful outcome for the first player to move. Correspondingly,
    // it is not a succesful outcome for the second player to move.
    // And vice versa.
    fn minimax(&mut self, board: Board, depth: u64) -> bool {
        let mut cache_board = None;
        if depth % CACHE_DEPTH_SKIP == 0 {
            cache_board = Some(cache::board(board));
            match self.cache.lookup(cache_board.unwrap()) {
                Some(result) => {
                    return result;
                }
                None => {}
            }
        }

        let moves = board.moves();
        if moves.empty() {
            return self.first_player_can_draw;
        }
        let moves = board.non_losing_moves(moves);
        if moves.empty() {
            return false;
        }

        self.moves_examined += 1;

        let mut col_order = [3, 2, 4, 1, 5, 6, 0];
        if depth < MOVE_ORDERING_MAX_DEPTH {
            let mut scores: [u32; 7] = [0; 7];
            for col in 0..scores.len() {
                let move_ = moves.for_column(col as u64);
                if move_.empty() {
                    scores[col] = 0;
                } else {
                    scores[col] = board.wins_involving(move_);
                }
                col_order[col] = col;
                for i in (0..col).rev() {
                    let s1 = scores[col_order[i + 1]];
                    let s0 = scores[col_order[i]];
                    if s1 > s0
                        || (s1 == s0 && col_order[i + 1].abs_diff(3) < col_order[i].abs_diff(3))
                    {
                        col_order.swap(i + 1, i);
                    }
                }
            }
        }

        let mut success = false;
        for col in col_order {
            let move_ = moves.for_column(col as u64);
            if move_.empty() {
                continue;
            }
            let moved_board = board.do_move(move_);
            let moved_result = self.minimax(moved_board, depth + 1);
            if depth == DEBUG_PRINT_DEPTH {
                moved_board.print();
                println!(
                    "player={}, result={}, examined={}",
                    moved_board.player(),
                    moved_result,
                    self.moves_examined
                );
            }
            if !moved_result {
                success = true;
                break;
            }
        }

        if cache_board.is_some() {
            self.cache.store(cache_board.unwrap(), success);
        }
        return success;
    }
}

fn main() {
    let mut cache = cache::new(26); // 1GB cache.
    println!(
        "First player to move can force a win: {}",
        MinimaxState {
            moves_examined: 0,
            cache: &mut cache,
            first_player_can_draw: false,
        }
        .minimax(board::empty(), 0)
    );
}

mod tests {
    use std::path::Path;

    use super::*;

    fn test_positions_from_file(fname: &str) {
        let mut win_setup = MinimaxState {
            moves_examined: 0,
            cache: &mut cache::new(27), // 2GB cache.
            first_player_can_draw: false,
        };
        let mut draw_setup = MinimaxState {
            moves_examined: 0,
            cache: &mut cache::new(27), // 2GB cache.
            first_player_can_draw: true,
        };
        // Using the format as described here: http://blog.gamesolver.org/solving-connect-four/02-test-protocol/
        for line in read_to_string(Path::new("testdata").join(fname))
            .unwrap()
            .lines()
        {
            println!("{}", line);
            let line_parts: Vec<_> = line.split(" ").collect();
            assert_eq!(line_parts.len(), 2);
            let moves: &str = line_parts[0];
            let score: i64 = line_parts[1].parse().unwrap();
            let mut board = board::empty();
            for mov in moves.as_bytes() {
                let possible_moves = board.moves();
                let move_ = possible_moves.for_column((mov - b'1') as u64);
                assert!(!move_.empty());
                board = board.do_move(move_);
            }
            let first_player_to_move = moves.len() % 2 == 0;
            let can_force_a_win = (if first_player_to_move {
                &mut win_setup
            } else {
                &mut draw_setup
            })
            .minimax(board, moves.len() as u64);
            if can_force_a_win {
                assert!(score > 0);
            } else {
                let can_force_a_draw = (if first_player_to_move {
                    &mut draw_setup
                } else {
                    &mut win_setup
                })
                .minimax(board, moves.len() as u64);
                if can_force_a_draw {
                    assert!(score == 0);
                } else {
                    assert!(score < 0);
                }
            }
        }
    }

    #[test]
    fn test_positions() {
        test_positions_from_file("Test_L3_R1");
        test_positions_from_file("Test_L2_R1");
        test_positions_from_file("Test_L2_R2");
        test_positions_from_file("Test_L1_R1");
        test_positions_from_file("Test_L1_R2");
        test_positions_from_file("Test_L1_R3");
    }
}
