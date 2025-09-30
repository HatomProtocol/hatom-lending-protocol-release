multiversx_sc::imports!();

use super::{common, errors::*, events, proxies, storage};

#[multiversx_sc::module]
pub trait StakingModule: common::CommonModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Claims staking rewards from the staking contract, and sends them to the caller's account.
    ///
    /// This function accrues interest then retrieves the amount of staking rewards and checks if there are any rewards to
    /// claim. If there are rewards, it transfers them from the protocol's cash balance to the caller's account, updates the
    /// reserve and staking rewards balance.
    ///
    #[endpoint(claimStakingRewards)]
    fn claim_staking_rewards(&self) {
        let staking_sc = match self.get_staking_contract() {
            None => sc_panic!(ERROR_UNDEFINED_STAKING_SC),
            Some(address) => address,
        };

        self.accrue_interest();
        self.require_market_fresh();

        let cash = self.cash().get();
        let staking_rewards = self.staking_rewards().get();

        // do nothing
        if staking_rewards == BigUint::zero() {
            return;
        }

        require!(staking_rewards <= cash, ERROR_INSUFFICIENT_BALANCE);

        self.total_reserves().update(|amount| *amount -= &staking_rewards);
        self.staking_rewards().update(|amount| *amount -= &staking_rewards);
        self.cash().update(|amount| *amount -= &staking_rewards);

        let (underlying_id, _) = self.get_money_market_identifiers();
        self.send().direct(&staking_sc, &underlying_id, 0, &staking_rewards);

        self.emit_updated_rates();
        self.staking_rewards_claimed_event(&staking_sc, &staking_rewards);
    }
}
