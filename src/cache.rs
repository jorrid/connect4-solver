use board::Board;

const CACHE_USED_BIT: u64 = 1 << 63; // Used on CacheBoard.first
const CACHE_OUTCOME_BIT: u64 = 1 << 63; // Used on CacheBoard.second

pub struct Cache {
    cache: Vec<CacheBoard>,
}

#[derive(Copy, Clone)]
pub struct CacheBoard {
    first: u64,
    second: u64,
}

pub fn new(log_size: u64) -> Cache {
    return Cache {
        cache: vec![empty(); 1 << log_size],
    };
}

pub fn board(board: Board) -> CacheBoard {
    let (b1, b2) = board.with_color_less().canonical().raw();
    return CacheBoard {
        first: b1 | CACHE_USED_BIT,
        second: b2,
    };
}

fn empty() -> CacheBoard {
    return CacheBoard {
        first: 0,
        second: 0,
    };
}

fn murmur(hash: u64) -> u64 {
    let mut h = std::num::Wrapping(hash); // Overflow is on purpose here.
    h ^= h >> 33;
    h *= 0xff51afd7ed558ccd;
    h ^= h >> 33;
    h *= 0xc4ceb9fe1a85ec53;
    h ^= h >> 33;
    return h.0;
}

impl Cache {
    fn key(&self, board: CacheBoard) -> usize {
        let b1 = murmur(board.first);
        let b2 = murmur(board.second);
        return ((b1 ^ b2) as usize) & (self.cache.len() - 1);
    }

    pub fn lookup(&mut self, board: CacheBoard) -> Option<bool> {
        let key = self.key(board);
        let value = self.cache[key];
        if value.first == board.first && value.second & !CACHE_OUTCOME_BIT == board.second {
            return Some(value.second & CACHE_OUTCOME_BIT != 0);
        }
        return None::<bool>;
    }

    pub fn store(&mut self, mut board: CacheBoard, result: bool) {
        let key = self.key(board);
        if result {
            board.second |= CACHE_OUTCOME_BIT;
        }
        self.cache[key] = board;
    }
}
