use hecs::World;
use crate::arithmodynamics::ArithmodynamicNode;
use crate::economy::{Inventory, TraderState};
use crate::engine::intents::{BuyIntent, SellIntent};
use crossbeam::channel::Sender;

pub fn trader_system(
    world: &World,
    tx_buy: &Sender<BuyIntent>,
    tx_sell: &Sender<SellIntent>,
    market: &crate::engine::market::Market,
) {
    for (entity, (node, inv, t_state)) in world.query::<(&ArithmodynamicNode, &Inventory, &TraderState)>().iter() {
         // Simple arbitrage: buy low, sell high
         let price = market.get_marginal_price(t_state.traded_good).unwrap_or(10.0);

         if inv.get(t_state.traded_good) > 1.0 {
             tx_sell.send(SellIntent {
                 seller: entity,
                 good: t_state.traded_good,
                 amount: inv.get(t_state.traded_good),
                 price_pv: (price * 1.1) as u64,
             }).unwrap();
         } else if node.prime_value > 100 {
             tx_buy.send(BuyIntent {
                 buyer: entity,
                 good: t_state.traded_good,
                 max_amount: 10.0,
                 max_price_pv: (price * 0.9) as u64,
             }).unwrap();
         }
    }
}
