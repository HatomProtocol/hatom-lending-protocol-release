multiversx_sc::imports!();

use super::{events, proxies, storage};
use crate::{errors::*, models::*};

#[multiversx_sc::module]
pub trait CommonsModule: events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// A utility function to highlight that this smart contract is a Discount Rate Model.
    ///
    #[view(isDiscountRateModel)]
    fn is_discount_rate_model(&self) -> bool {
        true
    }

    /// Checks whether the specified smart contract address is a USH Money Market.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    #[inline]
    fn is_ush_market_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_ush_market(sc_address)
    }

    /// Gets the discount data for all markets that have been granted a discount.
    ///
    #[view(getDiscountsData)]
    fn get_discounts_data(&self) -> MultiValueEncoded<DiscountData<Self::Api>> {
        let mut discounts_data = MultiValueEncoded::new();
        for node in self.discounts_data_list().iter() {
            let discount_data = node.into_value();
            discounts_data.push(discount_data);
        }

        discounts_data
    }

    /// Returns the current number of discounts.
    ///
    #[view(getDiscountsCount)]
    fn get_discounts_count(&self) -> usize {
        self.discounts_data_list().len()
    }

    /// Retrieves the exchange rate of a money market based on the specified method.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - Address of the money market contract.
    /// - `fx_type` - The method to use for fetching the exchange rate (`Cached` or `Updated`).
    ///
    /// # Notes:
    ///
    /// - When using `Cached`, the function returns the exchange rate stored previously in the `last_exchange_rate`.
    /// - When using `Updated`, the function fetches the latest exchange rate, stores it as the new `last_exchange_rate`, and returns the updated value.
    ///
    #[endpoint(getExchangeRate)]
    fn get_exchange_rate(&self, money_market: &ManagedAddress, fx_type: ExchangeRateType) -> BigUint {
        match fx_type {
            ExchangeRateType::Cached => {
                // this cannot happen, we always have at least one last exchange rate stored at setup
                require!(!self.last_exchange_rate(money_market).is_empty(), ERROR_MISSING_LAST_EXCHANGE_RATE);
                self.last_exchange_rate(money_market).get()
            },
            ExchangeRateType::Updated => {
                let exchange_rate = self.get_stored_exchange_rate(money_market);
                self.last_exchange_rate(money_market).set(&exchange_rate);

                exchange_rate
            },
        }
    }

    /// Fetches the Controller smart contract address from the given USH Money Market smart contract and sets both the USH
    /// Money Market and Controller addresses into the storage. The USH Money Market is required when setting the discount
    /// rate model at the USH Money Market.
    ///
    /// # Arguments:
    ///
    /// - `ush_money_market` - The USH Money Market smart contract address.
    ///
    fn set_ush_money_market(&self, ush_money_market: &ManagedAddress) {
        require!(self.is_ush_market_sc(ush_money_market), ERROR_INVALID_USH_MONEY_MARKET_SC);
        let (ush_token_id, _) = self.get_money_market_identifiers(ush_money_market);

        self.ush_money_market().set(ush_money_market);
        self.ush_token_id().set(ush_token_id.unwrap_esdt());

        let opt_controller = self.get_controller();
        require!(opt_controller.is_some(), ERROR_INVALID_CONTROLLER_SC);
        let controller = opt_controller.unwrap();

        self.controller().set(&controller);

        self.set_ush_money_market_event(ush_money_market, &controller);
    }
}
