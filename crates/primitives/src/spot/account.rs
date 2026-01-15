use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Account {
    /// Account ID
    #[serde(with = "serde_bytes")]
    pub id: Vec<u8>,
    /// Balances of the account with asset id as key and balance as value
    pub balances: HashMap<Vec<u8>, u64>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AccountError {
    #[error("asset not found")]
    AssetNotFound,
    #[error("balance not enough")]
    BalanceNotEnough,
}

impl From<HashMapError> for AccountError {
    fn from(err: HashMapError) -> Self {
        AccountError::HashMapError(err)
    }
}

impl Account {
    pub fn new(id: impl Into<Vec<u8>>) -> Self {
        Self { id: id.into(), balance: 0 }
    }

    pub fn deposit(&mut self, asset: impl Into<Vec<u8>>, amount: u64) {
        self.balances.entry(asset.into()).or_insert(0).saturating_add(amount);
    }

    pub fn withdraw(&mut self, asset: impl Into<Vec<u8>>, amount: u64) {
        self.balances.entry(asset.into()).or_insert(0).saturating_sub(amount);
    }

    pub fn transfer(&mut self,asset: impl Into<Vec<u8>>, from: impl Into<Vec<u8>>, to: impl Into<Vec<u8>>, amount: u64) -> Result<(), AccountError> {
        self.balances.entry(asset.into()).or_insert(0).saturating_sub(amount);
        self.balances.entry(to.into()).or_insert(0).saturating_add(amount);
    }
}