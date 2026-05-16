use hecs::World;
use crate::arithmodynamics::ArithmodynamicNode;
use crate::economy::{CreditBankState, BankAccount};

pub fn bank_system(
    world: &mut World,
) {
    // Collect interest from all bank accounts
    let mut transfers = Vec::new();

    // Query for all BankAccounts
    for (acc_entity, account) in world.query::<&BankAccount>().iter() {
        // Find the bank entity associated with this account
        if let Ok(bank_state) = world.get::<&CreditBankState>(account.bank) {
            let interest = (account.balance_pv as f64 * bank_state.interest_rate / 365.0) as u64;
            if interest > 0 {
                transfers.push((account.bank, acc_entity, interest));
            }
        }
    }

    for (bank_entity, acc_entity, amount) in transfers {
        if let Ok(mut bank_node) = world.get::<&mut ArithmodynamicNode>(bank_entity) {
            if bank_node.prime_value >= amount {
                bank_node.prime_value -= amount;
                if let Ok(mut account) = world.get::<&mut BankAccount>(acc_entity) {
                    account.balance_pv += amount;
                }
            }
        }
    }
}
