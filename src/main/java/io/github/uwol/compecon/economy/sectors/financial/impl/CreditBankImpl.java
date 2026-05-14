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

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Map.Entry;

import io.github.uwol.compecon.economy.behaviour.PricingBehaviour;
import io.github.uwol.compecon.economy.bookkeeping.impl.BalanceSheetDTO;
import io.github.uwol.compecon.economy.materia.GoodType;
import io.github.uwol.compecon.economy.property.Property;
import io.github.uwol.compecon.economy.sectors.financial.BankAccount;
import io.github.uwol.compecon.economy.sectors.financial.BankAccount.MoneyType;
import io.github.uwol.compecon.economy.sectors.financial.BankAccount.TermType;
import io.github.uwol.compecon.economy.sectors.financial.BankAccountDelegate;
import io.github.uwol.compecon.economy.sectors.financial.BankCustomer;
import io.github.uwol.compecon.economy.sectors.financial.CreditBank;
import io.github.uwol.compecon.economy.sectors.financial.Currency;
import io.github.uwol.compecon.economy.sectors.household.Household;
import io.github.uwol.compecon.economy.sectors.state.State;
import io.github.uwol.compecon.economy.security.debt.FixedRateBond;
import io.github.uwol.compecon.engine.applicationcontext.ApplicationContext;
import io.github.uwol.compecon.engine.timesystem.TimeSystemEvent;
import io.github.uwol.compecon.engine.timesystem.impl.DayType;
import io.github.uwol.compecon.engine.timesystem.impl.HourType;
import io.github.uwol.compecon.engine.timesystem.impl.MonthType;
import io.github.uwol.compecon.math.util.MathUtil;

/**
 * Agent type credit bank manages bank accounts and creates money by credit.
 */
public class CreditBankImpl extends BankImpl implements CreditBank {

	public class BondsTradingEvent implements TimeSystemEvent {

		protected double calculateBalanceSumOfPassiveSavingBankAccounts() {
			// bank accounts of non-banks managed by this bank
			double balanceSumOfPassiveBankAccounts = 0.0;

			for (final BankAccount bankAccount : ApplicationContext.getInstance().getBankAccountDAO()
					.findAllBankAccountsManagedByBank(CreditBankImpl.this)) {

				assert (bankAccount.getCurrency().equals(CreditBankImpl.this.primaryCurrency));

				// passive account
				if (bankAccount.getBalance() > 0.0 && TermType.LONG_TERM.equals(bankAccount.getTermType())) {
					// temporary assertion
					assert (bankAccount.getOwner() instanceof Household);
					balanceSumOfPassiveBankAccounts += bankAccount.getBalance();
				}
			}

			assert (balanceSumOfPassiveBankAccounts == 0.0
					|| ApplicationContext.getInstance().getConfiguration().householdConfig.getRetirementSaving());

			return balanceSumOfPassiveBankAccounts;
		}

		protected double calculateFaceValueSumOfOwnedBonds() {
			// bonds bought from other agents
			double faceValueSumOfBonds = 0.0;

			for (final Property property : ApplicationContext.getInstance().getPropertyService()
					.findAllPropertiesOfPropertyOwner(CreditBankImpl.this, FixedRateBond.class)) {
				assert (property instanceof FixedRateBond);
				final FixedRateBond bond = (FixedRateBond) property;
				assert (bond.getOwner() == CreditBankImpl.this);

				// if the bond is not issued by this bank -> is not an unsold
				// bond
				if (!bond.isDeconstructed() && bond.getIssuer() != CreditBankImpl.this) {
					// currently only state bonds are bought by credit banks;
					// TODO can and should be modified
					assert (bond.getIssuer() instanceof State);

					faceValueSumOfBonds += ((FixedRateBond) property).getFaceValue();
				}
			}

			assert (faceValueSumOfBonds == 0.0
					|| ApplicationContext.getInstance().getConfiguration().householdConfig.getRetirementSaving());

			return faceValueSumOfBonds;
		}

