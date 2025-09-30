multiversx_sc::imports!();

use super::{common, errors::*, events, proxies, storage};

#[multiversx_sc::module]
pub trait BorrowModule: common::CommonModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// A borrower requests underlying from the money market.
    ///
    /// # Arguments:
    ///
    /// - `underlying_amount` - The amount of underlying asset the borrower requests.
    ///
    #[endpoint(borrow)]
    fn borrow(&self, underlying_amount: BigUint) -> EgldOrEsdtTokenPayment {
        self.require_active();
        self.accrue_interest();
        require!(underlying_amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);
        let borrower = self.blockchain().get_caller();
        self.borrow_internal(borrower, underlying_amount)
    }

    fn borrow_internal(&self, borrower: ManagedAddress, underlying_amount: BigUint) -> EgldOrEsdtTokenPayment {
        let money_market = self.blockchain().get_sc_address();
        let borrow_allowed = self.borrow_allowed(&money_market, &borrower, &underlying_amount);
        require!(borrow_allowed, ERROR_CONTROLLER_REJECTED_BORROW);

        // check if accrual has been updated
        self.require_market_fresh();

        self.try_ensure_staking_rewards(&underlying_amount);

        // update account borrowed amount
        let borrow_index = self.get_borrow_index();
        let borrower_current_borrow_amount = self.get_account_borrow_amount(&borrower);
        let new_borrower_borrow_amount = &borrower_current_borrow_amount + &underlying_amount;
        self.set_account_borrow_snapshot(&borrower, &new_borrower_borrow_amount, &borrow_index);

        // update cash
        self.cash().update(|amount| *amount -= &underlying_amount);

        // update money market borrowed amount
        let new_total_borrows = self.total_borrows().get() + &underlying_amount;
        self.total_borrows().set(&new_total_borrows);

        // send underlying to borrower
        let underlying_id = self.underlying_id().get();
        self.send().direct(&borrower, &underlying_id, 0, &underlying_amount);

        self.emit_updated_rates();
        self.borrow_event(&borrower, &underlying_amount, &new_borrower_borrow_amount, &new_total_borrows, &borrow_index);

        EgldOrEsdtTokenPayment::new(underlying_id, 0, underlying_amount)
    }
}
