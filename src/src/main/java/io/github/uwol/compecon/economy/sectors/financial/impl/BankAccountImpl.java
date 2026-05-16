/*
Copyright (C) 2013 u.wol@wwu.de

This file is part of ComputationalEconomy.

ComputationalEconomy is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

ComputationalEconomy is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with ComputationalEconomy. If not, see <http://www.gnu.org/licenses/>.
 */

package io.github.uwol.compecon.economy.sectors.financial.impl;

import io.github.uwol.compecon.economy.sectors.financial.Bank;
import io.github.uwol.compecon.economy.sectors.financial.BankAccount;
import io.github.uwol.compecon.economy.sectors.financial.BankCustomer;
import io.github.uwol.compecon.economy.sectors.financial.Currency;

public class BankAccountImpl implements BankAccount {

	protected double balance = 2.0; // Starts at Twin prime (PV)
	protected int primeCounts = 1; 
	protected double balanceAdjustment = 0.0; // Tracks PV sitting between ordinal steps
	protected long cumulativeEntropy = 0; 
	
	protected long vaultBooks = 2; // Compliant with Rust default_state()

	protected int activeBookCounts = 0;

	protected Currency currency;

	protected int id;

	protected Bank managingBank;

	protected MoneyType moneyType;

	protected String name;

	protected boolean overdraftPossible = true;

	protected BankCustomer owner;

	protected TermType termType;

	@Override
	public void deposit(final double amount) {
		assert (!Double.isNaN(amount) && !Double.isInfinite(amount) && amount >= 0.0);
		int oldCounts = this.primeCounts;
		
		this.balance += amount;
		
		// Proper Arithmodynamic State Update
		this.primeCounts = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getCountsForPrimeValue((long) this.balance);
		long primeBase = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getPrimeValueForCounts(this.primeCounts);
		this.balanceAdjustment = this.balance - primeBase;
		
		this.cumulativeEntropy += (this.primeCounts - oldCounts);
	}

	public void tickMetronome() {
		// Burn books to perform work
		if (activeBookCounts == 0 && vaultBooks > 0) {
			vaultBooks--;
			activeBookCounts = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.TOTAL_BOOK_COUNTS;
		}

		if (activeBookCounts >= 1) {
			activeBookCounts--;
			primeCounts++;
			// Re-integrate the adjustment factor during ordinal leaps
			long primeBase = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getPrimeValueForCounts(primeCounts);
			balance = primeBase + balanceAdjustment;
		}

		// True Phase Transition: Subtract Ordinals, NOT Value
		long threshold = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.MINT_SCARCITY;
		int mintCounts = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getCountsForPrimeValue(threshold);

		while (balance >= threshold) {
			vaultBooks++;
			// Snap back the topological distance
			primeCounts = Math.max(1, primeCounts - mintCounts);
			balanceAdjustment = 0.0; // Efficiency gain wipes out frictional remainder
			balance = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getPrimeValueForCounts(primeCounts);
		}
	}

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

	@Override
	public double getBalance() {
		return getUsableDenominationBalance();
	}

	public double getRawPrimeValue() {
		return getUsableDenominationBalance();
	}

	@Override
	public Currency getCurrency() {
		return currency;
	}

	@Override
	public int getId() {
		return id;
	}

	@Override
	public Bank getManagingBank() {
		return managingBank;
	}

	@Override
	public MoneyType getMoneyType() {
		return moneyType;
	}

	@Override
	public String getName() {
		return name;
	}

	@Override
	public boolean getOverdraftPossible() {
		return overdraftPossible;
	}

	@Override
	public BankCustomer getOwner() {
		return owner;
	}

	@Override
	public TermType getTermType() {
		return termType;
	}

	public void setBalance(final double balance) {
		this.balance = balance;
		this.primeCounts = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getCountsForPrimeValue((long) balance);
	}

	public void setCurrency(final Currency currency) {
		this.currency = currency;
	}

	public void setId(final int id) {
		this.id = id;
	}

	public void setManagingBank(final Bank managingBank) {
		this.managingBank = managingBank;
	}

	public void setMoneyType(final MoneyType moneyType) {
		this.moneyType = moneyType;
	}

	public void setName(final String name) {
		this.name = name;
	}

	public void setOverdraftPossible(final boolean overdraftPossible) {
		this.overdraftPossible = overdraftPossible;
	}

	public void setOwner(final BankCustomer owner) {
		this.owner = owner;
	}

	public void setTermType(final TermType termType) {
		this.termType = termType;
	}

	@Override
	public String toString() {
		return "BankAccount [ID: " + id + ", Balance: " + Currency.formatMoneySum(balance) + " " + currency + ", Name: "
				+ name + ", Owner: " + owner + "]";
	}

	@Override
	public void withdraw(final double amount) {
		assert (!Double.isNaN(amount) && !Double.isInfinite(amount) && amount >= 0.0);
		assert (amount <= balance || overdraftPossible);
		int oldCounts = this.primeCounts;
		
		this.balance -= amount;
		
		// Proper Arithmodynamic State Update
		this.primeCounts = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getCountsForPrimeValue((long) Math.max(0, this.balance));
		long primeBase = io.github.uwol.compecon.economy.arithmodynamics.TopologyEngine.getPrimeValueForCounts(this.primeCounts);
		this.balanceAdjustment = this.balance - primeBase;
		
		this.cumulativeEntropy += (this.primeCounts - oldCounts); 
	}

}