		@Override
		public boolean isDeconstructed() {
			return CreditBankImpl.this.isDeconstructed;
		}

		@Override
		public void onEvent() {
			assureBankAccountBondLoan();
			assureBankAccountInterestTransactions();

			/*
			 * transfer received coupons to dividends bank account, so that money is not
			 * hoarded
			 */
			transferBankAccountBalanceToDividendBankAccount(bankAccountInterestTransactions);

			/*
			 * the credit bank is doing fractional reserve banking -> buy bonds for passive
			 * bank accounts
			 */

			final double balanceSumOfPassiveBankAccounts = calculateBalanceSumOfPassiveSavingBankAccounts();

			final double faceValueSumOfBonds = calculateFaceValueSumOfOwnedBonds();

			// TODO money reserves; Basel 3
			final double difference = balanceSumOfPassiveBankAccounts - faceValueSumOfBonds;

			if (getLog().isAgentSelectedByClient(CreditBankImpl.this)) {
				getLog().log(CreditBankImpl.this, BondsTradingEvent.class,
						"sumOfPassiveBankAccounts = %s %s; faceValueSumOfBonds = %s %s => difference = %s %s",
						Currency.formatMoneySum(balanceSumOfPassiveBankAccounts), CreditBankImpl.this.primaryCurrency,
						Currency.formatMoneySum(faceValueSumOfBonds), CreditBankImpl.this.primaryCurrency,
						Currency.formatMoneySum(difference), CreditBankImpl.this.primaryCurrency);
			}

			if (MathUtil.greater(difference, 0.0)) {
				final double balanceBeforeTransaction = CreditBankImpl.this.bankAccountBondLoan.getBalance();

				// obtain bond
				final FixedRateBond fixedRateBond = ApplicationContext.getInstance().getAgentService()
						.findState(CreditBankImpl.this.primaryCurrency)
						.obtainBond(difference, CreditBankImpl.this, getBankAccountBondLoanDelegate());

				assert (fixedRateBond.getOwner() == CreditBankImpl.this);

				fixedRateBond.setFaceValueToBankAccountDelegate(getBankAccountBondLoanDelegate());
				fixedRateBond.setCouponToBankAccountDelegate(getBankAccountInterestTransactionsDelegate());

				assert (balanceBeforeTransaction - difference == CreditBankImpl.this.bankAccountBondLoan.getBalance());

				// ApplicationContext.getInstance().getMarketService().getInstance().buy(FixedRateBond.class,
				// Double.NaN, difference, Double.NaN, CreditBank.this,
				// CreditBank.this.transactionsBankAccount);
			}
		}
	}

	public class DailyInterestCalculationEvent implements TimeSystemEvent {
		@Override
		public boolean isDeconstructed() {
			return CreditBankImpl.this.isDeconstructed;
		}

		@Override
		public void onEvent() {
			assureBankAccountInterestTransactions();

			final double monthlyInterestRate = 0.0;
			final double dailyInterestRate = monthlyInterestRate / 30.0;

			for (final BankAccount bankAccount : ApplicationContext.getInstance().getBankAccountDAO()
					.findAllBankAccountsManagedByBank(CreditBankImpl.this)) {
				if (bankAccount.getOwner() != CreditBankImpl.this) {
					assert (CreditBankImpl.this.primaryCurrency.equals(bankAccount.getCurrency()));

					final double dailyInterest = bankAccount.getBalance() * dailyInterestRate;

					// liability account & positive interest rate or asset
					// account & negative interest rate
					if (dailyInterest > 0.0) {
						transferMoney(CreditBankImpl.this.bankAccountInterestTransactions, bankAccount, dailyInterest,
								"interest earned for customer");
					}
					// asset account & positive interest rate or liability
					// account & negative interest rate
					else if (dailyInterest < 0.0) {
						// credit banks add margin on key interest rate
						final double absMarginDailyInterest = -1.0 * dailyInterest * 1.5;
						transferMoney(bankAccount, CreditBankImpl.this.bankAccountInterestTransactions,
								absMarginDailyInterest, "debt interest from customer");
					}
				}
			}
		}
	}


