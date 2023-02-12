use schemars::JsonSchema;
use serde::{ Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Addr, Storage, Uint128};
use cosmwasm_storage::{
    singleton, singleton_read, ReadonlySingleton, Singleton,
};
use secret_toolkit::{ 
    storage:: { Item, AppendStore }
};
use crate::msg::{HistoryToken, ContractInfo};

pub static CONFIG_KEY: &[u8] = b"config"; 
pub const ADMIN_KEY: &[u8] = b"admin";
pub const MY_ADDRESS_KEY: &[u8] = b"my_address"; 
pub const BURN_HISTORY_KEY: &[u8] = b"burn_history";
pub const PREFIX_REVOKED_PERMITS: &str = "revoke";

pub static CONFIG_ITEM: Item<State> = Item::new(CONFIG_KEY); 
pub static ADMIN_ITEM: Item<CanonicalAddr> = Item::new(ADMIN_KEY); 
pub static MY_ADDRESS_ITEM: Item<CanonicalAddr> = Item::new(MY_ADDRESS_KEY); 
pub static BURN_HISTORY_STORE: AppendStore<HistoryToken> = AppendStore::new(BURN_HISTORY_KEY);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {  
    pub owner: Addr, 
    pub contract_infos: Vec<ContractInfo>, 
    pub num_burned: i32,
    pub amount_paid: Uint128,
    pub shill_viewing_key: Option<String>,
    pub shill_contract: ContractInfo
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}