multiversx_sc::imports!();

use crate::storage::State;

#[multiversx_sc::module]
pub trait EventsModule {
    /// Event emitted when the market state is updated.
    #[event("set_market_state_event")]
    fn set_market_state_event(&self, #[indexed] old_state: &State, #[indexed] new_state: &State);

    /// Event emitted when tokens are minted.
    #[event("mint_event")]
    fn mint_event(&self, #[indexed] minter: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] tokens: &BigUint);

    /// Event emitted when tokens are redeemed.
    #[event("redeem_event")]
    fn redeem_event(&self, #[indexed] redeemer: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] tokens: &BigUint);

    /// Event emitted when a user borrows underlying.
    #[event("borrow_event")]
    fn borrow_event(&self, #[indexed] borrower: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] new_account_borrow: &BigUint, #[indexed] new_total_borrows: &BigUint, #[indexed] new_borrower_index: &BigUint);

    /// Event emitted when a borrower's position is liquidated.
    #[event("liquidate_borrow_event")]
    fn liquidate_borrow_event(&self, #[indexed] liquidator: &ManagedAddress, #[indexed] borrower: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] collateral_market: &ManagedAddress, #[indexed] tokens: &BigUint);

    /// Event emitted when a borrower repays some borrowed underlying.
    #[event("repay_borrow_event")]
    fn repay_borrow_event(&self, #[indexed] payer: &ManagedAddress, #[indexed] borrower: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] new_account_borrow: &BigUint, #[indexed] new_total_borrows: &BigUint);

    /// Event emitted when interest is accrued on the money market.
    #[event("accrue_interest_event")]
    fn accrue_interest_event(&self, #[indexed] prev_cash: &BigUint, #[indexed] accumulated_interest: &BigUint, #[indexed] new_borrow_index: &BigUint, #[indexed] new_total_borrows: &BigUint);

    /// Event emitted when market borrow and supply rates are updated.
    #[event("updated_rates_event")]
    fn updated_rates_event(&self, #[indexed] borrow_rate: &BigUint, #[indexed] supply_rate: &BigUint);

    /// Event emitted when the reserve factor is updated.
    #[event("new_reserve_factor_event")]
    fn new_reserve_factor_event(&self, #[indexed] old_reserve_factor: &BigUint, #[indexed] new_reserve_factor: &BigUint);

    /// Event emitted when the stake factor is updated.
    #[event("new_stake_factor_event")]
    fn new_stake_factor_event(&self, #[indexed] old_stake_factor: &BigUint, #[indexed] new_stake_factor: &BigUint);

    /// Event emitted when the controller address is updated.
    #[event("new_controller_event")]
    fn new_controller_event(&self, #[indexed] old_address: &Option<ManagedAddress>, #[indexed] new_address: &ManagedAddress);

    /// Event emitted when the staking contract address is updated.
    #[event("new_staking_contract_event")]
    fn new_staking_contract_event(&self, #[indexed] old_address: &Option<ManagedAddress>, #[indexed] new_address: &ManagedAddress);

    /// Event emitted when the interest rate model contract address is updated.
    #[event("new_interest_rate_model_event")]
    fn new_interest_rate_model_event(&self, #[indexed] old_address: &Option<ManagedAddress>, #[indexed] new_address: &ManagedAddress, #[indexed] r0: &BigUint, #[indexed] m1: &BigUint, #[indexed] m2: &BigUint, #[indexed] uo: &BigUint, #[indexed] r_max: &BigUint);

    /// Event emitted when the issuance of the token is started.
    #[event("issue_started_event")]
    fn issue_started_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] ticker: &ManagedBuffer, #[indexed] supply: &BigUint);

    /// Event emitted when the issuance of the token succeeds.
    #[event("issue_success_event")]
    fn issue_success_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] token_id: &TokenIdentifier, #[indexed] supply: &BigUint);

    /// Event emitted when the issuance of the token fails.
    #[event("issue_failure_event")]
    fn issue_failure_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] message: &ManagedBuffer);

    /// Event emitted when the initial supply is minted.
    #[event("mint_initial_supply_event")]
    fn mint_initial_supply_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] burned_tokens: &BigUint, #[indexed] tokens_left: &BigUint);

    /// Event emitted when reserves are added.
    #[event("reserves_added_event")]
    fn reserves_added_event(&self, #[indexed] benefactor: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] new: &BigUint);

    /// Event emitted when reserves are reduced.
    #[event("reserves_reduced_event")]
    fn reserves_reduced_event(&self, #[indexed] admin: &ManagedAddress, #[indexed] amount: &BigUint, #[indexed] new: &BigUint);

    /// Event emitted when staking rewards are claimed.
    #[event("staking_rewards_claimed_event")]
    fn staking_rewards_claimed_event(&self, #[indexed] staking_sc: &ManagedAddress, #[indexed] amount: &BigUint);

    /// Event emitted when the close factor is updated.
    #[event("new_close_factor_event")]
    fn new_close_factor_event(&self, #[indexed] old_close_factor: &BigUint, #[indexed] new_close_factor: &BigUint);

    /// Event emitted when the liquidation incentive is updated.
    #[event("new_liquidation_incentive_event")]
    fn new_liquidation_incentive_event(&self, #[indexed] new_liquidation_incentive: &BigUint, #[indexed] old_liquidation_incentive: &BigUint);

    /// Event emitted when the protocol seize share is updated.
    #[event("new_protocol_seize_share_event")]
    fn new_protocol_seize_share_event(&self, #[indexed] old_protocol_seize_share: &BigUint, #[indexed] new_protocol_seize_share: &BigUint);

    /// Event emitted when underlying id is set.
    #[event("set_underlying_id_event")]
    fn set_underlying_id_event(&self, #[indexed] underlying_id: &EgldOrEsdtTokenIdentifier);

    /// Event emitted when initial exchange rate is set.
    #[event("set_initial_exchange_rate_event")]
    fn set_initial_exchange_rate_event(&self, #[indexed] initial_exchange_rate: &BigUint);

    /// Event emitted when accrual timestamp is updated.
    #[event("set_accrual_timestamp_event")]
    fn set_accrual_timestamp_event(&self, #[indexed] timestamp: u64);

    /// Event emitted when accrual time threshold is updated.
    #[event("set_accrual_time_threshold_event")]
    fn set_accrual_time_threshold_event(&self, #[indexed] old_accrual_time_threshold: u64, #[indexed] new_accrual_time_threshold: u64);

    /// Emitted when a trusted minter is added.
    #[event("add_trusted_minter_event")]
    fn add_trusted_minter_event(&self, #[indexed] minter: &ManagedAddress);

    /// Emitted when a trusted minter is removed.
    #[event("remove_trusted_minter_event")]
    fn remove_trusted_minter_event(&self, #[indexed] minter: &ManagedAddress);
}
