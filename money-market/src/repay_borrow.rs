multiversx_sc::imports!();

use super::{borrow, common, errors::*, events, proxies, storage};

#[multiversx_sc::module]
pub trait RepayBorrowModule: borrow::BorrowModule + common::CommonModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Repays an outstanding borrow to the money market.
    ///
    /// # Arguments:
    ///
    /// - `opt_borrower` - An optional address to repay on behalf of this account.
    ///
    /// Notes:
    ///
    /// - The repayment amount can be higher than the outstanding borrow. In such case, the remainder is returned.
    ///
    #[payable("*")]
    #[endpoint(repayBorrow)]
    fn repay_borrow(&self, opt_borrower: OptionalValue<ManagedAddress>) -> EgldOrEsdtTokenPayment<Self::Api> {
        self.accrue_interest();

        let (underlying_id, paid_underlying_amount) = self.call_value().egld_or_single_fungible_esdt();
        self.require_valid_underlying_payment(&underlying_id, &paid_underlying_amount);

        let payer = self.blockchain().get_caller();
        match opt_borrower {
            OptionalValue::Some(borrower) => {
                require!(borrower != payer, ERROR_ADDRESSES_MUST_DIFFER);
                require!(!borrower.is_zero(), ERROR_CANNOT_BE_ADDRESS_ZERO);
                self.repay_borrow_internal(&payer, &borrower, &paid_underlying_amount)
            },

            OptionalValue::None => self.repay_borrow_internal(&payer, &payer, &paid_underlying_amount),
        }
    }

    /// Handle a borrow repayment.
    ///
    /// # Arguments:
    ///
    /// - `payer` - The account paying out the borrow.
    /// - `borrower` - The account with the debt being paid off.
    /// - `underlying_amount` - The amount of underlying being returned.
    ///
    fn repay_borrow_internal(&self, payer: &ManagedAddress, borrower: &ManagedAddress, paid_underlying_amount: &BigUint) -> EgldOrEsdtTokenPayment<Self::Api> {
        // check if accrual has been updated
        self.require_market_fresh();

        // check if borrow repayment is allowed
        let money_market = self.blockchain().get_sc_address();
        let repay_allowed = self.repay_borrow_allowed(&money_market, borrower);
        require!(repay_allowed, ERROR_CONTROLLER_REJECTED_BORROW_REPAYMENT);

        // Because of truncation errors, it might happen that the total borrows is smaller than the account borrows: maybe
        // all other borrowers have a really small amount of borrow or there are no other borrowers. In this case, we make
        // the account borrows equal the total borrows. All borrowers left will be able to pay their borrows but without
        // actually having to pay anything.
        let current_total_borrows = self.total_borrows().get();
        let borrower_current_borrow_amount = BigUint::min(current_total_borrows.clone(), self.get_account_borrow_amount(borrower));

        let (underlying_amount, underlying_amount_left) = if borrower_current_borrow_amount >= *paid_underlying_amount {
            // use all for borrow repayment and nothing left
            let repaid_underlying_amount = paid_underlying_amount.clone();
            let underlying_amount_left = BigUint::zero();
            (repaid_underlying_amount, underlying_amount_left)
        } else {
            // use a portion for borrow repayment
            let repaid_underlying_amount = borrower_current_borrow_amount.clone();
            let underlying_amount_left = paid_underlying_amount - &borrower_current_borrow_amount;
            (repaid_underlying_amount, underlying_amount_left)
        };

        let borrow_index = self.get_borrow_index();
        let new_borrower_borrow_amount = &borrower_current_borrow_amount - &underlying_amount;
        self.set_account_borrow_snapshot(borrower, &new_borrower_borrow_amount, &borrow_index);

        // update money market borrowed amount
        let new_total_borrows = current_total_borrows - &underlying_amount;
        self.total_borrows().set(&new_total_borrows);

        // update cash
        self.cash().update(|amount| *amount += &underlying_amount);

        let underlying_id = self.underlying_id().get();
        if underlying_amount_left > BigUint::zero() {
            self.send().direct(payer, &underlying_id, 0, &underlying_amount_left);
        }

        self.try_remove_account_market(&money_market, borrower);

        self.emit_updated_rates();
        self.repay_borrow_event(payer, borrower, &underlying_amount, &new_borrower_borrow_amount, &new_total_borrows);

        EgldOrEsdtTokenPayment::new(underlying_id, 0, underlying_amount)
    }
}
