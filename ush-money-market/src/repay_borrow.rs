multiversx_sc::imports!();

use super::{
    borrow, commons,
    errors::*,
    events, proxies,
    storage::{self, DiscountStrategy, InteractionType},
};

#[multiversx_sc::module]
pub trait RepayBorrowModule: borrow::BorrowModule + commons::CommonsModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Repays an outstanding USH borrow to the money market.
    ///
    /// # Arguments:
    ///
    /// - `opt_borrower` - An optional borrower address. Otherwise, caller is assumed to be the borrower.
    ///
    /// Notes:
    ///
    /// - The repayment amount can be higher than the outstanding borrow. In such case, the remainder is returned.
    ///
    #[payable("*")]
    #[endpoint(repayBorrow)]
    fn repay_borrow(&self, opt_borrower: OptionalValue<ManagedAddress>) -> EsdtTokenPayment<Self::Api> {
        self.accrue_interest();

        let (ush_id, ush_payment_amount) = self.call_value().single_fungible_esdt();
        self.require_valid_ush_payment(&ush_id, &ush_payment_amount);

        let caller = self.blockchain().get_caller();
        match opt_borrower {
            OptionalValue::Some(borrower) => {
                require!(borrower != caller, ERROR_ADDRESSES_MUST_DIFFER);
                require!(!borrower.is_zero(), ERROR_CANNOT_BE_ADDRESS_ZERO);
                self.repay_borrow_internal(&caller, &borrower, &ush_payment_amount, DiscountStrategy::UpdatedExchangeRate)
            },

            OptionalValue::None => self.repay_borrow_internal(&caller, &caller, &ush_payment_amount, DiscountStrategy::UpdatedExchangeRate),
        }
    }

    /// Handle a borrow repayment.
    ///
    /// # Arguments:
    ///
    /// - `caller` - The account paying out the borrow.
    /// - `borrower` - The account with the debt being paid off.
    /// - `ush_payment_amount` - The amount of USH being paid.
    /// - `discount_strategy` - Specifies the discount strategy, which varies based on the context (e.g., liquidation or
    ///   repay borrow).
    ///
    fn repay_borrow_internal(&self, caller: &ManagedAddress, borrower: &ManagedAddress, ush_payment_amount: &BigUint, discount_strategy: DiscountStrategy) -> EsdtTokenPayment<Self::Api> {
        // check if accrual has been updated
        self.require_market_fresh();

        // check if borrow repayment is allowed
        let money_market = self.blockchain().get_sc_address();
        let repayment_allowed = self.repay_borrow_allowed(&money_market, borrower);
        require!(repayment_allowed, ERROR_CONTROLLER_REJECTED_BORROW_REPAYMENT);

        // update borrow variables
        let (ush_repayment_amount, borrower_borrow, total_borrows) = self.update_borrows_data(borrower, ush_payment_amount, InteractionType::RepayBorrow, discount_strategy);

        // burn repaid USH. Notice that last borrowers could have a zero repayment amount.
        let ush_id = self.ush_id().get();
        let ush_repayment = EsdtTokenPayment::new(ush_id.clone(), 0, ush_repayment_amount.clone());
        self.ush_minter_burn(&ush_repayment);

        // return amount left back to the caller
        if ush_payment_amount > &ush_repayment_amount {
            let ush_left_amount = ush_payment_amount - &ush_repayment_amount;
            self.send().direct_esdt(caller, &ush_id, 0, &ush_left_amount);
        }

        self.try_remove_market_borrower(borrower);
        self.try_remove_account_market(&money_market, borrower);

        self.repay_borrow_event(caller, borrower, &ush_repayment_amount, &borrower_borrow, &total_borrows);

        ush_repayment
    }
}
