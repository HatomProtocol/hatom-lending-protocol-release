multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{errors::*, events, model::PairState, storage};

#[multiversx_sc::module]
pub trait ProxyModule: events::EventsModule + storage::StorageModule {
    // Wrapped EGLD

    fn get_wegld_id(&self, egld_wrapper: &ManagedAddress) -> TokenIdentifier {
        self.egld_wrapper_proxy(egld_wrapper.clone()).get_wrapped_egld_token_id().execute_on_dest_context()
    }

    fn is_egld_wrapper_paused(&self) -> bool {
        let egld_wrapper = self.egld_wrapper().get();
        self.egld_wrapper_proxy(egld_wrapper).is_paused().execute_on_dest_context()
    }

    // xExchange

    fn get_xexchange_state(&self, pair_address: &ManagedAddress) -> PairState {
        self.xexchange_proxy(pair_address.clone()).get_state().execute_on_dest_context()
    }

    fn get_xexchange_first_token_id(&self, pair_address: &ManagedAddress) -> TokenIdentifier {
        self.xexchange_proxy(pair_address.clone()).get_first_token_id().execute_on_dest_context()
    }

    fn get_xexchange_second_token_id(&self, pair_address: &ManagedAddress) -> TokenIdentifier {
        self.xexchange_proxy(pair_address.clone()).get_second_token_id().execute_on_dest_context()
    }

    fn get_xexchange_reserves(&self, pair_address: &ManagedAddress) -> (BigUint, BigUint) {
        let (reserves0, reserves1, _) = self.get_xexchange_reserves_and_total_supply(pair_address);
        require!(reserves0 > BigUint::zero() && reserves1 > BigUint::zero(), ERROR_PAIR_RESERVES);
        (reserves0, reserves1)
    }

    fn get_xexchange_reserves_and_total_supply(&self, pair_address: &ManagedAddress) -> (BigUint, BigUint, BigUint) {
        let result: MultiValue3<BigUint, BigUint, BigUint> = self.xexchange_proxy(pair_address.clone()).get_reserves_and_total_supply().execute_on_dest_context();
        result.into_tuple()
    }

    fn update_and_get_xexchange_safe_price(&self, pair_address: &ManagedAddress, input: EsdtTokenPayment<Self::Api>) -> BigUint {
        let result: EsdtTokenPayment<Self::Api> = self.xexchange_proxy(pair_address.clone()).update_and_get_safe_price(input).execute_on_dest_context();
        let price = result.amount;
        require!(price > BigUint::zero(), ERROR_PRICE_IS_ZERO);
        price
    }

    // Price Aggregator

    fn get_round_duration(&self) -> u64 {
        let price_aggregator_address = self.price_aggregator_address().get();
        self.price_aggregator_proxy(price_aggregator_address).get_round_duration().execute_on_dest_context()
    }

    fn get_price_aggregator_latest_price(&self, from: &ManagedBuffer, to: &ManagedBuffer) -> BigUint {
        let (_, _, _, timestamp, price, _) = self.get_price_aggregator_latest_price_feed(from, to);

        require!(price > BigUint::zero(), ERROR_PRICE_IS_ZERO);

        let t = self.blockchain().get_block_timestamp();
        let round_duration = self.round_duration().get();
        if t - timestamp > round_duration {
            self.price_aggregator_price_too_old_event(from, to, &price);
        }

        price
    }

    fn get_price_aggregator_latest_price_feed(&self, from: &ManagedBuffer, to: &ManagedBuffer) -> (u32, ManagedBuffer, ManagedBuffer, u64, BigUint, u8) {
        let price_aggregator_address = self.price_aggregator_address().get();
        let result: MultiValue6<u32, ManagedBuffer, ManagedBuffer, u64, BigUint, u8> = self.price_aggregator_proxy(price_aggregator_address).latest_price_feed(from, to).execute_on_dest_context();
        result.into_tuple()
    }

    // Liquid Staking

    fn is_liquid_staking(&self, sc_address: &ManagedAddress) -> bool {
        self.liquid_staking_proxy(sc_address.clone()).is_liquid_staking().execute_on_dest_context()
    }

    fn get_ls_token_id(&self) -> TokenIdentifier {
        let liquid_staking = self.liquid_staking().get();
        self.liquid_staking_proxy(liquid_staking).get_ls_token_id().execute_on_dest_context()
    }

    fn get_ls_token_price(&self) -> BigUint {
        let liquid_staking = self.liquid_staking().get();
        self.liquid_staking_proxy(liquid_staking).get_exchange_rate().execute_on_dest_context()
    }

    // TAO Liquid Staking

