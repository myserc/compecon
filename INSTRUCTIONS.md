You are an expert Java developer autonomous agent. Your task is to refactor the 'computational-economy' (compecon) simulator to replace its fiat currency system with a mathematically backed 'Arithmodynamic' currency system.

Execute the following steps in order. Be precise with file paths and replacements.

### STEP 1: Implement the O(1) Topology Engine
Create a new file: `src/main/java/io/github/uwol/compecon/economy/arithmodynamics/TopologyEngine.java`.
Populate it with the following code. This engine builds a Sieve of Eratosthenes up to a 20,000,000 limit and maps ordinal `prime_counts` to `prime_value`.

```java
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
```

### STEP 2: Refactor the Currency Enum
Target File: `src/main/java/io/github/uwol/compecon/economy/sectors/financial/Currency.java`
Replace the entire enum declaration with a single ARITHMODYNAMIC currency.

Find:
```java
public enum Currency {
	EURO("EUR"), USDOLLAR("USD"), YEN("YEN");
```
Replace with:
```java
public enum Currency {
	ARITHMODYNAMIC("ARITH");
```

### STEP 3: Inject Thermodynamics into the Bank Account
Target File: `src/main/java/io/github/uwol/compecon/economy/sectors/financial/impl/BankAccountImpl.java`

We need to hijack the `balance` variable so it represents `prime_value`, while tracking `prime_counts` and `entropy` under the hood.

Find the instance variables:
```java
	protected double balance;
	protected Currency currency;
```
Replace with:
```java
	protected double balance; // Represents Prime Value (PV)
	protected int primeCounts; // Ordinal position in prime sequence
	protected long cumulativeEntropy; // Net non-linear movement
	protected Currency currency;
```

Find the `deposit` method:
```java
	@Override
	public void deposit(final double amount) {
		assert (!Double.isNaN(amount) && !Double.isInfinite(amount) && amount >= 0.0);
		balance = balance + amount;
	}
```
Replace with:
```java
	@Override
	public void deposit(final double amount) {
		assert (!Double.isNaN(amount) && !Double.isInfinite(amount) && amount >= 0.0);
		balance = balance + amount;
		
		// Arithmodynamic Phase Transition
		int newCounts = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getCountsForPrimeValue((long) balance);
		int entropyDelta = newCounts - this.primeCounts;
		this.cumulativeEntropy += entropyDelta;
		this.primeCounts = newCounts;
	}
```

Find the `withdraw` method:
```java
	@Override
	public void withdraw(final double amount) {
		assert (!Double.isNaN(amount) && !Double.isInfinite(amount) && amount >= 0.0);
		assert (amount <= balance || overdraftPossible);
		balance = balance - amount;
	}
```
Replace with:
```java
	@Override
	public void withdraw(final double amount) {
		assert (!Double.isNaN(amount) && !Double.isInfinite(amount) && amount >= 0.0);
		assert (amount <= balance || overdraftPossible);
		balance = balance - amount;
		
		// Arithmodynamic Phase Transition
		int newCounts = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getCountsForPrimeValue((long) Math.max(0, balance));
		int entropyDelta = newCounts - this.primeCounts;
		this.cumulativeEntropy += entropyDelta; // Will be negative entropy loss
		this.primeCounts = newCounts;
	}
```

### STEP 4: Update Configuration Files
We must remove references to EUR, USD, and YEN to prevent application context crashes, replacing them with ARITHMODYNAMIC.

For the following files:
- `src/main/resources/minimal.configuration.properties`
- `src/main/resources/interdependencies.configuration.properties`
- `src/main/resources/nodependencies.configuration.properties`
- `src/test/resources/testing.configuration.properties`

Perform a global Find & Replace inside those files:
1. Find `EURO` -> Replace with `ARITHMODYNAMIC`
2. Find `USDOLLAR` -> Replace with `DUMMY1` (and set all `.number = 0` for DUMMY1 lines if necessary, or just delete the blocks for USDOLLAR and YEN).
*Note for Agent:* The easiest way to handle the config is to delete all lines containing `USDOLLAR` and `YEN`, and rename `EURO` to `ARITHMODYNAMIC`.

### STEP 5: Fix JMX Dashboards (Optional but required for compile)
Target File: `src/main/java/io/github/uwol/compecon/jmx/JmxNumberOfAgentsModel.java`
Target File: `src/main/java/io/github/uwol/compecon/jmx/JmxNumberOfAgentsModelMBean.java`

Since we deleted currencies, the JMX models will fail to compile. 
In `JmxNumberOfAgentsModelMBean.java`, replace the methods:
```java
	public int getNumberOfHouseholdsArithmodynamic();
```
In `JmxNumberOfAgentsModel.java`, replace the methods:
```java
	@Override
	public int getNumberOfHouseholdsArithmodynamic() {
		return (int) ApplicationContext.getInstance().getModelRegistry()
				.getNationalEconomyModel(Currency.ARITHMODYNAMIC).numberOfAgentsModels.get(Household.class).getValue();
	}
```
(Remove the USD and YEN methods entirely).

### STEP 6: Run Maven Build
Execute `mvn clean package -DskipTests` to verify compilation.
If compilation succeeds, the Arithmodynamic core is successfully integrated as the base ledger system.
