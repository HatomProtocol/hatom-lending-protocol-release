multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::model::*;

#[multiversx_sc::module]
pub trait StorageModule {
    ///  Stores wrapped EGLD smart contract address.
    #[view(getEgldWrapper)]
    #[storage_mapper("egld_wrapper")]
    fn egld_wrapper(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the token identifier of the wrapped EGLD token.
    #[view(getWegldId)]
    #[storage_mapper("wegld_id")]
    fn wegld_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores the address of the EGLD liquid staking smart contract.
    #[view(getLiquidStakingAddress)]
    #[storage_mapper("liquid_staking")]
    fn liquid_staking(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the address of the TAO liquid staking smart contract.
    #[view(getTaoLiquidStakingAddress)]
    #[storage_mapper("tao_liquid_staking")]
    fn tao_liquid_staking(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the USH Minter smart contract address.
    #[view(getUshMinter)]
    #[storage_mapper("ush_minter")]
    fn ush_minter(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the token identifier of the liquid staking token.
    #[view(getLsTokenId)]
    #[storage_mapper("ls_token_id")]
    fn ls_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores the token identifier of the liquid staking TAO token.
    #[view(getStaoTokenId)]
    #[storage_mapper("stao_token_id")]
    fn stao_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores the USH token identifier.
    #[view(getUshTokenId)]
    #[storage_mapper("ush_token_id")]
    fn ush_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    // Stores the USH fallback token identifier.
    #[view(getUshFallbackTokenId)]
    #[storage_mapper("ush_fallback_token_id")]
    fn ush_fallback_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores the xExchange pricing method.
    #[view(getXExchangePricingMethod)]
    #[storage_mapper("xexchange_pricing_method")]
    fn xexchange_pricing_method(&self) -> SingleValueMapper<ExchangePricingMethod>;

    /// Stores the guardian address.
    #[view(getGuardian)]
    #[storage_mapper("guardian")]
    fn guardian(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the address of the Price Aggregator.
    #[view(getPriceAggregatorAddress)]
    #[storage_mapper("price_aggregator_address")]
    fn price_aggregator_address(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the effective round duration based on the Price Aggregator round duration.
    #[view(getRoundDuration)]
    #[storage_mapper("round_duration")]
    fn round_duration(&self) -> SingleValueMapper<u64>;

    /// Whitelisted tokens, i.e. supported tokens.
    #[view(getWhitelistedTokens)]
    #[storage_mapper("whitelisted_tokens")]
    fn whitelisted_tokens(&self) -> UnorderedSetMapper<Self::Api, TokenIdentifier>;

    /// Stores the supported tokens.
    #[view(getSupportedTokens)]
    #[storage_mapper("supported_tokens")]
    fn supported_tokens(&self, token_id: &TokenIdentifier) -> SingleValueMapper<TokenData<Self::Api>>;

    /// Stores the pricing method for each token.
    #[view(getPricingMethod)]
    #[storage_mapper("pricing_method")]
    fn pricing_method(&self, token_id: &TokenIdentifier) -> SingleValueMapper<PricingMethod>;

    /// Stores the last reported price for each token.
    #[view(getLastPrice)]
    #[storage_mapper("last_price")]
    fn last_price(&self, token_id: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    /// Stores whether the token has an unreliable price.
    #[view(hasUnreliablePrice)]
    #[storage_mapper("has_unreliable_price")]
    fn has_unreliable_price(&self, token_id: &TokenIdentifier) -> SingleValueMapper<bool>;

    /// Stores whether the token pricing is paused.
    #[view(isPaused)]
    #[storage_mapper("is_token_paused")]
    fn is_token_paused(&self, token_id: &TokenIdentifier) -> SingleValueMapper<bool>;
}
