multiversx_sc::imports!();

use super::{
    borrow, commons, events, governance, liquidate, proxies, repay_borrow, seize, staking,
    storage::{self, DiscountStrategy, InteractionType},
};

#[multiversx_sc::module]
pub trait ObserverModule: admin::AdminModule + borrow::BorrowModule + commons::CommonsModule + events::EventsModule + governance::GovernanceModule + liquidate::LiquidateModule + proxies::ProxyModule + repay_borrow::RepayBorrowModule + seize::SeizeModule + staking::StakingModule + storage::StorageModule {
    /// This endpoint is called by the Controller smart contract whenever an account changes its collateral.
    ///
    /// # Arguments:
    ///
    /// - `account`: The account address.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the Controller smart contract.
    ///
    #[endpoint(onMarketChange)]
    fn on_market_change(&self, account: &ManagedAddress) {
        self.require_controller();

        if self.is_finalized() {
            return;
        }

        if !self.market_borrowers().contains(account) {
            return;
        }

        self.accrue_interest();

        self.update_borrows_data(account, &BigUint::zero(), InteractionType::EnterOrExitMarket, DiscountStrategy::CachedExchangeRate);
    }
}
