use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::AccountBalances;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpotAccount {
    /// Account ID
    #[serde(with = "serde_bytes")]
    pub id: Vec<u8>,
    /// client id
    #[serde(with = "serde_bytes")]
    pub cid: Vec<u8>,
    /// Balances of the account with asset id as key and balance as value
    pub balances: HashMap<Vec<u8>, u64>,
    /// state hash of the account
    pub state_hash: Vec<u8>,
}

impl AccountBalances for SpotAccount {
    fn balances(&self) -> &HashMap<Vec<u8>, u64> {
        &self.balances
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SpotAccountError {
    #[error("asset not found")]
    AssetNotFound,
    #[error("balance not enough")]
    BalanceNotEnough,
}

impl SpotAccount {
    pub fn new(id: impl Into<Vec<u8>>, cid: impl Into<Vec<u8>>) -> Self {
        Self { id: id.into(), cid: cid.into(), balances: HashMap::new(), state_hash: Vec::new() }
    }

    pub fn deposit(&mut self, asset: impl Into<Vec<u8>>, amount: u64) {
        let entry = self.balances.entry(asset.into()).or_insert(0);
        *entry = entry.saturating_add(amount);
    }

    pub fn withdraw(&mut self, asset: impl Into<Vec<u8>>, amount: u64) {
        let entry = self.balances.entry(asset.into()).or_insert(0);
        *entry = entry.saturating_sub(amount);
    }

    pub fn transfer(
        &mut self,
        asset: impl Into<Vec<u8>>,
        _from: impl Into<Vec<u8>>,
        to: impl Into<Vec<u8>>,
        amount: u64,
    ) -> Result<(), SpotAccountError> {
        self.balances
            .entry(asset.into())
            .and_modify(|balance| {
                *balance = balance.saturating_sub(amount);
            })
            .or_insert(0);
        self.balances
            .entry(to.into())
            .and_modify(|balance| {
                *balance = balance.saturating_add(amount);
            })
            .or_insert(amount);
        Ok(())
    }

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