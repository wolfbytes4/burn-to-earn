use cosmwasm_std::{
    entry_point, to_binary, Env, Deps, DepsMut,
    MessageInfo, Response, StdError, StdResult, Addr, CanonicalAddr,
    Binary, Uint128, CosmosMsg
};
use crate::error::ContractError;
use crate::msg::{ContractsResponse, BurnInfoResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ContractInfo, HistoryToken };
use crate::state::{ State, CONFIG_ITEM, ADMIN_ITEM, BURN_HISTORY_STORE, MY_ADDRESS_ITEM, PREFIX_REVOKED_PERMITS};
use secret_toolkit::{
    snip721::{
        batch_burn_nft_msg, register_receive_nft_msg, set_viewing_key_msg, Burn
    },
    permit::{validate, Permit, RevokedPermits},
    snip20::{ transfer_msg }
};  
pub const BLOCK_SIZE: usize = 256;


#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg
) -> Result<Response, StdError> {
    // create initial state
    let state = State { 
        owner: info.sender.clone(),  
        contract_infos: msg.contract_infos,
        shill_contract: msg.shill_contract,
        shill_viewing_key: Some(msg.entropy_shill),
        amount_paid: Uint128::from(0u32),
        num_burned: 0
    };

    //Save Contract state
    CONFIG_ITEM.save(deps.storage, &state)?;
    ADMIN_ITEM.save(deps.storage, &deps.api.addr_canonicalize(&info.sender.to_string())?)?;
    MY_ADDRESS_ITEM.save(deps.storage,  &deps.api.addr_canonicalize(&_env.contract.address.to_string())?)?;
 
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();
    for contract_info in state.contract_infos.iter() { 
        response_msgs.push(
            register_receive_nft_msg(
                _env.contract.code_hash.clone(),
                Some(true),
                None,
                BLOCK_SIZE,
                contract_info.code_hash.clone(),
                contract_info.address.clone().to_string(),
            )?
        ); 
    }
   
    response_msgs.push(
        set_viewing_key_msg(
            state.shill_viewing_key.unwrap().to_string(),
            None,
            BLOCK_SIZE,
            state.shill_contract.code_hash,
            state.shill_contract.address.to_string(),
        )?
    );
   
    deps.api.debug(&format!("Contract was initialized by {}", info.sender));
     
    Ok(Response::new().add_messages(response_msgs))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
) -> Result<Response, ContractError> {
    match msg { 
        ExecuteMsg::RegisterNftReceive { contract_info } => try_register_nft_receive(deps, _env, &info.sender, contract_info), 
        ExecuteMsg::BatchReceiveNft { from, token_ids } => {
            try_batch_receive(deps, _env, &info.sender, &from, token_ids)
        },
        ExecuteMsg::RevokePermit { permit_name } => {
            try_revoke_permit(deps, &info.sender, &permit_name)
        },

        ExecuteMsg::ChangeShillReward{ contract_info }=> {
            try_change_shill_reward(deps, _env, &info.sender, contract_info)
        },
        ExecuteMsg::SendShillBack { amount, address } => {
            try_send_shill_back(deps, _env, &info.sender, amount, address)
        }
    }
} 

fn try_batch_receive(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    from: &Addr,
    token_ids: Vec<String>
) -> Result<Response, ContractError> { 
    deps.api.debug(&format!("Batch received"));
    let mut state = CONFIG_ITEM.load(deps.storage)?;
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();
    let mut shill_amount_to_send = Uint128::from(0u32);
    let mut response_attrs = vec![];
    let burn_history_store = BURN_HISTORY_STORE.add_suffix(from.to_string().as_bytes());

    if state.contract_infos.iter().any(|i| &i.address==sender) {
        let contract_info = state.contract_infos.iter().find(|x| &x.address == sender).unwrap();

        if token_ids.len() > 0 {
            state.num_burned = state.num_burned + token_ids.len() as i32;
            //transfer back 
            let mut burns: Vec<Burn> = Vec::new(); 
            burns.push(
                Burn{ 
                    token_ids: token_ids.clone(),
                    memo: None
                }
            );

            let cosmos_batch_msg = batch_burn_nft_msg(
                burns,
                None,
                BLOCK_SIZE,
                contract_info.code_hash.clone(),
                contract_info.address.to_string(),
            )?;
            response_msgs.push(cosmos_batch_msg);  
            
            shill_amount_to_send+=contract_info.shill_reward * Uint128::from(token_ids.len() as u32);
            
            if shill_amount_to_send > Uint128::from(0u32) {
                response_msgs.push(
                    transfer_msg(
                        from.to_string(),
                        shill_amount_to_send,
                        None,
                        None,
                        BLOCK_SIZE,
                        state.shill_contract.code_hash.to_string(),
                        state.shill_contract.address.to_string()
                    )?);
                state.amount_paid = state.amount_paid + shill_amount_to_send;
            } 

            let history_token: HistoryToken = { HistoryToken {
                token_ids: token_ids,
                owner: from.clone(),
                contract_address: contract_info.address.clone(), 
                burn_date: Some(_env.block.time.seconds()), 
                reward_amount: shill_amount_to_send
            }};
            
            burn_history_store.push(deps.storage, &history_token)?;
            response_attrs.push(("shill_amount".to_string(), shill_amount_to_send.to_string()));
        }
        else{
            return Err(ContractError::CustomError {val: "No Tokens to Burn".to_string()});
        }
    }
    else{
        return Err(ContractError::CustomError {val: "This contract address is not enrolled in the burn to earn program".to_string()});
    }

    CONFIG_ITEM.save(deps.storage, &state)?;
    Ok(Response::new().add_messages(response_msgs).add_attributes(response_attrs))
}

