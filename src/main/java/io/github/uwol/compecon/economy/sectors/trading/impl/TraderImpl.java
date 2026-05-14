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

package io.github.uwol.compecon.economy.sectors.trading.impl;

import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Map.Entry;
import java.util.Set;

import io.github.uwol.compecon.economy.behaviour.BudgetingBehaviour;
import io.github.uwol.compecon.economy.bookkeeping.impl.BalanceSheetDTO;
import io.github.uwol.compecon.economy.materia.GoodType;
import io.github.uwol.compecon.economy.property.Property;
import io.github.uwol.compecon.economy.sectors.financial.BankAccount;
import io.github.uwol.compecon.economy.sectors.financial.BankAccount.MoneyType;
import io.github.uwol.compecon.economy.sectors.financial.BankAccount.TermType;
import io.github.uwol.compecon.economy.sectors.financial.BankAccountDelegate;
import io.github.uwol.compecon.economy.sectors.financial.CreditBank;
import io.github.uwol.compecon.economy.sectors.financial.Currency;
import io.github.uwol.compecon.economy.sectors.trading.Trader;
import io.github.uwol.compecon.economy.security.equity.impl.JointStockCompanyImpl;
import io.github.uwol.compecon.engine.applicationcontext.ApplicationContext;
import io.github.uwol.compecon.engine.timesystem.TimeSystemEvent;
import io.github.uwol.compecon.engine.timesystem.impl.DayType;
import io.github.uwol.compecon.engine.timesystem.impl.MonthType;
import io.github.uwol.compecon.math.util.MathUtil;

/**
 * Agent type Trader imports goods and sells them on his domestic market.
 * Traders are price takers.
 */
public class TraderImpl extends JointStockCompanyImpl implements Trader {

	public class TradingEvent implements TimeSystemEvent {

		@Override
		public boolean isDeconstructed() {
			return TraderImpl.this.isDeconstructed;
		}

		protected void offerGoods() {
			/*
			 * for each good type offer owned goods at market price -> price taker
			 */
			for (final GoodType goodType : GoodType.values()) {
				if (!excludedGoodTypes.contains(goodType)) {
					ApplicationContext.getInstance().getMarketService().removeAllSellingOffers(TraderImpl.this,
							TraderImpl.this.primaryCurrency, goodType);

					final double amount = ApplicationContext.getInstance().getPropertyService()
							.getGoodTypeBalance(TraderImpl.this, goodType);
					final double marketPrice = ApplicationContext.getInstance().getMarketService()
							.getMarginalMarketPrice(TraderImpl.this.primaryCurrency, goodType);

					ApplicationContext.getInstance().getMarketService().placeSellingOffer(goodType, TraderImpl.this,
							getBankAccountTransactionsDelegate(), amount, marketPrice);
				}
			}
		}

		@Override
		public void onEvent() {
			assureBankAccountTransactions();

			transferBankAccountBalanceToDividendBankAccount(TraderImpl.this.bankAccountTransactions);

			offerGoods();
		}
	}

	protected BudgetingBehaviour budgetingBehaviour;

	protected Set<GoodType> excludedGoodTypes = new HashSet<GoodType>();

	@Override
	public void deconstruct() {
		super.deconstruct();

		ApplicationContext.getInstance().getTraderFactory().deleteTrader(this);
	}

	@Override
	public BankAccountDelegate getBankAccountGoodsTradeDelegate(final Currency currency) {
		return null;
	}

	public Set<GoodType> getExcludedGoodTypes() {
		return excludedGoodTypes;
	}

	@Override
	public void initialize() {
		super.initialize();

		// trading event every hour
		final TimeSystemEvent tradingEvent = new TradingEvent();
		timeSystemEvents.add(tradingEvent);
		ApplicationContext.getInstance().getTimeSystem().addEvent(tradingEvent, -1, MonthType.EVERY,
				DayType.EVERY, ApplicationContext.getInstance().getTimeSystem().suggestRandomHourType());

		budgetingBehaviour = ApplicationContext.getInstance().getBudgetingBehaviourFactory()
				.newInstanceBudgetingBehaviour(this);
	}

	@Override
	protected BalanceSheetDTO issueBalanceSheet() {
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
		TraderImpl.this.assureBankAccountTransactions();
	}

	@Override
	public void onMarketSettlement(final Property property, final double totalPrice, final Currency currency) {
	}


	public void setExcludedGoodTypes(final Set<GoodType> excludedGoodTypes) {
		this.excludedGoodTypes = excludedGoodTypes;
	}
}