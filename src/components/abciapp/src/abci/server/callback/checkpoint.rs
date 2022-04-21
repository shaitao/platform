#[allow(unused_imports)]
use findora_scanner::db::{self, SqlxError, SqlxPgPool};
use findora_scanner::schema::{
    DelegationInfo, DelegationLite, DelegationState as ScannerDelegationState, Rate,
};
use tokio::runtime::Runtime;

use std::collections::HashMap;

lazy_static! {
    pub static ref RUNTIME: Runtime = Runtime::new().unwrap();
    pub static ref PG_POOL: SqlxPgPool = {
        let pool = RUNTIME.block_on(db::connect()).unwrap();
        println!("Connecting database succeeded.");
        pool
    };
}

use ledger::staking::DelegationState;
use ledger::store::LedgerState;

pub fn state_checkpoint(state: &LedgerState) {
    let staking = state.get_staking();
    let h = staking.cur_height();
    let mut global_delegation_records_map = HashMap::new();
    for (key, d) in staking.delegation_info.global_delegation_records_map.iter() {
        let delegation_lite = DelegationLite {
            delegations: HashMap::from_iter(d.delegations.iter().map(|(k, v)| (*k, *v))),
            //[TODO] use public key.
            id: d.id, // delegation rewards will be paid to this pk by default
            start_height: d.start_height,
            end_height: d.end_height,
            state: match d.state {
                DelegationState::Bond => ScannerDelegationState::Bond,
                DelegationState::Paid => ScannerDelegationState::Paid,
                DelegationState::Free => ScannerDelegationState::Free,
            },
            rwd_amount: d.rwd_amount,
            proposer_rwd_cnt: d.proposer_rwd_cnt, // how many times you get proposer rewards
            delegation_rwd_cnt: d.delegation_rwd_cnt, // how many times you get delegation rewards
            receiver_pk: d.receiver_pk.clone(),
            tmp_delegators: HashMap::from_iter(
                d.tmp_delegators.iter().map(|(k, v)| (*k, *v)),
            ), 
        };
        global_delegation_records_map.insert(*key, delegation_lite);
    }
    let validator_addr_map = if let Some(v) = staking.validator_get_current() {
        HashMap::from_iter(v.addr_td_to_app.iter().map(|(k, v)| (k.clone(), *v)))
    } else {
        HashMap::new()
    };

    let return_rate = {
        let rate_u128 = state.staking_get_block_rewards_rate();
        Rate {
            value: rate_u128[0] as f64 / rate_u128[1] as f64,
        }
    };

    let info = DelegationInfo {
        global_delegation_records_map,
        validator_addr_map,
        return_rate,
    };

    RUNTIME
        .block_on(db::save_delegations(h as _, &info, &PG_POOL))
        .unwrap();
}
