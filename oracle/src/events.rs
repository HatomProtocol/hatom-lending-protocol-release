multiversx_sc::imports!();

use crate::model::*;

#[multiversx_sc::module]
pub trait EventsModule {
    /// Emitted when a new guardian is set.
    #[event("new_guardian_event")]
    fn new_guardian_event(&self, #[indexed] old: &Option<ManagedAddress>, #[indexed] new: &ManagedAddress);

    /// Event emitted when a new token is supported.
    #[event("support_token_event")]
    fn support_token_event(&self, #[indexed] token_data: &TokenData<Self::Api>);

    /// Event emitted when the Liquid Staked EGLD token is supported.
    #[event("support_ls_token_event")]
    fn support_ls_token_event(&self, #[indexed] token_identifier: &TokenIdentifier);

    /// Event emitted when the Liquid Staked TAO token is supported.
    #[event("support_stao_token_event")]
    fn support_stao_token_event(&self, #[indexed] token_identifier: &TokenIdentifier);

    /// Event emitted when the USH token is supported.
    #[event("support_ush_token_event")]
    fn support_ush_token_event(&self, #[indexed] ush_minter: &ManagedAddress, #[indexed] ush_token_data: &TokenData<Self::Api>);

    /// Event emitted when the USH fallback token is set.
    #[event("set_ush_fallback_token_event")]
    fn set_ush_fallback_token_event(&self, #[indexed] token_id: &TokenIdentifier);

    /// Event emitted when the Price Aggregator smart contract is supported as a price source.
    #[event("support_price_aggregator_event")]
    fn support_price_aggregator_event(&self, #[indexed] price_aggregator_address: &ManagedAddress);

    /// Event emitted when the round duration is changed.
    #[event("updated_round_duration_event")]
    fn updated_round_duration_event(&self, #[indexed] round_duration: u64);

    /// Event emitted when a token pricing is unpaused.
    #[event("unpause_token_event")]
    fn unpause_token_event(&self, token_id: &TokenIdentifier);

    /// Event emitted when a token pricing is paused.
    #[event("pause_token_event")]
    fn pause_token_event(&self, token_id: &TokenIdentifier);

    /// Event emitted when a token pricing method is set to either Instantaneous, Safe or Price Aggregator.
    #[event("unreliable_pricing_method_event")]
    fn unreliable_pricing_method_event(&self, #[indexed] token_id: &TokenIdentifier, #[indexed] pricing_method: &PricingMethod);

    /// Event emitted when the pricing method for a token is changed.
    #[event("pricing_method_event")]
    fn pricing_method_event(&self, #[indexed] token_id: &TokenIdentifier, #[indexed] pricing_method: &PricingMethod);

    /// Event emitted when the EGLD Wrapper smart contract is set.
    #[event("set_egld_wrapper_event")]
    fn set_egld_wrapper_event(&self, #[indexed] egld_wrapper: &ManagedAddress, #[indexed] wegld_id: &TokenIdentifier);

    /// Event emitted when the xExchange pricing method is set.
    #[event("set_xexchange_pricing_method_event")]
    fn set_xexchange_pricing_method_event(&self, #[indexed] xexchange_pricing_method: ExchangePricingMethod);

    /// Event emitted when the first and last anchor tolerances for a token are changed.
    #[event("anchor_tolerances_event")]
    fn anchor_tolerances_event(&self, #[indexed] token_id: &TokenIdentifier, #[indexed] tolerances: &ToleranceData<Self::Api>);

    /// Event emitted when the instantaneous price of a token is fetched from xExchange.
    #[event("xexchange_price_fetched_event")]
    fn xexchange_price_fetched_event(&self, #[indexed] token_id: &TokenIdentifier, #[indexed] new_price: &BigUint);

    /// Event emitted when the safe price of a token is fetched from xExchange.
    #[event("xexchange_safe_price_fetched_event")]
    fn xexchange_safe_price_fetched_event(&self, #[indexed] token_id: &TokenIdentifier, #[indexed] new_price: &BigUint);

    /// Event emitted when the price of a token is fetched from the Price Aggregator.
    #[event("price_aggregator_price_fetched_event")]
    fn price_aggregator_price_fetched_event(&self, #[indexed] token_id: &TokenIdentifier, #[indexed] new_price: &BigUint);

    /// Event emitted when the Price Aggregator price of a token has not been updated for a round duration.
    #[event("price_aggregator_price_too_old_event")]
    fn price_aggregator_price_too_old_event(&self, #[indexed] from: &ManagedBuffer, #[indexed] to: &ManagedBuffer, #[indexed] price: &BigUint);

    /// Event emitted when the reported price of a token is outside of the first anchor bounds.
    #[event("first_anchor_surpassed_event")]
    fn first_anchor_surpassed_event(&self, #[indexed] token_id: &TokenIdentifier, #[indexed] reporter_price: &BigUint, #[indexed] anchor_price: &BigUint);

    /// Event emitted when the reported price of a token is outside of the last anchor bounds.
    #[event("last_anchor_surpassed_event")]
    fn last_anchor_surpassed_event(&self, #[indexed] token_id: &TokenIdentifier, #[indexed] reporter_price: &BigUint, #[indexed] anchor_price: &BigUint);

    /// Event emitted when the last reported price of a token is updated.
    #[event("last_price_event")]
    fn last_price_event(&self, #[indexed] token_id: &TokenIdentifier, #[indexed] price: &BigUint);
}
