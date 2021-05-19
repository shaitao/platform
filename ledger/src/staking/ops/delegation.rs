//!
//! # Delegation Operation
//!
//! Data representation required when users propose a delegation.
//!

use crate::{
    data_model::{NoReplayToken, Operation, Transaction, ASSET_TYPE_FRA},
    staking::{
        Amount, Staking, TendermintAddr, Validator, COINBASE_PRINCIPAL_PK,
        STAKING_VALIDATOR_MIN_POWER,
    },
};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use zei::xfr::{
    sig::{XfrKeyPair, XfrPublicKey, XfrSignature},
    structs::{XfrAmount, XfrAssetType},
};

/// Used as the inner object of a `Delegation Operation`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelegationOps {
    pub(crate) body: Box<Data>,
    pub(crate) pubkey: XfrPublicKey,
    signature: XfrSignature,
}

impl DelegationOps {
    /// Check the validity of an operation by running it in a staking simulator.
    #[inline(always)]
    pub fn check_run(
        &self,
        staking_simulator: &mut Staking,
        tx: &Transaction,
    ) -> Result<()> {
        self.apply(staking_simulator, tx).c(d!())
    }

    /// Apply new delegation to the target `Staking` instance.
    pub fn apply(&self, staking: &mut Staking, tx: &Transaction) -> Result<()> {
        self.verify()
            .c(d!())
            .and_then(|_| self.check_set_context(staking, tx).c(d!()))
            .and_then(|am| {
                staking
                    .delegate(self.pubkey, &self.body.validator, am)
                    .c(d!())
            })
    }

    /// Verify signature.
    #[inline(always)]
    pub fn verify(&self) -> Result<()> {
        self.pubkey
            .verify(&self.body.to_bytes(), &self.signature)
            .c(d!())
    }

    #[inline(always)]
    fn check_set_context(
        &self,
        staking: &mut Staking,
        tx: &Transaction,
    ) -> Result<Amount> {
        let am = check_delegation_context(tx).c(d!())?;

        if let Some(v) = self.body.validator_staking.as_ref() {
            let h = staking.cur_height;

            if !v.staking_is_basic_valid()
                || am < STAKING_VALIDATOR_MIN_POWER
                || self.body.validator != hex::encode_upper(&v.td_addr)
            {
                return Err(eg!("invalid"));
            }

            let mut v = v.clone();
            v.td_power = am;

            staking
                .validator_check_power_x(am, 0)
                .c(d!())
                .and_then(|_| staking.validator_add_staker(h, v).c(d!()))?;
        }

        Ok(am)
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_related_pubkeys(&self) -> Vec<XfrPublicKey> {
        vec![self.pubkey]
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn new(
        keypair: &XfrKeyPair,
        validator: TendermintAddr,
        nonce: NoReplayToken,
    ) -> Self {
        let body = Box::new(Data::new(validator, nonce));
        let signature = keypair.sign(&body.to_bytes());
        DelegationOps {
            body,
            pubkey: keypair.get_pk(),
            signature,
        }
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn set_nonce(&mut self, nonce: NoReplayToken) {
        self.body.set_nonce(nonce);
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_nonce(&self) -> NoReplayToken {
        self.body.get_nonce()
    }
}

/// The body of a delegation operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// the target validator to delegated to
    pub validator: TendermintAddr,
    /// if set this field, then enter staking flow
    pub validator_staking: Option<Validator>,
    nonce: NoReplayToken,
}

impl Data {
    #[inline(always)]
    fn new(v: TendermintAddr, nonce: NoReplayToken) -> Self {
        Data {
            validator: v,
            validator_staking: None,
            nonce,
        }
    }

    #[inline(always)]
    fn to_bytes(&self) -> Vec<u8> {
        pnk!(bincode::serialize(self))
    }

    #[inline(always)]
    fn set_nonce(&mut self, nonce: NoReplayToken) {
        self.nonce = nonce;
    }

    #[inline(always)]
    fn get_nonce(&self) -> NoReplayToken {
        self.nonce
    }
}

fn check_delegation_context(tx: &Transaction) -> Result<Amount> {
    let owner = tx
        .body
        .operations
        .iter()
        .flat_map(|op| {
            if let Operation::Delegation(ref x) = op {
                Some(x.pubkey)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // only one delegation operation is allowed per transaction
    if 1 != owner.len() {
        return Err(eg!());
    }

    check_delegation_context_principal(tx, owner[0])
        .c(d!("delegation amount is not paid correctly"))
}

fn check_delegation_context_principal(
    tx: &Transaction,
    owner: XfrPublicKey,
) -> Result<Amount> {
    let target_pk = *COINBASE_PRINCIPAL_PK;

    let am = tx
        .body
        .operations
        .iter()
        .flat_map(|op| {
            if let Operation::TransferAsset(ref x) = op {
                let keynum = x
                    .body
                    .transfer
                    .inputs
                    .iter()
                    .map(|i| i.public_key)
                    .collect::<HashSet<_>>()
                    .len();

                // make sure:
                //
                // - all inputs are owned by a same address
                // - the owner of all inputs is same as the delegator
                if 1 == keynum && owner == x.body.transfer.inputs[0].public_key {
                    let am = x
                        .body
                        .outputs
                        .iter()
                        .flat_map(|o| {
                            if let XfrAssetType::NonConfidential(ty) =
                                o.record.asset_type
                            {
                                if ty == ASSET_TYPE_FRA
                                    && target_pk == o.record.public_key
                                {
                                    if let XfrAmount::NonConfidential(i_am) =
                                        o.record.amount
                                    {
                                        return Some(i_am);
                                    }
                                }
                            }
                            None
                        })
                        .sum::<u64>();

                    return Some(am);
                }
            }
            None
        })
        .sum();

    alt!(0 < am, Ok(am), Err(eg!()))
}
