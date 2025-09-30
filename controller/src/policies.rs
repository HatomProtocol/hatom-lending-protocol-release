multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::{errors::*, events, guardian, proxies, rewards, risk_profile, shared, storage};

use crate::storage::Status;

#[multiversx_sc::module]
pub trait PolicyModule: admin::AdminModule + events::EventModule + guardian::GuardianModule + proxies::ProxyModule + shared::SharedModule + rewards::RewardsModule + risk_profile::RiskProfileModule + storage::StorageModule {
    /// Checks whether minting is allowed at a specified money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    /// # Notes:
    ///
    /// - It does not depend on the account that intends to mint.
    /// - Fails with panic and a clear error message or returns true.
    ///
    #[view(mintAllowed)]
    fn mint_allowed(&self, money_market: &ManagedAddress, amount: BigUint) -> bool {
        self.require_whitelisted_money_market(money_market);
        require!(self.get_mint_status(money_market) == Status::Active, ERROR_MINT_PAUSED);

        // check if the liquidity cap (if any) has been reached
        if let Some(cap) = self.get_liquidity_cap(money_market) {
            let liquidity = self.get_liquidity(money_market);
            let new_liquidity = liquidity + amount;
            require!(new_liquidity < cap, ERROR_REACHED_LIQUIDITY_CAP);
        }

        true
    }

    /// Checks whether an account (redeemer) should be allowed to withdraw a given amount of Hatom tokens from a given
    /// market, i.e. withdraw collateral.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `redeemer` - The account that intends to withdraw the tokens.
    /// - `tokens` - The amount of Hatom tokens to withdraw.
    ///
    /// # Notes:
    ///
    /// - This function is not used when redeeming at a money market, it is only used when redeeming (exiting the market) at
    ///   the controller.
    /// - A simulation of the resulting risk profile is performed.
    /// - Fails with panic and a clear error message, returns false if redeemer would become risky or true if she remains
    ///   solvent.
    ///
    #[endpoint(redeemAllowed)]
    fn redeem_allowed(&self, money_market: &ManagedAddress, redeemer: &ManagedAddress, tokens: &BigUint) -> bool {
        self.require_whitelisted_money_market(money_market);

        // the redeemer must have provided enough collateral
        require!(self.get_account_collateral_tokens(money_market, redeemer) >= *tokens, ERROR_NOT_ENOUGH_COLLATERAL_REDEEMER);

        // a risk profile is needed to confirm if the redeeming is possible
        let risk_profile = self.simulate_risk_profile(redeemer, money_market, tokens, &BigUint::zero(), true);

        // check if redeeming is possible
        if !risk_profile.can_redeem() {
            return false;
        }
        self.update_supply_rewards_batches_state(money_market);
        self.distribute_supplier_batches_rewards(money_market, redeemer);
        true
    }

    /// Checks whether an account (borrower) should be allowed to take a borrow of a given amount on a given money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `borrower` - The account that intends to take a borrow.
    /// - `amount` - The amount of underlying to borrow.
    ///
    /// # Notes:
    ///
    /// - Fails with panic and a clear error message, returns false if borrower would become risky or true if she remains
    ///   solvent.
    ///
    #[endpoint(borrowAllowed)]
    fn borrow_allowed(&self, money_market: &ManagedAddress, borrower: &ManagedAddress, amount: &BigUint) -> bool {
        self.require_whitelisted_money_market(money_market);

        require!(self.get_borrow_status(money_market) == Status::Active, ERROR_BORROW_PAUSED);

        // money markets can add accounts to a market. this is needed when an account wants to take a borrow from a market in
        // which it has not entered yet, because the liquidity computation must loop in that market to compute the borrows
        // effect
        if !self.market_members(money_market).contains(borrower) {
            let caller = self.blockchain().get_caller();
            require!(caller == *money_market, ERROR_ONLY_MONEY_MARKET_CALLER);
            self.enter_market_internal(money_market, borrower, &BigUint::zero());
        }

        // check oracle pricing
        self.get_underlying_price(money_market);

        // check if the borrow cap (if any) has been reached
        if let Some(cap) = self.get_borrow_cap(money_market) {
            let total_borrows = self.get_total_borrows(money_market);
            let new_total_borrows = total_borrows + amount;
            require!(new_total_borrows < cap, ERROR_REACHED_BORROW_CAP);
        }

        // a risk profile is needed to confirm if the borrowing is possible
        let risk_profile = self.simulate_risk_profile(borrower, money_market, &BigUint::zero(), amount, true);

        // check if borrowing is possible
        if !risk_profile.can_borrow() {
            return false;
        }
        self.update_borrow_rewards_batches_state(money_market);
        self.distribute_borrower_batches_rewards(money_market, borrower);
        true
    }

