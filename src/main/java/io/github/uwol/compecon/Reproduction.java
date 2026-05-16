package io.github.uwol.compecon;

import io.github.uwol.compecon.engine.applicationcontext.ApplicationContext;
import io.github.uwol.compecon.engine.applicationcontext.ApplicationContextFactory;
import io.github.uwol.compecon.economy.sectors.financial.Currency;
import java.io.IOException;

public class Reproduction {
    public static void main(String[] args) throws IOException {
        ApplicationContextFactory.configureInMemoryApplicationContext("interdependencies.configuration.properties");
        ApplicationContext.getInstance().getAgentFactory().constructAgentsFromConfiguration();
        
        System.out.println("Day,Households,Factories,Traders,Banks");
        for (int day = 0; day < 300; day++) {
            for (int hour = 0; hour < 24; hour++) {
                ApplicationContext.getInstance().getTimeSystem().nextHour();
            }
            
            int hhCount = ApplicationContext.getInstance().getHouseholdDAO().findAll().size();
            int factoryCount = ApplicationContext.getInstance().getFactoryDAO().findAll().size();
            int traderCount = ApplicationContext.getInstance().getTraderDAO().findAll().size();
            int bankCount = ApplicationContext.getInstance().getCreditBankDAO().findAll().size();
            
            System.out.println(day + "," + hhCount + "," + factoryCount + "," + traderCount + "," + bankCount);
        }
    }
}
