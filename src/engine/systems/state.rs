use hecs::World;
use crate::arithmodynamics::ArithmodynamicNode;
use crate::economy::{Inventory, StateState, GoodType};
use crate::engine::intents::BuyIntent;
use crossbeam::channel::Sender;

pub fn state_system(
    world: &World,
    tx_buy: &Sender<BuyIntent>,
) {
    for (entity, (_node, _inv, _s_state)) in world.query::<(&ArithmodynamicNode, &Inventory, &StateState)>().iter() {
        // Deficit spending / Public services
        tx_buy.send(BuyIntent {
            buyer: entity,
            good: GoodType::FOOD,
            max_amount: 10.0,
            max_price_pv: 50,
        }).unwrap();

        tx_buy.send(BuyIntent {
            buyer: entity,
            good: GoodType::REALESTATE,
            max_amount: 1.0,
            max_price_pv: 1000,
        }).unwrap();
    }
}
