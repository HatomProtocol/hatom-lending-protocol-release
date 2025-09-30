multiversx_sc::imports!();

use super::{commons, events, proxies, storage};

#[multiversx_sc::module]
pub trait StakingModule: commons::CommonsModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Claims staking rewards from the staking contract.
    ///
    #[endpoint(claimStakingRewards)]
    fn claim_staking_rewards(&self) {
        self.require_staking_sc();

        self.accrue_interest();
        self.require_market_fresh();

        // do nothing
        let staking_rewards = self.staking_rewards().get();
        if staking_rewards == BigUint::zero() {
            return;
        }

        // update reserves and staking rewards
        self.total_reserves().update(|amount| *amount -= &staking_rewards);
        self.staking_rewards().update(|amount| *amount -= &staking_rewards);

        // mint USH to staking contract
        let staking_sc = self.staking_sc().get();
        self.ush_minter_mint(&staking_rewards, OptionalValue::Some(staking_sc));

        self.staking_rewards_claimed_event(&staking_rewards);
    }
}
