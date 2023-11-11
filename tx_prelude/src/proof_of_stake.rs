//! Proof of Stake system integration with functions for transactions

use namada_core::types::dec::Dec;
use namada_core::types::hash::Hash;
use namada_core::types::transaction::pos::InitValidator;
use namada_core::types::{key, token};
pub use namada_proof_of_stake::parameters::PosParams;
use namada_proof_of_stake::types::ValidatorMetaData;
use namada_proof_of_stake::{
    become_validator, bond_tokens, change_validator_commission_rate,
    change_validator_metadata, claim_reward_tokens, deactivate_validator,
    reactivate_validator, read_pos_params, redelegate_tokens, unbond_tokens,
    unjail_validator, withdraw_tokens, BecomeValidator,
};
pub use namada_proof_of_stake::{parameters, types, ResultSlashing};

use super::*;

impl Ctx {
    /// Self-bond tokens to a validator when `source` is `None` or equal to
    /// the `validator` address, or delegate tokens from the `source` to the
    /// `validator`.
    pub fn bond_tokens(
        &mut self,
        source: Option<&Address>,
        validator: &Address,
        amount: token::Amount,
    ) -> TxResult {
        let current_epoch = self.get_block_epoch()?;
        bond_tokens(self, source, validator, amount, current_epoch, None)
    }

    /// Unbond self-bonded tokens from a validator when `source` is `None`
    /// or equal to the `validator` address, or unbond delegated tokens from
    /// the `source` to the `validator`.
    pub fn unbond_tokens(
        &mut self,
        source: Option<&Address>,
        validator: &Address,
        amount: token::Amount,
    ) -> EnvResult<ResultSlashing> {
        let current_epoch = self.get_block_epoch()?;
        unbond_tokens(self, source, validator, amount, current_epoch, false)
    }

    /// Withdraw unbonded tokens from a self-bond to a validator when
    /// `source` is `None` or equal to the `validator` address, or withdraw
    /// unbonded tokens delegated to the `validator` to the `source`.
    pub fn withdraw_tokens(
        &mut self,
        source: Option<&Address>,
        validator: &Address,
    ) -> EnvResult<token::Amount> {
        let current_epoch = self.get_block_epoch()?;
        withdraw_tokens(self, source, validator, current_epoch)
    }

    /// Change validator commission rate.
    pub fn change_validator_commission_rate(
        &mut self,
        validator: &Address,
        rate: &Dec,
    ) -> TxResult {
        let current_epoch = self.get_block_epoch()?;
        change_validator_commission_rate(self, validator, *rate, current_epoch)
    }

    /// Unjail a jailed validator and re-enter the validator sets.
    pub fn unjail_validator(&mut self, validator: &Address) -> TxResult {
        let current_epoch = self.get_block_epoch()?;
        unjail_validator(self, validator, current_epoch)
    }

    /// Redelegate bonded tokens from one validator to another one.
    pub fn redelegate_tokens(
        &mut self,
        owner: &Address,
        src_validator: &Address,
        dest_validator: &Address,
        amount: token::Amount,
    ) -> TxResult {
        let current_epoch = self.get_block_epoch()?;
        redelegate_tokens(
            self,
            owner,
            src_validator,
            dest_validator,
            current_epoch,
            amount,
        )
    }

    /// Claim available reward tokens
    pub fn claim_reward_tokens(
        &mut self,
        source: Option<&Address>,
        validator: &Address,
    ) -> EnvResult<token::Amount> {
        let current_epoch = self.get_block_epoch()?;
        claim_reward_tokens(self, source, validator, current_epoch)
    }

    /// Attempt to initialize a validator account. On success, returns the
    /// initialized validator account's address.
    pub fn init_validator(
        &mut self,
        InitValidator {
            account_keys,
            threshold,
            consensus_key,
            eth_cold_key,
            eth_hot_key,
            protocol_key,
            commission_rate,
            max_commission_rate_change,
            email,
            description,
            website,
            discord_handle,
            validator_vp_code_hash: _,
        }: InitValidator,
        validator_vp_code_hash: Hash,
    ) -> EnvResult<Address> {
        let current_epoch = self.get_block_epoch()?;
        // Init validator account
        let validator_address = self.init_account(validator_vp_code_hash)?;
        storage_api::account::init_account_storage(
            self,
            &validator_address,
            &account_keys,
            threshold,
        )?;
        let eth_cold_key = key::common::PublicKey::Secp256k1(eth_cold_key);
        let eth_hot_key = key::common::PublicKey::Secp256k1(eth_hot_key);

        let params = read_pos_params(self)?;
        become_validator(BecomeValidator {
            storage: self,
            params: &params,
            address: &validator_address,
            consensus_key: &consensus_key,
            protocol_key: &protocol_key,
            eth_cold_key: &eth_cold_key,
            eth_hot_key: &eth_hot_key,
            current_epoch,
            commission_rate,
            max_commission_rate_change,
            metadata: ValidatorMetaData {
                email,
                description,
                website,
                discord_handle,
            },
            offset_opt: None,
        })?;

        Ok(validator_address)
    }

    /// Deactivate validator
    pub fn deactivate_validator(&mut self, validator: &Address) -> TxResult {
        let current_epoch = self.get_block_epoch()?;
        deactivate_validator(self, validator, current_epoch)
    }

    /// Reactivate validator
    pub fn reactivate_validator(&mut self, validator: &Address) -> TxResult {
        let current_epoch = self.get_block_epoch()?;
        reactivate_validator(self, validator, current_epoch)
    }

    /// Change validator metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn change_validator_metadata(
        &mut self,
        validator: &Address,
        email: Option<String>,
        description: Option<String>,
        website: Option<String>,
        discord_handle: Option<String>,
        commission_rate: Option<Dec>,
    ) -> TxResult {
        let current_epoch = self.get_block_epoch()?;
        change_validator_metadata(
            self,
            validator,
            email,
            description,
            website,
            discord_handle,
            commission_rate,
            current_epoch,
        )
    }
}
