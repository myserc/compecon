use std::sync::OnceLock;

pub const LIMIT: usize = 5_000_000;
pub const TOTAL_BOOK_COUNTS: u32 = 180;
pub const MINT_SCARCITY: u64 = 1069; // nthprime(180)

pub fn get_primes() -> &'static Vec<u64> {
    static PRIMES: OnceLock<Vec<u64>> = OnceLock::new();
    PRIMES.get_or_init(|| {
        let mut sieve = vec![true; LIMIT];
        sieve[0] = false;
        sieve[1] = false;
        for p in 2..((LIMIT as f64).sqrt() as usize + 1) {
            if sieve[p] {
                let mut i = p * p;
                while i < LIMIT {
                    sieve[i] = false;
                    i += p;
                }
            }
        }
        sieve.iter().enumerate()
            .filter(|&(_, &is_prime)| is_prime)
            .map(|(i, _)| i as u64)
            .collect()
    })
}

pub fn get_prime_index_map() -> &'static Vec<u32> {
    static MAP: OnceLock<Vec<u32>> = OnceLock::new();
    MAP.get_or_init(|| {
        let primes = get_primes();
        let mut map = vec![0u32; LIMIT];
        let mut current_idx = 0;
        for i in 0..LIMIT {
            if current_idx + 1 < primes.len() && i as u64 >= primes[current_idx + 1] {
                current_idx += 1;
            }
            map[i] = current_idx as u32;
        }
        map
    })
}

pub fn get_counts_for_prime_value(prime_value: u64) -> u32 {
    if prime_value >= LIMIT as u64 {
        return (get_primes().len() - 1) as u32;
    }
    if prime_value < 2 {
        return 0;
    }
    get_prime_index_map()[prime_value as usize] + 1
}

pub fn get_prime_value_for_counts(counts: u32) -> u64 {
    if counts == 0 {
        return 0;
    }
    let primes = get_primes();
    let idx = (counts - 1) as usize;
    if idx >= primes.len() {
        return primes[primes.len() - 1];
    }
    primes[idx]
}

pub fn greatest_prime_factor(mut n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    let mut max_p = 1;
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            max_p = d;
            while n % d == 0 {
                n /= d;
            }
        }
        d += 1;
    }
    if n > 1 {
        max_p = n;
    }
    max_p
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArithmodynamicNode {
    pub prime_value: u64,
    pub counts: u32,
    pub vault_books: u32,
    pub active_book_counts: u32,
    pub balance_adjustment: u64,
    pub entropy_delta: i64,
}

impl ArithmodynamicNode {
    pub fn new(initial_pv: u64) -> Self {
        let counts = get_counts_for_prime_value(initial_pv);
        Self {
            prime_value: initial_pv,
            counts,
            vault_books: 1,
            active_book_counts: 0,
            balance_adjustment: 0,
            entropy_delta: 0,
        }
    }
}

#[cfg(test)]
mod tests;
