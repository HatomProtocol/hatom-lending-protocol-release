multiversx_sc::imports!();

use super::{commons, events, proxies, storage};
use crate::{constants::*, models::*};

#[multiversx_sc::module]
pub trait DiscountModule: admin::AdminModule + commons::CommonsModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Gets the effective discount rate for the specified borrower and borrow amount in WAD units.
    ///
    /// # Arguments:
    ///
    /// - `borrower` - The borrower address.
    /// - `borrow` - The borrowed amount.
    ///
    #[endpoint(getAccountDiscount)]
    fn get_account_discount(&self, borrower: &ManagedAddress, borrow: &BigUint, fx_type: ExchangeRateType) -> BigUint {
        if borrow == &BigUint::zero() || self.discounts_data_list().is_empty() {
            return BigUint::zero();
        }
        self.get_account_discount_internal(borrower, borrow, fx_type)
    }

    fn get_account_discount_internal(&self, borrower: &ManagedAddress, borrow: &BigUint, fx_type: ExchangeRateType) -> BigUint {
        let mut borrow_left = borrow.clone();
        let mut total_discounted = BigUint::zero();

        let controller = self.controller().get();
        let oracle = self.get_price_oracle();

        let ush_token_id = EgldOrEsdtTokenIdentifier::esdt(self.ush_token_id().get());
        let p_ush = self.get_underlying_price(&oracle, ush_token_id);

        for node in self.discounts_data_list().iter() {
            let discount_data = node.into_value();
            let DiscountData { money_market, underlying_id, discount } = discount_data;

            let tokens = self.get_account_collateral_tokens(&controller, &money_market, borrower);
            if tokens == BigUint::zero() {
                continue;
            }

            // money market parameters
            let fx = self.get_exchange_rate(&money_market, fx_type);
            let pi = self.get_underlying_price(&oracle, underlying_id);
            let ltv = self.get_ush_borrower_collateral_factor(&controller, &money_market);

            // the amount subject to a discount
            let discounted_amount = tokens * ltv * fx * pi / (&p_ush * WAD * WAD);
            let discounted_amount_eff = BigUint::min(discounted_amount, borrow_left.clone());

            total_discounted += &discounted_amount_eff * &discount / BPS;
            borrow_left -= &discounted_amount_eff;

            if borrow_left == BigUint::zero() {
                break;
            }
        }

        // the effective discount in wad
        total_discounted * WAD / borrow
    }
}
