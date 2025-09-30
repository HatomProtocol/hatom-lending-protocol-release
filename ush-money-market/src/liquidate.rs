multiversx_sc::imports!();

use super::{
    borrow, commons,
    errors::*,
    events, proxies, repay_borrow, seize,
    storage::{self, DiscountStrategy},
};

pub type LiquidateBorrowResultType<BigUint> = MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[multiversx_sc::module]
pub trait LiquidateModule: borrow::BorrowModule + commons::CommonsModule + events::EventsModule + proxies::ProxyModule + repay_borrow::RepayBorrowModule + seize::SeizeModule + storage::StorageModule {
    /// Liquidate a risky borrower by taking her Hatom's tokens deposited as collateral at a specified money market.
    ///
    /// # Arguments:
    ///
    /// - `borrower` - The account to be liquidated.
    /// - `collateral_market ` - The money market in which to seize collateral from the borrower.
    /// - `opt_min_tokens` - The minimum amount of tokens to be seized from the borrower.
    ///
    #[payable("*")]
    #[endpoint(liquidateBorrow)]
    fn liquidate_borrow(&self, borrower: ManagedAddress, collateral_market: ManagedAddress, opt_min_tokens: OptionalValue<BigUint>) -> LiquidateBorrowResultType<Self::Api> {
        self.accrue_interest();
        self.accrue_interest_in_other_money_market(&collateral_market);

        let liquidator = self.blockchain().get_caller();
        let (ush_id, ush_amount) = self.call_value().single_fungible_esdt();
        self.require_valid_ush_payment(&ush_id, &ush_amount);

        self.liquidate_borrow_internal(&liquidator, &borrower, &ush_amount, &collateral_market, opt_min_tokens)
    }

    /// A liquidator performs a liquidation to a given borrowers, seizing her collateral at a discount.
    ///
    /// # Arguments:
    ///
    /// - `liquidator` - The account repaying the borrow and seizing collateral.
    /// - `borrower` - The account to be liquidated.
    /// - `underlying_amount` - The amount of the underlying borrowed to be repaid.
    /// - `collateral_market ` - The money market in which to seize collateral from the borrower.
    ///
    fn liquidate_borrow_internal(&self, liquidator: &ManagedAddress, borrower: &ManagedAddress, ush_amount: &BigUint, collateral_market: &ManagedAddress, opt_min_tokens: OptionalValue<BigUint>) -> LiquidateBorrowResultType<Self::Api> {
        // check if accrual has been updated
        self.require_market_fresh();

        require!(borrower != liquidator, ERROR_CANNOT_LIQUIDATE_YOURSELF);

        let borrow_market = self.blockchain().get_sc_address();
        let liquidation_allowed = self.liquidate_borrow_allowed(&borrow_market, collateral_market, borrower, ush_amount);
        require!(liquidation_allowed, ERROR_CONTROLLER_REJECTED_LIQUIDATION);

        // repay borrowers debt using a previous discount, because it will be updated when seizing collateral from the
        // borrower at `seize_internal`, specifically at `set_account_collateral_tokens`
        self.repay_borrow_internal(liquidator, borrower, ush_amount, DiscountStrategy::PreviousDiscount);

        // compute the number of tokens to seize from the borrower's collateral and check
        let tokens_to_seize = self.tokens_to_seize(&borrow_market, collateral_market, ush_amount);
        let borrower_collateral_tokens = self.get_account_collateral_tokens(collateral_market, borrower);
        require!(tokens_to_seize <= borrower_collateral_tokens, ERROR_TOO_MUCH_LIQUIDATION);

        // the liquidator gets this payment
        let liquidator_seize_tokens = if &borrow_market == collateral_market {
            // if liquidator wants to seize collateral from current money market
            self.seize_internal(collateral_market, liquidator, borrower, &tokens_to_seize)
        } else {
            // otherwise, if liquidator wants to seize collateral from other money market
            self.seize_in_other_money_market(collateral_market, liquidator, borrower, &tokens_to_seize)
        };

        if let Some(min_tokens) = opt_min_tokens.into_option() {
            require!(liquidator_seize_tokens.amount >= min_tokens, ERROR_NOT_ENOUGH_SEIZED_TOKENS);
        }

        // the liquidated account sees this amount of tokens removed from his account
        let total_seize_tokens = EsdtTokenPayment::new(liquidator_seize_tokens.token_identifier.clone(), 0, tokens_to_seize.clone());

        self.liquidate_borrow_event(liquidator, borrower, ush_amount, collateral_market, &tokens_to_seize);

        (liquidator_seize_tokens, total_seize_tokens).into()
    }
}