    /// Checks whether repaying a borrow is allowed at a specified money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `borrower` - The address of the borrower.
    ///
    /// # Notes:
    ///
    /// - It does not depend on the account that intends to repay the borrow.
    ///
    #[endpoint(repayBorrowAllowed)]
    fn repay_borrow_allowed(&self, money_market: &ManagedAddress, borrower: &ManagedAddress) -> bool {
        if !self.is_whitelisted_money_market(money_market) {
            return false;
        }
        self.update_borrow_rewards_batches_state(money_market);
        self.distribute_borrower_batches_rewards(money_market, borrower);
        true
    }

    /// Checks whether a liquidation is allowed or not to happen, repaying a borrow at a given money market and seizing
    /// collateral at the same or another specified money market.
    ///
    /// # Arguments:
    ///
    /// - `borrow_market` - The money market where the borrower has borrow its underlying.
    /// - `collateral_market` - The money market where the borrower has collateral which is intended to be seized.
    /// - `borrower` - The address of the borrower.
    /// - `amount` - The amount of underlying being repaid by the liquidator.
    ///
    /// # Notes:
    ///
    /// - Borrows at deprecated markets can be fully repaid (the close factor does not play any role).
    /// - Fails with panic and a clear error message, returns false if the borrower cannot be liquidated (i.e. the borrower
    ///   is solvent) or true if the liquidation can be performed (i.e. the borrower is risky and repayment amount does not
    ///   exceeds its maximum allowed).
    ///
    #[endpoint(liquidateBorrowAllowed)]
    fn liquidate_borrow_allowed(&self, borrow_market: &ManagedAddress, collateral_market: &ManagedAddress, borrower: &ManagedAddress, amount: &BigUint) -> bool {
        self.require_whitelisted_money_market(borrow_market);
        self.require_whitelisted_money_market(collateral_market);

        // get the borrower balance
        let borrow_amount = self.get_stored_account_borrow_amount(borrow_market, borrower);

        // allow complete liquidation at deprecated money markets
        if self.is_deprecated(borrow_market) {
            require!(amount <= &borrow_amount, ERROR_REPAYMENT_EXCEEDS_TOTAL_BORROW);
            return true;
        }

        // at non-deprecated markets, borrows can only be repaid if there is risk of insolvency or insolvency
        let risk_profile = self.simulate_risk_profile(borrower, &ManagedAddress::zero(), &BigUint::zero(), &BigUint::zero(), true);

        // also, the maximum repayment amount depends on the close factor
        let close_factor = self.get_close_factor(borrow_market);
        match risk_profile.can_be_liquidated(amount, &borrow_amount, &close_factor) {
            risk_profile::Liquidation::Allowed => true,
            risk_profile::Liquidation::NotAllowed => false,
            risk_profile::Liquidation::AllowedButTooMuch => {
                sc_panic!(ERROR_TOO_MUCH_REPAYMENT)
            },
        }
    }

    /// Checks whether seizing is or not allowed.
    ///
    /// # Arguments:
    ///
    /// - `collateral_market` - The money market where the borrower has collateral which is intended to be seized.
    /// - `borrow_market` - The money market where the borrower has borrow its underlying.
    /// - `borrower` - The address of the borrower.
    /// - `_liquidator` - The address of the liquidator (legacy).
    ///
    /// # Notes:
    ///
    /// - Money markets should be whitelisted and share the same Controller.
    ///
    #[endpoint(seizeAllowed)]
    fn seize_allowed(&self, collateral_market: &ManagedAddress, borrow_market: &ManagedAddress, borrower: &ManagedAddress, _liquidator: &ManagedAddress) -> bool {
        require!(self.get_global_seize_status() == Status::Active, ERROR_GLOBAL_SEIZE_PAUSED);

        self.require_whitelisted_money_market(borrow_market);
        self.require_whitelisted_money_market(collateral_market);

        for money_market in self.account_markets(borrower).iter() {
            require!(self.seize_status(&money_market).get() == Status::Active, ERROR_SEIZE_PAUSED);
        }

        let opt_controller_a = self.get_controller(borrow_market);
        let opt_controller_b = self.get_controller(collateral_market);

        match (opt_controller_a, opt_controller_b) {
            (Some(controller_a), Some(controller_b)) => {
                if controller_a == controller_b {
                    self.update_supply_rewards_batches_state(collateral_market);
                    self.distribute_supplier_batches_rewards(collateral_market, borrower);
                    return true;
                }
                false
            },
            _ => false,
        }
    }
}
