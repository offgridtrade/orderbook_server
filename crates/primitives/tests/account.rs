use offgrid_primitives::account::collect_balances;
use offgrid_primitives::account::futures::FuturesAccount;
use offgrid_primitives::account::option::OptionsAccount;
use offgrid_primitives::account::spot::SpotAccount;
use offgrid_primitives::account::AccountBalances;

#[test]
fn collect_balances_across_accounts() {
    let mut spot = SpotAccount::default();
    let mut futures = FuturesAccount::default();
    let mut options = OptionsAccount::default();

    spot.balances.insert(vec![1], 100);
    futures.balances.insert(vec![2], 200);
    options.balances.insert(vec![1], 300);

    let accounts: [&dyn AccountBalances; 3] = [&spot, &futures, &options];
    let asset_ids = vec![vec![1], vec![2]];
    let balances = collect_balances(&accounts, &asset_ids);
    let balances_map: std::collections::HashMap<Vec<u8>, u64> = balances.into_iter().collect();

    assert_eq!(balances_map.len(), 2);
    assert_eq!(balances_map.get(&vec![1]), Some(&300));
    assert_eq!(balances_map.get(&vec![2]), Some(&200));
}
