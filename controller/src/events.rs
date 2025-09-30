multiversx_sc::imports!();

use crate::storage::{RewardsBatch, RewardsBooster};

#[multiversx_sc::module]
pub trait EventModule {
    /// Emitted when a new market is supported.
    #[event("support_money_market_event")]
    fn support_money_market_event(&self, #[indexed] money_market: &ManagedAddress);

    /// Emitted when an account enters a market, i.e. deposits tokens as collateral.
    #[event("enter_market_event")]
    fn enter_market_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] borrower: &ManagedAddress, #[indexed] tokens: &BigUint);

    /// Emitted when an account exits a market, i.e. removes tokens from collateral.
    #[event("exit_market_event")]
    fn exit_market_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] borrower: &ManagedAddress, #[indexed] tokens: &BigUint);

    /// Emitted when an account exits a market and redeems in one shot.
    #[event("exit_market_and_redeem_event")]
    fn exit_market_and_redeem_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] redeemer: &ManagedAddress, #[indexed] underlying_payment: &EgldOrEsdtTokenPayment, #[indexed] token_payment: &EsdtTokenPayment);

    /// Emitted when a new maximum number of markets that can be entered per account is set.
    #[event("new_max_markets_per_account_event")]
    fn new_max_markets_per_account_event(&self, #[indexed] old_max_markets_per_account: usize, #[indexed] new_max_markets_per_account: usize);

    /// Emitted when a booster observer is set.
    #[event("set_booster_observer_event")]
    fn set_booster_observer_event(&self, #[indexed] rewards_booster: &ManagedAddress);

    /// Emitted when the booster observer is cleared.
    #[event("clear_booster_observer_event")]
    fn clear_booster_observer_event(&self, #[indexed] rewards_booster: &ManagedAddress);

    /// Emitted when a USH Market observer is set.
    #[event("set_ush_market_observer_event")]
    fn set_ush_market_observer_event(&self, #[indexed] ush_market: &ManagedAddress);

    /// Emitted when the USH market observer is cleared.
    #[event("clear_ush_market_observer_event")]
    fn clear_ush_market_observer_event(&self, #[indexed] ush_market: &ManagedAddress);

    /// Emitted when a new collateral factor is defined for a given money market.
    #[event("new_collateral_factor_event")]
    fn new_collateral_factor_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] old: &BigUint, #[indexed] new: &BigUint);

    /// Emitted when a new USH borrower collateral factor is defined for a given money market.
    #[event("new_ush_borrower_collateral_factor_event")]
    fn new_ush_borrower_collateral_factor_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] old: &BigUint, #[indexed] new: &BigUint);

    /// Emitted when next collateral factors are set.
    #[event("new_next_collateral_factors_event")]
    fn new_next_collateral_factors_event(&self, #[indexed] timestamp: u64, #[indexed] next_collateral_factor: &BigUint, #[indexed] next_ush_borrower_collateral_factor: &BigUint);

    /// Emitted when next collateral factors are cleared.
    #[event("clear_next_collateral_factors_event")]
    fn clear_next_collateral_factors_event(&self);

    /// Emitted when the price oracle is modified.
    #[event("new_price_oracle_event")]
    fn new_price_oracle_event(&self, #[indexed] old: &Option<ManagedAddress>, #[indexed] new: &ManagedAddress);

    /// Emitted when a new liquidity cap is defined for a given money market.
    #[event("new_liquidity_cap_event")]
    fn new_liquidity_cap_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] old: &Option<BigUint>, #[indexed] new: &BigUint);

    /// Emitted when a new borrow cap is defined for a given money market.
    #[event("new_borrow_cap_event")]
    fn new_borrow_cap_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] old: &Option<BigUint>, #[indexed] new: &BigUint);

    /// Emitted when a new maximum amount of rewards batches is defined for a given money market.
    #[event("new_max_rewards_batches_event")]
    fn new_max_rewards_batches_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] old: usize, #[indexed] new: usize);

    /// Emitted when a new maximum slippage is defined.
    #[event("new_max_slippage_event")]
    fn new_max_slippage_event(&self, #[indexed] old: &BigUint, #[indexed] new: &BigUint);

    /// Emitted when a new guardian is set.
    #[event("new_pause_guardian_event")]
    fn new_pause_guardian_event(&self, #[indexed] old: &Option<ManagedAddress>, #[indexed] new: &ManagedAddress);

    /// Emitted when a new rewards manager is set.
    #[event("new_rewards_manager_event")]
    fn new_rewards_manager_event(&self, #[indexed] old: &Option<ManagedAddress>, #[indexed] new: &ManagedAddress);

    /// Event emitted when mint is paused or unpaused.
    #[event("mint_paused_event")]
    fn mint_paused_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] paused: bool);

    /// Event emitted when borrow is paused or unpaused.
    #[event("borrow_paused_event")]
    fn borrow_paused_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] paused: bool);

    /// Event emitted when seize is paused or unpaused.
    #[event("seize_paused_event")]
    fn seize_paused_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] paused: bool);

    /// Event emitted when global seize is paused or unpaused.
    #[event("global_seize_paused_event")]
    fn global_seize_paused_event(&self, #[indexed] paused: bool);

    /// Event emitted when supplier rewards are distributed.
    #[event("supplier_rewards_distributed_event")]
    fn supplier_rewards_distributed_event(&self, #[indexed] supplier: &ManagedAddress, #[indexed] rewards_batch: &RewardsBatch<Self::Api>, #[indexed] delta_rewards: &BigUint);

    /// Event emitted when borrower rewards are distributed.
    #[event("borrower_rewards_distributed_event")]
    fn borrower_rewards_distributed_event(&self, #[indexed] borrower: &ManagedAddress, #[indexed] rewards_batch: &RewardsBatch<Self::Api>, #[indexed] delta_rewards: &BigUint);

    /// Event emitted when rewards are claimed by a user.
    #[event("rewards_claimed_event")]
    fn rewards_claimed_event(&self, #[indexed] claimer: &ManagedAddress, #[indexed] rewards_batch: &RewardsBatch<Self::Api>, #[indexed] claimed_amount: &BigUint);

    /// Event emitted when user rewards are claimed.
    #[event("rewards_token_claimed_event")]
    fn rewards_token_claimed_event(&self, #[indexed] claimer: &ManagedAddress, #[indexed] rewards_token_id: &EgldOrEsdtTokenIdentifier, #[indexed] claimed_amount: &BigUint);

    /// Event emitted when a rewards batch is set.
    #[event("set_rewards_batch_event")]
    fn set_rewards_batch_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] rewards_batch: &RewardsBatch<Self::Api>);

    /// Event emitted when a rewards batch adds more rewards.
    #[event("add_rewards_batch_event")]
    fn add_rewards_batch_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] rewards_batch: &RewardsBatch<Self::Api>);

    /// Event emitted when a rewards batch is cancelled.
    #[event("cancel_rewards_batch_event")]
    fn cancel_rewards_batch_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] rewards_batch: &RewardsBatch<Self::Api>);

    /// Event emitted when a rewards batch is removed.
    #[event("remove_rewards_batch_event")]
    fn remove_rewards_batch_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] batch_id: usize);

    /// Event emitted when the rewards batch speed is updated.
    #[event("update_rewards_batch_speed_event")]
    fn update_rewards_batch_speed_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] rewards_batch: &RewardsBatch<Self::Api>);

    /// Event emitted when the remaining period of a rewards batch is updated.
    #[event("update_rewards_batch_remaining_period_event")]
    fn update_rewards_batch_remaining_period_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] rewards_batch: &RewardsBatch<Self::Api>);

    /// Event emitted when the undistributed rewards are claimed.
    #[event("claim_undistributed_rewards_event")]
    fn claim_undistributed_rewards_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] rewards_token_id: &EgldOrEsdtTokenIdentifier, #[indexed] claimed_amount: &BigUint);

    /// Event emitted when the supply rewards batch index is updated.
    #[event("supply_rewards_batches_updated_event")]
    fn supply_rewards_batches_updated_event(&self, #[indexed] rewards_batch: &RewardsBatch<Self::Api>);

    /// Event emitted when the borrow rewards batch index is updated.
    #[event("borrow_rewards_batches_updated_event")]
    fn borrow_rewards_batches_updated_event(&self, #[indexed] rewards_batch: &RewardsBatch<Self::Api>);

    /// Event emitted when rewards batch boosting is supported.
    #[event("support_rewards_batch_boosting_event")]
    fn support_rewards_batch_boosting_event(&self);

    /// Event emitted when rewards batch boosting is enabled.
    #[event("enable_rewards_batch_boosting_event")]
    fn enable_rewards_batch_boosting_event(&self);

    /// Event emitted when rewards batch boosting is disabled.
    #[event("disable_rewards_batch_boosting_event")]
    fn disable_rewards_batch_boosting_event(&self);

    /// Event emitted when rewards are boosted for a specific rewards token.
    #[event("boost_rewards_event")]
    fn boost_rewards_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] rewards_batch_booster: &RewardsBooster<Self::Api>);

    /// Event emitted when a booster is updated for a specific rewards token.
    #[event("update_booster_event")]
    fn update_booster_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] rewards_batch_booster: &RewardsBooster<Self::Api>);

    /// Event emitted when a booster is cancelled for a specific rewards token.
    #[event("cancel_booster_event")]
    fn cancel_booster_event(&self, #[indexed] caller: &ManagedAddress, #[indexed] token_id: &EgldOrEsdtTokenIdentifier);

    /// Event emitted when boosted rewards are claimed.
    #[event("boosted_rewards_claimed_event")]
    fn boosted_rewards_claimed_event(&self, #[indexed] claimer: &ManagedAddress, #[indexed] rewards_batch_booster: &RewardsBooster<Self::Api>, #[indexed] claimed_amount: &BigUint);
}
