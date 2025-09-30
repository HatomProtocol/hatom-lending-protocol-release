multiversx_sc::imports!();

use super::{commons, events, proxies, storage};
use crate::{
    constants::*,
    errors::*,
    models::{DiscountData, ExchangeRateType},
};

#[multiversx_sc::module]
pub trait GovernanceModule: admin::AdminModule + commons::CommonsModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Sets the discount for a specific money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The money market address.
    /// - `discount` - The discount in bps.
    ///
    #[endpoint(setDiscountData)]
    fn set_discount_data(&self, money_market: ManagedAddress, discount: BigUint) {
        self.require_admin();

        require!(self.discounts_data_list().len() <= MAX_DISCOUNTS, ERROR_TOO_MANY_DISCOUNTS);
        require!(self.is_whitelisted_money_market(&money_market), ERROR_NON_WHITELISTED_MARKET);
        require!(!self.has_discount_data(&money_market).get(), ERROR_DISCOUNT_DATA_ALREADY_SET);

        require!(discount > BigUint::zero() && discount <= BigUint::from(BPS), ERROR_INVALID_DISCOUNT);

        let (underlying_id, _) = self.get_money_market_identifiers(&money_market);
        let discount_data = DiscountData { money_market: money_market.clone(), underlying_id, discount: discount.clone() };
        self.has_discount_data(&money_market).set(true);
        self.insert_discount_data_in_list(discount_data);

        // store a exchange rate
        self.get_exchange_rate(&money_market, ExchangeRateType::Updated);

        self.set_discount_data_event(&money_market, &discount);
    }

    /// Removes the discount data for a specific money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The money market address.
    ///
    #[endpoint(removeDiscountData)]
    fn remove_discount_data(&self, money_market: ManagedAddress) {
        self.require_admin();

        require!(self.has_discount_data(&money_market).get(), ERROR_DISCOUNT_DATA_UNSET);

        self.has_discount_data(&money_market).set(false);
        self.remove_discount_data_from_list(&money_market);

        self.remove_discount_data_event(&money_market);
    }

    /// Updates the discount of a specific money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The money market address.
    /// - `discount` - The discount in bps.
    ///
    #[endpoint(updateDiscountData)]
    fn update_discount_data(&self, money_market: ManagedAddress, discount: BigUint) {
        self.require_admin();
        self.remove_discount_data(money_market.clone());
        self.set_discount_data(money_market, discount);
    }

    /// Inserts discount data in the list of discounts data sorted by discount in descending order (largest discount first).
    ///
    fn insert_discount_data_in_list(&self, discount_data: DiscountData<Self::Api>) {
        let mut discounts_list = self.discounts_data_list();

        if discounts_list.is_empty() {
            discounts_list.push_front(discount_data);
            return;
        }

        let discount = &discount_data.discount;
        for node in discounts_list.iter() {
            let node_id = node.get_node_id();
            let node_data = node.into_value();
            if discount >= &node_data.discount {
                discounts_list.push_before_node_id(node_id, discount_data);
                return;
            }
        }

        // if not set, the discount is the smallest
        discounts_list.push_back(discount_data);
    }

    /// Removes the discount data from the list of discounts data for a specific money market.
    ///
    fn remove_discount_data_from_list(&self, money_market: &ManagedAddress) {
        for node in self.discounts_data_list().iter() {
            let node_id = node.get_node_id();
            let node_data = node.into_value();
            if money_market == &node_data.money_market {
                self.discounts_data_list().remove_node_by_id(node_id);
                break;
            }
        }
    }
}
