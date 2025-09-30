multiversx_sc::imports!();

use super::storage::State;

#[multiversx_sc::module]
pub trait EventsModule {
    /// Event emitted when the market state is updated.
    #[event("set_market_state_event")]
    fn set_market_state_event(&self, #[indexed] state: State);

    /// Event emitted when the controller address is updated.
    #[event("set_controller_event")]
    fn set_controller_event(&self, #[indexed] controller: &ManagedAddress);

    /// Event emitted when the USH minter contract address is set.
    #[event("set_ush_minter_event")]
    fn set_ush_minter_event(&self, #[indexed] ush_minter: &ManagedAddress, #[indexed] ush_id: &TokenIdentifier);

    /// Event emitted when the issuance of HUSH is started.
    #[event("issue_started_event")]
    fn issue_started_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] ticker: &ManagedBuffer, #[indexed] supply: &BigUint);

    /// Event emitted when the issuance of HUSH succeeds.
    #[event("issue_success_event")]
    fn issue_success_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] token_id: &TokenIdentifier, #[indexed] supply: &BigUint);

    /// Event emitted when the issuance of HUSH fails.
    #[event("issue_failure_event")]
    fn issue_failure_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] message: &ManagedBuffer);

    /// Event emitted when the discount rate model contract address is updated.
    #[event("set_discount_rate_model_event")]
    fn set_discount_rate_model_event(&self, #[indexed] discount_rate_model: &ManagedAddress);

    /// Event emitted when accrual timestamp is updated.
    #[event("set_accrual_timestamp_event")]
    fn set_accrual_timestamp_event(&self, #[indexed] timestamp: u64);

    /// Event emitted when tokens are minted.
    #[event("mint_event")]
    fn mint_event(&self, #[indexed] minter: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] tokens: &BigUint);

    /// Event emitted when tokens are redeemed.
    #[event("redeem_event")]
    fn redeem_event(&self, #[indexed] redeemer: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] tokens: &BigUint);

    /// Event emitted when a user borrows underlying.
    #[event("borrow_event")]
    fn borrow_event(&self, #[indexed] borrower: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] account_borrow: &BigUint, #[indexed] total_borrows: &BigUint);

    /// Event emitted when a borrower's position is liquidated.
    #[event("liquidate_borrow_event")]
    fn liquidate_borrow_event(&self, #[indexed] liquidator: &ManagedAddress, #[indexed] borrower: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] collateral_market: &ManagedAddress, #[indexed] tokens: &BigUint);

    /// Event emitted when a borrower repays some borrowed underlying.
    #[event("repay_borrow_event")]
    fn repay_borrow_event(&self, #[indexed] payer: &ManagedAddress, #[indexed] borrower: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] account_borrow: &BigUint, #[indexed] total_borrows: &BigUint);

    /// Event emitted when interest is accrued on the money market.
    #[event("accrue_interest_event")]
    fn accrue_interest_event(&self, #[indexed] delta_borrows: &BigUint, #[indexed] borrow_index: &BigUint, #[indexed] total_borrows: &BigUint);

    /// Event emitted when the stake factor is updated.
    #[event("set_stake_factor_event")]
    fn set_stake_factor_event(&self, #[indexed] stake_factor: &BigUint);

    /// Event emitted when the staking contract address is updated.
    #[event("set_staking_contract_event")]
    fn set_staking_contract_event(&self, #[indexed] staking_sc: &ManagedAddress);

    /// Event emitted when reserves are added.
    #[event("reserves_added_event")]
    fn reserves_added_event(&self, #[indexed] donor: &ManagedAddress, #[indexed] amount: &BigUint);

    /// Event emitted when reserves are reduced.
    #[event("reserves_reduced_event")]
    fn reserves_reduced_event(&self, #[indexed] amount: &BigUint);

    /// Event emitted when staking rewards are claimed.
    #[event("staking_rewards_claimed_event")]
    fn staking_rewards_claimed_event(&self, #[indexed] staking_rewards: &BigUint);

    /// Event emitted when the close factor is updated.
    #[event("set_close_factor_event")]
    fn set_close_factor_event(&self, #[indexed] close_factor: &BigUint);

    /// Event emitted when the liquidation incentive is updated.
    #[event("set_liquidation_incentive_event")]
    fn set_liquidation_incentive_event(&self, #[indexed] liquidation_incentive: &BigUint);

    /// Event emitted when the protocol seize share is updated.
    #[event("set_protocol_seize_share_event")]
    fn set_protocol_seize_share_event(&self, #[indexed] protocol_seize_share: &BigUint);

    /// Event emitted when accrual time threshold is updated.
    #[event("set_borrow_rate_event")]
    fn set_borrow_rate_event(&self, #[indexed] borrow_rate: &BigUint);

    /// Event emitted when accrual time threshold is updated.
    #[event("set_accrual_time_threshold_event")]
    fn set_accrual_time_threshold_event(&self, #[indexed] accrual_time_threshold: u64);

    /// Emitted when a trusted minter is added.
    #[event("add_trusted_minter_event")]
    fn add_trusted_minter_event(&self, #[indexed] minter: &ManagedAddress);

    /// Emitted when a trusted minter is removed.
    #[event("remove_trusted_minter_event")]
    fn remove_trusted_minter_event(&self, #[indexed] minter: &ManagedAddress);
}
