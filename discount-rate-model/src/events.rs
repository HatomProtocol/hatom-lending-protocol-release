multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    /// Event emitted when the USH money market and Controller addresses are set.
    #[event("set_ush_money_market_event")]
    fn set_ush_money_market_event(&self, #[indexed] ush_money_market: &ManagedAddress, #[indexed] controller: &ManagedAddress);

    /// Event emitted when new discount data is added.
    #[event("set_discount_data_event")]
    fn set_discount_data_event(&self, #[indexed] money_market: &ManagedAddress, #[indexed] discount: &BigUint);

    /// Event emitted when discount data is removed.
    #[event("remove_discount_data_event")]
    fn remove_discount_data_event(&self, #[indexed] money_market: &ManagedAddress);
}
