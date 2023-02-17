use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{
   Addr, Uint128
};
use secret_toolkit::{ 
    permit:: { Permit }
};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg { 
    //nft contract information
    pub contract_infos: Vec<ContractInfo>,
    pub shill_contract: ContractInfo,
    pub entropy_shill: String,
}
 
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address
    pub address: Addr,
    pub shill_reward: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct HistoryToken {
    pub token_ids: Vec<String>,
    pub owner: Addr,
    pub contract_address: Addr,
    pub burn_date: Option<u64>, 
    pub reward_amount: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg { 
    BatchReceiveNft{
        from: Addr, 
        token_ids: Vec<String>
    },  
    RegisterNftReceive{
        contract_info: ContractInfo
    },
    ChangeShillReward{
        contract_info: ContractInfo
    },
    RevokePermit{
        permit_name: String
    },
    SendShillBack{
        amount: Uint128,
        address: Addr
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg { 
    GetBurnInfo {},
    GetContracts {},
    GetUserBurnHistory { 
        permit: Permit,
        start_page: u32, 
        page_size: u32 
    },
    GetNumUserBurnHistory{
        permit: Permit
    }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractsResponse {
    pub contract_infos: Vec<ContractInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct BurnInfoResponse {
    pub num_burned: i32,
    pub amount_paid: Uint128
}