fn try_register_nft_receive(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    contract_info: ContractInfo
) -> Result<Response, ContractError> {
    
    let mut state = CONFIG_ITEM.load(deps.storage)?;
    if state.contract_infos.iter().any(|i| i.address==contract_info.address) {
        return Err(ContractError::CustomError {val: "This contract address has already been registered".to_string()});
    }

    state.contract_infos.push(contract_info.clone());

    CONFIG_ITEM.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_message(register_receive_nft_msg(
            _env.contract.code_hash,
            Some(true),
            None,
            BLOCK_SIZE,
            contract_info.code_hash.clone(),
            contract_info.address.clone().to_string(),
        )?)
    )
}


fn try_revoke_permit(
    deps: DepsMut,
    sender: &Addr,
    permit_name: &str,
) -> Result<Response, ContractError> {
    RevokedPermits::revoke_permit(deps.storage, PREFIX_REVOKED_PERMITS, &sender.to_string(), permit_name);
    
    Ok(Response::default())
}

pub fn try_change_shill_reward(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    contract_info: ContractInfo 
)-> Result<Response, ContractError> {  
    let state = CONFIG_ITEM.load(deps.storage)?;
    if sender.clone() != state.owner {
        return Err(ContractError::Unauthorized {});
    }
    let mut state = CONFIG_ITEM.load(deps.storage)?;
    if state.contract_infos.iter().any(|i| i.address==contract_info.address) {
        let mut c_info = state.contract_infos.iter_mut().find(|x| x.address == contract_info.address).unwrap();
        c_info.shill_reward = contract_info.shill_reward;
        CONFIG_ITEM.save(deps.storage, &state)?;
    }
    else{
        return Err(ContractError::CustomError {val: "This contract address isn't supported".to_string()});
    }
    
    Ok(Response::default())
}

pub fn try_send_shill_back(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    amount: Uint128,
    address: Addr
) -> Result<Response, ContractError> {  
    let state = CONFIG_ITEM.load(deps.storage)?;
    if sender.clone() != state.owner {
        return Err(ContractError::Unauthorized {});
    }
   
    Ok(Response::new().add_message(
        transfer_msg(
            address.to_string(),
            amount,
            None,
            None,
            256,
            state.shill_contract.code_hash.to_string(),
            state.shill_contract.address.to_string()
        )?)
    ) 
}

#[entry_point]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg { 
        QueryMsg::GetContracts {} => to_binary(&query_contracts(deps)?), 
        QueryMsg::GetBurnInfo {} => to_binary(&query_burn_info(deps)?),  
        QueryMsg::GetNumUserBurnHistory { permit } => to_binary(&query_num_user_burn_history(deps, permit)?),
        QueryMsg::GetUserBurnHistory {permit, start_page, page_size} => to_binary(&query_user_burn_history(deps, permit, start_page, page_size)?),
    }
}
 
fn query_contracts(
    deps: Deps,
) -> StdResult<ContractsResponse> { 
    let state = CONFIG_ITEM.load(deps.storage)?;
    Ok(ContractsResponse { contract_infos: state.contract_infos })
} 

fn query_burn_info(
    deps: Deps,
) -> StdResult<BurnInfoResponse> { 
    let state = CONFIG_ITEM.load(deps.storage)?;
    Ok(BurnInfoResponse { num_burned: state.num_burned, amount_paid: state.amount_paid })
} 
 
fn query_user_burn_history(
    deps: Deps, 
    permit: Permit,
    start_page: u32, 
    page_size: u32
) -> StdResult<Vec<HistoryToken>> {
    let (user_raw, my_addr) = get_querier(deps, permit)?;
    
    let burn_history_store = BURN_HISTORY_STORE.add_suffix(&user_raw); 
    let history = burn_history_store.paging(deps.storage, start_page, page_size)?;
    Ok(history)
}

fn query_num_user_burn_history(
    deps: Deps, 
    permit: Permit
) -> StdResult<u32> { 
    let (user_raw, my_addr) = get_querier(deps, permit)?;
    let burn_history_store = BURN_HISTORY_STORE.add_suffix(&user_raw);
    let num = burn_history_store.get_len(deps.storage)?;
    Ok(num)
}  

fn get_querier(
    deps: Deps,
    permit: Permit,
) -> StdResult<(CanonicalAddr, Option<CanonicalAddr>)> {
    if let pmt = permit {
        let me_raw: CanonicalAddr = MY_ADDRESS_ITEM.load(deps.storage)?;
        let my_address = deps.api.addr_humanize(&me_raw)?;
        let querier = deps.api.addr_canonicalize(&validate(
            deps,
            PREFIX_REVOKED_PERMITS,
            &pmt,
            my_address.to_string(),
            None
        )?)?;
        if !pmt.check_permission(&secret_toolkit::permit::TokenPermissions::Owner) {
            return Err(StdError::generic_err(format!(
                "Owner permission is required for burn history queries, got permissions {:?}",
                pmt.params.permissions
            )));
        }
        return Ok((querier, Some(me_raw)));
    }
    return Err(StdError::generic_err(
        "Unauthorized",
    ));  
}

