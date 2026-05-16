package io.github.uwol.compecon.economy.arithmodynamics;

public class TopologyEngine {
    public static final int LIMIT = 20_000_000;

    // Arithmodynamic Constants
    public static final int TOTAL_BOOK_COUNTS = 180;
    public static final long MINT_SCARCITY = 1069; // nthprime(180)

    // Denominations
    public static final long HEUR_PV = 151;   // 36 counts
    public static final long DEGREE_PV = 23;  // 9 counts
    public static final long TWIN_PV = 2;     // 1 count

    private static final long[] PRIMES;
    private static final int[] PRIME_INDEX_MAP;

    static {
        boolean[] sieve = new boolean[LIMIT];
        for (int i = 2; i < LIMIT; i++) sieve[i] = true;
        int sqrtLimit = (int) Math.sqrt(LIMIT);
        for (int p = 2; p <= sqrtLimit; p++) {
            if (sieve[p]) {
                for (int i = p * p; i < LIMIT; i += p) sieve[i] = false;
            }
        }
        int count = 0;
        for (int i = 2; i < LIMIT; i++) if (sieve[i]) count++;

        PRIMES = new long[count];
        int idx = 0;
        for (int i = 2; i < LIMIT; i++) if (sieve[i]) PRIMES[idx++] = i;

        PRIME_INDEX_MAP = new int[LIMIT];
        int currentIdx = 0;
        for (int i = 0; i < LIMIT; i++) {
            if (currentIdx + 1 < PRIMES.length && i >= PRIMES[currentIdx + 1]) currentIdx++;
            PRIME_INDEX_MAP[i] = currentIdx;
        }
    }

    public static int getCountsForPrimeValue(long primeValue) {
        if (primeValue >= LIMIT) return PRIMES.length - 1;
        if (primeValue < 2) return 0;
        return PRIME_INDEX_MAP[(int) primeValue] + 1;
    }

    public static long getPrimeValueForCounts(int counts) {
        if (counts <= 0) return 0;
        if (counts > PRIMES.length) return PRIMES[PRIMES.length - 1];
        return PRIMES[counts - 1];
    }
}
