multiversx_sc::imports!();

use super::{common, constants::*, errors::*, events, proxies, storage};

#[multiversx_sc::module]
pub trait SeizeModule: common::CommonModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Handler for `seize_internal` via smart contract to smart contract calls.
    ///
    /// # Arguments:
    ///
    /// - `liquidator` - The account retrieving the seized collateral.
    /// - `borrower` - The account having collateral seized.
    /// - `tokens_to_seize` - The tokens to seize.
    ///
    #[endpoint(seize)]
    fn seize(&self, liquidator: &ManagedAddress, borrower: &ManagedAddress, tokens_to_seize: &BigUint) -> EsdtTokenPayment {
        self.accrue_interest();
        let borrow_market = self.blockchain().get_caller();
        self.seize_internal(&borrow_market, liquidator, borrower, tokens_to_seize)
    }

    /// Transfers collateral tokens to the liquidator.
    ///
    /// # Arguments:
    ///
    /// - `borrow_market` - The money market seizing the collateral tokens, in which the repayment has been done.
    /// - `liquidator` - The account receiving the seized collateral tokens.
    /// - `borrower` - The account having collateral seized.
    /// - `tokens_to_seize` - The tokens to seize.
    ///
    fn seize_internal(&self, borrow_market: &ManagedAddress, liquidator: &ManagedAddress, borrower: &ManagedAddress, tokens_to_seize: &BigUint) -> EsdtTokenPayment {
        require!(borrower != liquidator, ERROR_ADDRESSES_MUST_DIFFER);

        let token_id = self.token_id().get();
        let collateral_market = self.blockchain().get_sc_address();

        let seize_allowed = self.seize_allowed(&collateral_market, borrow_market, borrower, liquidator);
        require!(seize_allowed, ERROR_CONTROLLER_REJECTED_LIQUIDATION_SEIZE);

        // update borrowers collateral
        let borrower_collateral_tokens = self.get_account_collateral_tokens(&collateral_market, borrower);
        let new_borrower_collateral_tokens = &borrower_collateral_tokens - tokens_to_seize;
        self.set_account_collateral_tokens(&collateral_market, borrower, &new_borrower_collateral_tokens);

        // for exponential math
        let wad = BigUint::from(WAD);
        let protocol_seize_share = self.protocol_seize_share().get();

        // seized tokens will be transferred to both liquidator and the protocol reserves (redeemed to underlying)
        let protocol_seize_tokens = protocol_seize_share * tokens_to_seize / &wad;
        let liquidator_seize_tokens = tokens_to_seize - &protocol_seize_tokens;

        // At this point, the protocol redeems a portion of the seized Hatom's tokens for underlying, which is added to the
        // reserves. The underlying is already deposited at this money market SC so there is no need to transfer it.
        let delta_reserves = self.tokens_to_underlying_amount(&protocol_seize_tokens);
        self.total_reserves().update(|amount| *amount += &delta_reserves);

        // also, update staking rewards and revenue
        let fs = self.stake_factor().get();
        let delta_rewards = fs * &delta_reserves / &wad;
        let delta_revenue = &delta_reserves - &delta_rewards;

        self.revenue().update(|amount| *amount += &delta_revenue);
        self.staking_rewards().update(|amount| *amount += &delta_rewards);
        self.historical_staking_rewards().update(|amount| *amount += &delta_rewards);

        // Finally, the Hatom's tokens must be burned given that they have been redeemed.
        self.total_supply().update(|tokens| *tokens -= &protocol_seize_tokens);
        self.controller_burn_tokens(&token_id, &protocol_seize_tokens);

        // send Hatom's tokens to liquidator
        let liquidator_payment = EsdtTokenPayment::new(token_id, 0, liquidator_seize_tokens);
        self.controller_transfer_tokens(&liquidator, &liquidator_payment);

        // rates have been updated at this money market
        self.emit_updated_rates();

        liquidator_payment
    }
}