    fn is_tao_liquid_staking(&self, sc_address: &ManagedAddress) -> bool {
        self.tao_liquid_staking_proxy(sc_address.clone()).is_tao_liquid_staking().execute_on_dest_context()
    }

    fn get_tao_token_id(&self) -> TokenIdentifier {
        let tao_liquid_staking = self.tao_liquid_staking().get();
        self.tao_liquid_staking_proxy(tao_liquid_staking).get_token_id().execute_on_dest_context()
    }

    fn get_stao_token_id(&self) -> TokenIdentifier {
        let tao_liquid_staking = self.tao_liquid_staking().get();
        self.tao_liquid_staking_proxy(tao_liquid_staking).get_ls_token_id().execute_on_dest_context()
    }

    fn get_tao_ls_exchange_rate(&self) -> BigUint {
        let tao_liquid_staking = self.tao_liquid_staking().get();
        self.tao_liquid_staking_proxy(tao_liquid_staking).get_exchange_rate().execute_on_dest_context()
    }

    // USH Minter Calls

    fn is_ush_minter(&self, ush_minter: &ManagedAddress) -> bool {
        self.ush_minter_proxy(ush_minter.clone()).is_ush_minter().execute_on_dest_context()
    }

    fn get_ush_id(&self, ush_minter: &ManagedAddress) -> TokenIdentifier {
        self.ush_minter_proxy(ush_minter.clone()).get_ush_id().execute_on_dest_context()
    }

    #[proxy]
    fn egld_wrapper_proxy(&self, sc_address: ManagedAddress) -> egld_wrapper_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn xexchange_proxy(&self, sc_address: ManagedAddress) -> xexchange_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn price_aggregator_proxy(&self, sc_address: ManagedAddress) -> price_aggregator_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn liquid_staking_proxy(&self, sc_address: ManagedAddress) -> liquid_staking_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn tao_liquid_staking_proxy(&self, sc_address: ManagedAddress) -> tao_liquid_staking_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn ush_minter_proxy(&self, sc_address: ManagedAddress) -> ush_minter_mod::ProxyTo<Self::Api>;
}
mod egld_wrapper_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait EgldWrapper {
        #[view(isPaused)]
        fn is_paused(&self) -> bool;

        #[view(getWrappedEgldTokenId)]
        fn get_wrapped_egld_token_id(&self) -> TokenIdentifier;
    }
}

pub mod xexchange_mod {
    multiversx_sc::imports!();
    multiversx_sc::derive_imports!();
    use crate::model::PairState;

    #[multiversx_sc::proxy]
    pub trait Pair {
        #[view(getState)]
        fn get_state(&self) -> PairState;

        #[view(getFirstTokenId)]
        fn get_first_token_id(&self) -> TokenIdentifier;

        #[view(getSecondTokenId)]
        fn get_second_token_id(&self) -> TokenIdentifier;

        #[view(getReservesAndTotalSupply)]
        fn get_reserves_and_total_supply(&self) -> MultiValue3<BigUint, BigUint, BigUint>;

        #[endpoint(updateAndGetSafePrice)]
        fn update_and_get_safe_price(&self, input: EsdtTokenPayment<Self::Api>) -> EsdtTokenPayment<Self::Api>;
    }
}

mod price_aggregator_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait PriceAggregator {
        #[view(getRoundDuration)]
        fn get_round_duration(&self) -> u64;

        #[view(latestPriceFeed)]
        fn latest_price_feed(&self, from: &ManagedBuffer, to: &ManagedBuffer) -> SCResult<MultiValue6<u32, ManagedBuffer, ManagedBuffer, u64, BigUint, u8>>;
    }
}

mod liquid_staking_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait LiquidStaking {
        #[view(isLiquidStaking)]
        fn is_liquid_staking(&self) -> bool;

        #[view(getLsTokenId)]
        fn get_ls_token_id(&self) -> TokenIdentifier;

        #[view(getExchangeRate)]
        fn get_exchange_rate(&self) -> BigUint;
    }
}

mod tao_liquid_staking_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait TaoLiquidStaking {
        #[view(isTaoLiquidStaking)]
        fn is_tao_liquid_staking(&self) -> bool;

        #[view(getTokenId)]
        fn get_token_id(&self) -> TokenIdentifier;

        #[view(getLsTokenId)]
        fn get_ls_token_id(&self) -> TokenIdentifier;

        #[view(getExchangeRate)]
        fn get_exchange_rate(&self) -> BigUint;
    }
}

mod ush_minter_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait UshMinter {
        #[view(isUshMinter)]
        fn is_ush_minter(&self) -> bool;

        #[view(getUshId)]
        fn get_ush_id(&self) -> TokenIdentifier;
    }
}
