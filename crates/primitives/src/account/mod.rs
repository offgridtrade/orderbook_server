pub mod futures;
pub mod spot;
pub mod option;

use std::collections::HashMap;

/// Common interface to read balances across account types.
pub trait AccountBalances {
    fn balances(&self) -> &HashMap<Vec<u8>, u64>;
}

/// Collect all balances from multiple accounts into a flat list.
pub fn collect_balances(accounts: &[&dyn AccountBalances], asset_ids: &[Vec<u8>]) -> Vec<(Vec<u8>, u64)> {
    let mut balances = HashMap::new();
    for account in accounts {
        for asset_id in asset_ids {
            if let Some(amount) = account.balances().get(asset_id) {
                balances.insert(asset_id.clone(), *amount);
            }
        }
    }
    balances.into_iter().collect()
}