use sui_types::effects::TransactionEffects;
use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ChangeSet(TransactionEffects);

impl Into<TransactionEffects> for ChangeSet {
    fn into(self) -> TransactionEffects {
        self.0
    }
}

impl From<TransactionEffects> for ChangeSet {
    fn from(effects: TransactionEffects) -> Self {
        Self(effects)
    }
}