	@Override
	protected void assertCurrencyIsOffered(final Currency currency) {
		assert (primaryCurrency.equals(currency));
	}


	@Override
	public void assureBankAccountTransactions() {
		if (isDeconstructed) {
			return;
		}

		if (bankAccountTransactions == null) {
			/*
			 * initialize the banks own bank account and open a customer account at this new
			 * bank, so that this bank can transfer money from its own bank account
			 */
			bankAccountTransactions = getPrimaryBank().openBankAccount(this, primaryCurrency, true, "transactions",
					TermType.SHORT_TERM, MoneyType.DEPOSITS);
		}
	}

	@Override
	public void closeCustomerAccount(final BankCustomer customer) {
		assureBankAccountTransactions();

		// for each customer bank account ...
		for (final BankAccount bankAccount : ApplicationContext.getInstance().getBankAccountDAO().findAll(this,
				customer)) {
			/*
			 * transfer balance
			 */
			// when tearing down the simulation, the transactions bank account
			// might be null, if the credit bank is already deconstructed
			// has to be checked each in iteration
			if (!isDeconstructed && bankAccountTransactions != null) {
				if (bankAccount != bankAccountTransactions) {
					// on closing has to be evened up to 0, so that no money
					// is lost in the monetary system
					if (bankAccount.getBalance() >= 0) {
						transferMoney(bankAccount, bankAccountTransactions, bankAccount.getBalance(),
								"evening-up of closed bank account");
					} else {
						transferMoney(bankAccountTransactions, bankAccount, -1.0 * bankAccount.getBalance(),
								"evening-up of closed bank account");
					}
				}
			}
			// inform customer
			customer.onBankCloseBankAccount(bankAccount);
		}

		// convert profit to dividends
		if (bankAccountTransactions != null) {
			transferBankAccountBalanceToDividendBankAccount(bankAccountTransactions);
		}

		ApplicationContext.getInstance().getBankAccountFactory().deleteAllBankAccounts(this, customer);
	}

	@Override
	public void deconstruct() {
		super.deconstruct();

		ApplicationContext.getInstance().getCreditBankFactory().deleteCreditBank(this);
	}

	@Override
	public void deposit(final BankAccount bankAccount, final double amount) {

		assertBankAccountIsManagedByThisBank(bankAccount);
		assert (amount >= 0.0);

		bankAccount.deposit(amount);
	}

	@Override
	public void depositCash(final BankCustomer customer, final BankAccount to, final double amount,
			final Currency currency) {
		assertIsCustomerOfThisBank(customer);
		assertBankAccountIsManagedByThisBank(to);
		assertCurrencyIsOffered(currency);

		// transfer money
		ApplicationContext.getInstance().getHardCashService().decrement(customer, currency, amount);
		to.deposit(amount);
	}

	@Override
	public BankAccountDelegate getBankAccountCentralBankMoneyReservesDelegate() {
		return null;
	}

	@Override
	public BankAccountDelegate getBankAccountCentralBankTransactionsDelegate() {
		return null;
	}

	@Override
	public BankAccountDelegate getBankAccountCurrencyTradeDelegate(final Currency currency) {
		return null;
	}

	private double getSumOfBorrowings(final Currency currency) {
		double sumOfBorrowings = 0;

		for (final BankAccount creditBankAccount : ApplicationContext.getInstance().getBankAccountDAO()
				.findAllBankAccountsManagedByBank(CreditBankImpl.this)) {
			if (creditBankAccount.getCurrency() == currency) {
				if (creditBankAccount.getBalance() > 0.0) {
					sumOfBorrowings += creditBankAccount.getBalance();
				}
			}
		}

		return sumOfBorrowings;
	}

