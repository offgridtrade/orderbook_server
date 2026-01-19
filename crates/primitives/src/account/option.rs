use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::AccountBalances;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OptionsAccount {
    /// Account ID
    #[serde(with = "serde_bytes")]
    pub id: Vec<u8>,
    /// Balances of the account with asset id as key and balance as value
    pub balances: HashMap<Vec<u8>, u64>,
    /// state hash of the account
    pub state_hash: Vec<u8>,
}

impl AccountBalances for OptionsAccount {
    fn balances(&self) -> &HashMap<Vec<u8>, u64> {
        &self.balances
    }
}

impl OptionsAccount {
    pub fn update_state_hash(&mut self) {
        self.state_hash = self.hash_state();
    }

    pub fn hash_state(&self) -> Vec<u8> {
        let mut hasher = Hasher::new();
        for (asset, amount) in self.balances.iter() {
            hasher.update(asset);
            hasher.update(&amount.to_le_bytes());
        }
        hasher.finalize().as_bytes().to_vec()
    }
}
