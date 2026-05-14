You are an expert Java developer autonomous agent. Your task is to complete the integration of the Arithmodynamic engine into the 'compecon' repository, using 'abm.rs' as your conceptual reference. We are transitioning from a neoclassical fractional-reserve fiat model to a thermodynamic proof-of-work model. 

Execute the following steps sequentially. Be extremely careful to remove obsolete imports when deleting classes.

### STEP 1: Purge the Central Bank and Forex Systems
Because we are using a single universal currency (ARITHMODYNAMIC) and an endogenous minting metronome, the Central Bank and Foreign Exchange systems are obsolete.
1. **Delete** the following files entirely:
   - `src/main/java/io/github/uwol/compecon/economy/sectors/financial/CentralBank.java`
   - `src/main/java/io/github/uwol/compecon/economy/sectors/financial/CentralBankCustomer.java`
   - `src/main/java/io/github/uwol/compecon/economy/sectors/financial/impl/CentralBankImpl.java`
   - `src/main/java/io/github/uwol/compecon/engine/dao/CentralBankDAO.java`
   - `src/main/java/io/github/uwol/compecon/engine/dao/inmemory/impl/CentralBankDAOImpl.java`
   - `src/main/java/io/github/uwol/compecon/engine/factory/CentralBankFactory.java`
   - `src/main/java/io/github/uwol/compecon/engine/factory/impl/CentralBankImplFactoryImpl.java`
2. **Remove references:** 
   - In `AgentService.java` and `AgentServiceImpl.java`, delete all `findCentralBank` methods.
   - In `ApplicationContext.java` and `ApplicationContextFactory.java`, delete all Central Bank DAOs and Factories.
   - In `AgentImplFactoryImpl.java`, delete the Central Bank initialization loop.
3. **Purge Forex:** 
   - In `TraderImpl.java` and `CreditBankImpl.java`, delete the `CurrencyTradeEvent` and `ArbitrageTradingEvent` nested classes. Remove all `bankAccountsGoodTrade` logic related to foreign currencies. (Agents only trade in their primary currency now).

### STEP 2: Update the TopologyEngine
Target File: `src/main/java/io/github/uwol/compecon/economy/arithmodynamics/TopologyEngine.java`
Update the engine to include bidirectional mapping, minting constants, and denomination constants. Replace the class body with this:

```java
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
```

### STEP 3: Implement The Universal Metronome and Denominations in BankAccountImpl
Target File: `src/main/java/io/github/uwol/compecon/economy/sectors/financial/impl/BankAccountImpl.java`

We must implement endogenous money minting based on thermodynamic work (`activeBookCounts`), and strictly enforce that transactions happen using the discrete prime denominations.

Add these variables to `BankAccountImpl`:
```java
	protected long vaultBooks = 1; // Starts with 1 active book
	protected int activeBookCounts = 0;
```

Add the Metronome Tick method:
```java
	public void tickMetronome() {
		// Burn books to perform work
		if (activeBookCounts == 0 && vaultBooks > 0) {
			vaultBooks--;
			activeBookCounts = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.TOTAL_BOOK_COUNTS;
		}

		if (activeBookCounts >= 1) {
			activeBookCounts--;
			primeCounts++;
			balance = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getPrimeValueForCounts(primeCounts);
		}

		// Phase Transition: Crystallize PV into new Vault Books
		while (balance >= io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.MINT_SCARCITY) {
			vaultBooks++;
			balance -= io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.MINT_SCARCITY;
			primeCounts = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getCountsForPrimeValue((long) balance);
		}
	}
```

Add the Denomination-Derivation method:
```java
	public double getUsableDenominationBalance() {
		long tempPV = (long) balance;
		long heurs = tempPV / io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.HEUR_PV; 
		tempPV %= io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.HEUR_PV;
		
		long degrees = tempPV / io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.DEGREE_PV; 
		tempPV %= io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.DEGREE_PV;
		
		long twins = tempPV / io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.TWIN_PV;
		
		return (heurs * io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.HEUR_PV) + 
		       (degrees * io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.DEGREE_PV) + 
		       (twins * io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.TWIN_PV);
	}
```

Update `getBalance()` to ONLY return the usable denominations, forcing agents to price things in discrete quanta:
```java
	@Override
	public double getBalance() {
		return getUsableDenominationBalance();
	}
	
	// Add this for internal metrics if needed
	public double getRawPrimeValue() {
	    return balance;
	}
```

### STEP 4: Hook the Metronome into the TimeSystem
Target File: `src/main/java/io/github/uwol/compecon/engine/timesystem/impl/TimeSystemImpl.java`

Find the `nextHour()` method. Inside `nextHour()`, BEFORE calling `triggerEvents()`, execute the Metronome across all bank accounts in the simulation so that they autonomously mint their Arithmodynamic value based on time passed:

Add this block inside `nextHour()`:
```java
		// Execute Universal Metronome for Arithmodynamic Minting
		if (ApplicationContext.getInstance().getBankAccountDAO() != null) {
			for (io.github.uwol.compecon.economy.sectors.financial.BankAccount acc : ApplicationContext.getInstance().getBankAccountDAO().findAll()) {
				if (acc instanceof io.github.uwol.compecon.economy.sectors.financial.impl.BankAccountImpl) {
					((io.github.uwol.compecon.economy.sectors.financial.impl.BankAccountImpl) acc).tickMetronome();
				}
			}
		}
```

### STEP 5: Fix Discretized Order Matching 
Target File: `src/main/java/io/github/uwol/compecon/engine/service/impl/MarketServiceImpl.java`

Because agents now transact strictly using `Heur`, `Degree`, and `Twin` denominations, fractions of prices are invalid. 
In `findBestFulfillmentSet()`, locate the `amountToTake` variable calculation:
```java
double amountToTake = Math.max(0, Math.min(amountToTakeByMaxAmountRestriction,
					Math.min(amountToTakeByTotalPriceRestriction, amountToTakeByMaxPricePerUnitRestriction)));
```
Immediately below it, force it to floor to whole numbers (quanta) regardless of the GoodType:
```java
amountToTake = Math.floor(amountToTake);
```

### STEP 6: Run Maven and Clean Up Compilation Errors
Execute `mvn clean compile`.
You will encounter compilation errors in classes like `StateImpl.java` or `BankImpl.java` where they try to call `getEffectiveKeyInterestRate()` on the deleted `CentralBank`. 
- Hardcode key interest rates to `0.0` or remove the interest calculation loops entirely, as Arithmodynamic value does not generate fiat interest. The money supply is fully determined by the Metronome.
