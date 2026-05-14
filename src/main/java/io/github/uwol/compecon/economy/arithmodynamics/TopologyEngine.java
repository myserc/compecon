package io.github.uwol.compecon.economy.arithmodynamics;

public class TopologyEngine {
    public static final int LIMIT = 20_000_000;
    private static final long[] PRIMES;
    private static final int[] PRIME_INDEX_MAP;

    static {
        System.out.println("Initializing Arithmodynamic Topology Engine...");
        boolean[] sieve = new boolean[LIMIT];
        for (int i = 2; i < LIMIT; i++) sieve[i] = true;
        int sqrtLimit = (int) Math.sqrt(LIMIT);
        for (int p = 2; p <= sqrtLimit; p++) {
            if (sieve[p]) {
                for (int i = p * p; i < LIMIT; i += p) sieve[i] = false;
            }
        }
        int count = 0;
        for (int i = 2; i < LIMIT; i++) {
            if (sieve[i]) count++;
        }
        PRIMES = new long[count];
        int idx = 0;
        for (int i = 2; i < LIMIT; i++) {
            if (sieve[i]) PRIMES[idx++] = i;
        }
        PRIME_INDEX_MAP = new int[LIMIT];
        int currentIdx = 0;
        for (int i = 0; i < LIMIT; i++) {
            if (currentIdx + 1 < PRIMES.length && i >= PRIMES[currentIdx + 1]) {
                currentIdx++;
            }
            PRIME_INDEX_MAP[i] = currentIdx;
        }
        System.out.println("Arithmodynamic Topology Engine initialized. Primes generated: " + count);
    }

    public static int getCountsForPrimeValue(long primeValue) {
        if (primeValue >= LIMIT) return PRIMES.length - 1;
        if (primeValue < 2) return 0;
        return PRIME_INDEX_MAP[(int) primeValue] + 1;
    }
}
