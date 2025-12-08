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

const CACHE_DEPTH_SKIP: u64 = 2;
const MOVE_ORDERING_MAX_DEPTH: u64 = 20;

struct MinimaxState<'cache> {
    moves_examined: u64,
    cache: &'cache mut Cache,
}

impl<'cache> MinimaxState<'cache> {
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

        let moves = board.safe_moves(board.moves());
        if moves.empty() {
            // Two possibilities:
            // 1. The board is full, it follows that we're the first player.
            //    We are testing if the first player can win the game, so return false.
            // 2. There are no safe moves, we'll always lose, see safe_moves().
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
            if depth == 5 {
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
        "Result: {}",
        MinimaxState {
            moves_examined: 0,
            cache: &mut cache
        }
        .minimax(board::empty(), 0)
    );
}