	@Override
	public void initialize() {
		super.initialize();

		// calculate interest on customers bank accounts
		final TimeSystemEvent interestCalculationEvent = new DailyInterestCalculationEvent();
		timeSystemEvents.add(interestCalculationEvent);
		ApplicationContext.getInstance().getTimeSystem().addEvent(interestCalculationEvent, -1, MonthType.EVERY,
				DayType.EVERY, HourType.HOUR_02);

		// bonds trading
		// should happen every hour, so that money flow is distributed over the
		// period, leading to less volatility on markets
		final TimeSystemEvent bondsTradingEvent = new BondsTradingEvent();
		timeSystemEvents.add(bondsTradingEvent);
		ApplicationContext.getInstance().getTimeSystem().addEventEvery(bondsTradingEvent, -1, MonthType.EVERY,
				DayType.EVERY, HourType.EVERY);
	}

	@Override
	protected BalanceSheetDTO issueBalanceSheet() {
		assureBankAccountBondLoan();

		final BalanceSheetDTO balanceSheet = super.issueBalanceSheet();

		return balanceSheet;
	}

	@Override
	public void onBankCloseBankAccount(final BankAccount bankAccount) {
		super.onBankCloseBankAccount(bankAccount);
	}

	@Override
	public void onMarketSettlement(final Currency commodityCurrency, final double amount, final double pricePerUnit,
			final Currency currency) {
	}

	@Override
	public void onMarketSettlement(final GoodType goodType, final double amount, final double pricePerUnit,
			final Currency currency) {
	}

	@Override
	public void onMarketSettlement(final Property property, final double totalPrice, final Currency currency) {
	}

	@Override
	public void transferMoney(final BankAccount from, final BankAccount to, final double amount, final String subject) {
		assert (!isDeconstructed);

		assertIsCustomerOfThisBank(from.getOwner());
		assertBankAccountIsManagedByThisBank(from);

		assert (amount >= 0.0);
		assert (from.getCurrency().equals(to.getCurrency()));
		assert (from.getBalance() >= amount || from.getOverdraftPossible());

		assertIdenticalMoneyType(from, to);

		// no Exception for identical bank accounts, as this correctly
		// might happen in case of bonds etc.
		if (from != to) {
			getLog().bank_onTransfer(from, to, from.getCurrency(), amount, subject);

			final double fromBalanceBefore = from.getBalance();
			final double toBalanceBefore = to.getBalance();

			// is the money flowing internally in this bank?
			if (to.getManagingBank() == this && from.getManagingBank() == this) {
				// transfer money internally
				from.withdraw(amount);
				to.deposit(amount);
			} else { // transfer to another bank
				// For now, in a single currency system, all banks can transfer to each other
				// but without a central bank, we'll just do it directly if it's the same currency
				// Actually, the instructions say the Central Bank is obsolete.
				// In reality we might need some settlement mechanism, but for this simulation
				// let's just do the transfer.
				from.withdraw(amount);
				to.deposit(amount);
			}

			assert (fromBalanceBefore - amount == from.getBalance());
			assert (toBalanceBefore + amount == to.getBalance());
		}
	}

	@Override
	public void withdraw(final BankAccount bankAccount, final double amount) {
		assertBankAccountIsManagedByThisBank(bankAccount);
		assert (amount >= 0.0);

		bankAccount.withdraw(amount);
	}

	@Override
	public double withdrawCash(final BankCustomer customer, final BankAccount from, final double amount,
			final Currency currency) {
		assertIsCustomerOfThisBank(customer);
		assertBankAccountIsManagedByThisBank(from);
		assertCurrencyIsOffered(currency);

		// transfer money
		from.withdraw(amount);
		return ApplicationContext.getInstance().getHardCashService().increment(customer, currency, amount);
	}
}