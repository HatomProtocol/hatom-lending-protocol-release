multiversx_sc::imports!();

use crate::models::DiscountData;

#[multiversx_sc::module]
pub trait StorageModule {
    /// Stores the Controller address.
    #[view(getController)]
    #[storage_mapper("controller")]
    fn controller(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the USH money market address.
    #[view(getUshMoneyMarket)]
    #[storage_mapper("ush_money_market")]
    fn ush_money_market(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the USH Token Identifier.
    #[view(getUshTokenId)]
    #[storage_mapper("ush_token_id")]
    fn ush_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores whether the money market has discount data or not.
    #[view(hasDiscountData)]
    #[storage_mapper("has_discount_data")]
    fn has_discount_data(&self, money_market: &ManagedAddress) -> SingleValueMapper<bool>;

    /// Stores all the discounts data ordered by discounts.
    #[view(getDiscountsDataList)]
    #[storage_mapper("discounts_data_list")]
    fn discounts_data_list(&self) -> LinkedListMapper<DiscountData<Self::Api>>;

    /// Stores the last fetched exchange rate.
    #[storage_mapper("last_exchange_rate")]
    fn last_exchange_rate(&self, money_market: &ManagedAddress) -> SingleValueMapper<BigUint>;
}
