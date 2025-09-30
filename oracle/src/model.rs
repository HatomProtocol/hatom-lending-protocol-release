multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Debug)]
pub enum PricingMethod {
    None,
    Default,
    Instantaneous,
    Safe,
    PriceAggregator,
}

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Clone, Copy, Debug)]
pub enum ExchangePricingMethod {
    None,
    SafePriceOnly,
    InstantaneousPriceOnly,
    All,
}

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Copy, Clone, Debug)]
pub enum PairState {
    Inactive,
    Active,
    PartialActive,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq)]
pub enum TokenType {
    None,
    Native,
    Synthetic,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq)]
pub struct TokenData<M: ManagedTypeApi> {
    pub token_type: TokenType,
    pub identifier: TokenIdentifier<M>,
    pub unit_price: BigUint<M>,
    pub ticker: ManagedBuffer<M>,
    pub decimals: usize,
    pub exp: BigUint<M>,
    pub xexchange_pair: Option<ExchangePair<M>>,
    pub tolerances: Option<ToleranceData<M>>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq)]
pub struct ExchangePair<M: ManagedTypeApi> {
    pub address: ManagedAddress<M>,
    pub token0: TokenIdentifier<M>,
    pub token1: TokenIdentifier<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Clone)]
pub struct ToleranceData<M: ManagedTypeApi> {
    pub first_upper_bound_ratio: BigUint<M>,
    pub first_lower_bound_ratio: BigUint<M>,
    pub last_upper_bound_ratio: BigUint<M>,
    pub last_lower_bound_ratio: BigUint<M>,
}
