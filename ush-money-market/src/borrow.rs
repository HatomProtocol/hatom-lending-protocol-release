multiversx_sc::imports!();

use super::{
    commons,
    errors::*,
    events, proxies,
    storage::{self, DiscountStrategy, InteractionType},
};

#[multiversx_sc::module]
pub trait BorrowModule: commons::CommonsModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// A borrower requests USH from the money market.
    ///
    /// # Arguments:
    ///
    /// - `ush_amount` - The requested amount of USH.
    ///
    #[endpoint(borrow)]
    fn borrow(&self, ush_amount: BigUint) -> EsdtTokenPayment<Self::Api> {
        self.require_active();
        self.accrue_interest();
        require!(ush_amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);
        let borrower = self.blockchain().get_caller();
        self.borrow_internal(borrower, ush_amount)
    }

    fn borrow_internal(&self, borrower: ManagedAddress, ush_amount: BigUint) -> EsdtTokenPayment<Self::Api> {
        // check if accrual has been updated
        self.require_market_fresh();

        // check if borrow repayment is allowed
        let money_market = self.blockchain().get_sc_address();
        let borrow_allowed = self.borrow_allowed(&money_market, &borrower, &ush_amount);
        require!(borrow_allowed, ERROR_CONTROLLER_REJECTED_BORROW);

        // update borrow variables
        let (_, borrower_borrow, total_borrows) = self.update_borrows_data(&borrower, &ush_amount, InteractionType::Borrow, DiscountStrategy::UpdatedExchangeRate);

        // mint requested USH
        let ush_payment = self.ush_minter_mint(&ush_amount, OptionalValue::Some(borrower.clone()));

        // keep track of market borrowers
        self.market_borrowers().insert(borrower.clone());

        self.borrow_event(&borrower, &ush_amount, &borrower_borrow, &total_borrows);

        ush_payment
    }
}
