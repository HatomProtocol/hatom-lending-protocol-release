multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct DiscountData<M>
where
    M: ManagedTypeApi,
{
    pub money_market: ManagedAddress<M>,
    pub underlying_id: EgldOrEsdtTokenIdentifier<M>,
    pub discount: BigUint<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Eq, Copy, Clone, Debug)]
pub enum ExchangeRateType {
    Cached,
    Updated,
}
