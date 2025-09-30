multiversx_sc::imports!();

use super::{constants::*, errors::*, storage};

use oracle::{common::ProxyTrait as _, prices::ProxyTrait as _};

use crate::storage::SwapOperationType;

#[multiversx_sc::module]
pub trait ProxyModule: storage::StorageModule {
    // Money Market calls

    fn is_money_market(&self, sc_address: &ManagedAddress) -> bool {
        self.get_money_market_proxy(sc_address).is_money_market().execute_on_dest_context()
    }

    fn get_money_market_identifiers(&self, sc_address: &ManagedAddress) -> (EgldOrEsdtTokenIdentifier, TokenIdentifier) {
        self.get_money_market_proxy(sc_address).get_money_market_identifiers().execute_on_dest_context()
    }

    fn get_liquidity(&self, sc_address: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(sc_address).get_liquidity().execute_on_dest_context()
    }

    fn get_total_borrows(&self, sc_address: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(sc_address).get_total_borrows().execute_on_dest_context()
    }

    fn get_base_total_borrows(&self, sc_address: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(sc_address).get_base_total_borrows().execute_on_dest_context()
    }

    fn get_stored_account_borrow_amount(&self, sc_address: &ManagedAddress, borrower: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(sc_address).stored_account_borrow_amount(borrower).execute_on_dest_context()
    }

    fn get_base_account_borrow_amount(&self, sc_address: &ManagedAddress, borrower: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(sc_address).base_account_borrow_amount(borrower).execute_on_dest_context()
    }

    fn get_close_factor(&self, sc_address: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(sc_address).get_close_factor().execute_on_dest_context()
    }

    fn get_stored_exchange_rate(&self, sc_address: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(sc_address).get_stored_exchange_rate().execute_on_dest_context()
    }

    fn get_liquidation_incentive(&self, sc_address: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(sc_address).get_liquidation_incentive().execute_on_dest_context()
    }

    fn get_reserve_factor(&self, sc_address: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(sc_address).get_reserve_factor().execute_on_dest_context()
    }

    fn get_controller(&self, sc_address: &ManagedAddress) -> Option<ManagedAddress> {
        self.get_money_market_proxy(sc_address).get_controller().execute_on_dest_context()
    }

    fn get_account_snapshot(&self, sc_address: &ManagedAddress, account: &ManagedAddress) -> (BigUint, BigUint) {
        self.get_money_market_proxy(sc_address).get_reliable_account_snapshot(account).execute_on_dest_context()
    }

    fn redeem(&self, sc_address: &ManagedAddress, token_payment: &EsdtTokenPayment, opt_underlying_amount: Option<BigUint>) -> money_market_mod::RedeemResultType<Self::Api> {
        self.get_money_market_proxy(sc_address).redeem(OptionalValue::from(opt_underlying_amount)).with_esdt_transfer(token_payment.clone()).execute_on_dest_context()
    }

    // Oracle calls

    fn is_price_oracle(&self, sc_address: &ManagedAddress) -> bool {
        self.price_oracle_proxy(sc_address.clone()).is_price_oracle().execute_on_dest_context()
    }

    fn get_price_oracle(&self) -> Option<ManagedAddress> {
        if self.price_oracle().is_empty() {
            None
        } else {
            let address = self.price_oracle().get();
            Some(address)
        }
    }

    fn get_underlying_price(&self, money_market: &ManagedAddress) -> BigUint {
        let (underlying_id, _) = self.identifiers(money_market).get();

        if underlying_id.is_egld() {
            return BigUint::from(WAD);
        }

        let mut proxy = self.get_price_oracle_proxy();
        let price = proxy.get_price_in_egld(&underlying_id.unwrap_esdt()).execute_on_dest_context();
        require!(price > BigUint::zero(), ERROR_ORACLE_FAILED_RETRIEVE_UNDERLYING_PRICE);
        price
    }

    // xExchange calls

    fn get_xexchange_router(&self) -> Option<ManagedAddress> {
        if self.router().is_empty() {
            None
        } else {
            let address = self.router().get();
            Some(address)
        }
    }

    fn multi_pair_swap(&self, swap_operations: MultiValueEncoded<SwapOperationType<Self::Api>>, token_in: &TokenIdentifier, token_amount: &BigUint) {
        let mut proxy = self.get_xexchange_router_proxy();
        proxy.multi_pair_swap(swap_operations).with_esdt_transfer((token_in.clone(), 0, token_amount.clone())).execute_on_dest_context()
    }

    // Wrapped EGLD

    fn get_wegld_id(&self, egld_wrapper: &ManagedAddress) -> TokenIdentifier {
        self.egld_wrapper_proxy(egld_wrapper.clone()).get_wrapped_egld_token_id().execute_on_dest_context()
    }

    fn wrap_egld(&self, amount: &BigUint) {
        let egld_wrapper = self.egld_wrapper().get();
        self.egld_wrapper_proxy(egld_wrapper).wrap_egld().with_egld_transfer(amount.clone()).execute_on_dest_context()
    }

    fn unwrap_egld(&self, amount: &BigUint) {
        let egld_wrapper = self.egld_wrapper().get();
        let wegld_id = self.wegld_id().get();
        self.egld_wrapper_proxy(egld_wrapper).unwrap_egld().with_esdt_transfer((wegld_id, 0, amount.clone())).execute_on_dest_context()
    }

    // Market observer calls

    fn is_finalized(&self, market_observer: &ManagedAddress) -> bool {
        self.market_observer_proxy(market_observer.clone()).is_finalized().execute_on_dest_context()
    }

    // Rewards booster calls

    fn is_rewards_booster(&self, sc_address: &ManagedAddress) -> bool {
        self.rewards_booster_proxy(sc_address.clone()).is_rewards_booster().execute_on_dest_context()
    }

    fn get_rewards_booster_version(&self, rewards_booster: &ManagedAddress) -> u8 {
        self.rewards_booster_proxy(rewards_booster.clone()).get_version().execute_on_dest_context()
    }

    fn on_market_change_booster_v1(&self, sc_address: &ManagedAddress, money_market: &ManagedAddress, account: &ManagedAddress, tokens: &BigUint) {
        self.rewards_booster_v1_proxy(sc_address.clone()).on_market_change(money_market, account, tokens).execute_on_dest_context()
    }

    fn on_market_change_booster_v2(&self, sc_address: &ManagedAddress, money_market: &ManagedAddress, account: &ManagedAddress, tokens: &BigUint, prev_tokens: &BigUint) {
        self.rewards_booster_v2_proxy(sc_address.clone()).on_market_change(money_market, account, tokens, prev_tokens).execute_on_dest_context()
    }

    // USH market calls

    fn is_ush_market(&self, sc_address: &ManagedAddress) -> bool {
        self.get_ush_market_proxy(sc_address).is_ush_market().execute_on_dest_context()
    }

    fn on_market_change_ush_market(&self, sc_address: &ManagedAddress, account: &ManagedAddress) {
        self.get_ush_market_proxy(sc_address).on_market_change(account).execute_on_dest_context()
    }

    // Proxies

    #[proxy]
    fn money_market_proxy(&self, sc_address: ManagedAddress) -> money_market_mod::ProxyTo<Self::Api>;

    fn get_money_market_proxy(&self, sc_address: &ManagedAddress) -> money_market_mod::ProxyTo<Self::Api> {
        self.money_market_proxy(sc_address.clone())
    }

    #[proxy]
    fn ush_market_proxy(&self, sc_address: ManagedAddress) -> ush_market_mod::ProxyTo<Self::Api>;

    fn get_ush_market_proxy(&self, sc_address: &ManagedAddress) -> ush_market_mod::ProxyTo<Self::Api> {
        self.ush_market_proxy(sc_address.clone())
    }

    #[proxy]
    fn price_oracle_proxy(&self, sc_address: ManagedAddress) -> oracle::ProxyTo<Self::Api>;

    fn get_price_oracle_proxy(&self) -> oracle::ProxyTo<Self::Api> {
        let oracle_address = self.get_price_oracle();

        match oracle_address {
            None => sc_panic!(ERROR_ORACLE_NOT_INITIALIZED),
            Some(address) => self.price_oracle_proxy(address),
        }
    }

    #[proxy]
    fn xexchange_proxy(&self, sc_address: ManagedAddress) -> xexchange_mod::ProxyTo<Self::Api>;

    fn get_xexchange_router_proxy(&self) -> xexchange_mod::ProxyTo<Self::Api> {
        let router = self.get_xexchange_router();

        match router {
            None => sc_panic!(ERROR_ROUTER_NOT_INITIALIZED),
            Some(address) => self.xexchange_proxy(address),
        }
    }

    #[proxy]
    fn egld_wrapper_proxy(&self, sc_address: ManagedAddress) -> egld_wrapper_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn market_observer_proxy(&self, sc_address: ManagedAddress) -> market_observer_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn rewards_booster_proxy(&self, sc_address: ManagedAddress) -> rewards_booster_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn rewards_booster_v1_proxy(&self, sc_address: ManagedAddress) -> rewards_booster_v1_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn rewards_booster_v2_proxy(&self, sc_address: ManagedAddress) -> rewards_booster_v2_mod::ProxyTo<Self::Api>;
}

mod money_market_mod {
    multiversx_sc::imports!();

    pub type RedeemResultType<BigUint> = MultiValue2<EgldOrEsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

    #[multiversx_sc::proxy]
    pub trait MoneyMarket {
        #[view(isMoneyMarket)]
        fn is_money_market(&self) -> bool;

        #[view(getMoneyMarketIdentifiers)]
        fn get_money_market_identifiers(&self) -> (EgldOrEsdtTokenIdentifier, TokenIdentifier);

        #[view(getLiquidity)]
        fn get_liquidity(&self) -> BigUint;

        #[view(getTotalBorrows)]
        fn get_total_borrows(&self) -> BigUint;

        #[view(getBaseTotalBorrows)]
        fn get_base_total_borrows(&self) -> BigUint;

        #[view(getStoredAccountBorrowAmount)]
        fn stored_account_borrow_amount(&self, account: &ManagedAddress) -> BigUint;

        #[view(getBaseAccountBorrowAmount)]
        fn base_account_borrow_amount(&self, account: &ManagedAddress) -> BigUint;

        #[view(getStoredExchangeRate)]
        fn get_stored_exchange_rate(&self) -> BigUint;

        #[view(getCloseFactor)]
        fn get_close_factor(&self) -> BigUint;

        #[view(getLiquidationIncentive)]
        fn get_liquidation_incentive(&self) -> BigUint;

        #[view(getReserveFactor)]
        fn get_reserve_factor(&self) -> BigUint;

        #[view(getController)]
        fn get_controller(&self) -> Option<ManagedAddress>;

        #[endpoint(getReliableAccountSnapshot)]
        fn get_reliable_account_snapshot(&self, account: &ManagedAddress) -> (BigUint, BigUint);

        #[payable("*")]
        #[endpoint(redeem)]
        fn redeem(&self, opt_underlying_amount: OptionalValue<BigUint>) -> RedeemResultType<Self::Api>;
    }
}

mod ush_market_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait UshMoneyMarket {
        #[view(isUshMarket)]
        fn is_ush_market(&self) -> bool;

        #[endpoint(onMarketChange)]
        fn on_market_change(&self, account: &ManagedAddress);
    }
}

pub mod xexchange_mod {
    multiversx_sc::imports!();

    type SwapOperationType<M> = MultiValue4<ManagedAddress<M>, ManagedBuffer<M>, TokenIdentifier<M>, BigUint<M>>;

    #[multiversx_sc::proxy]
    pub trait Router {
        #[payable("*")]
        #[endpoint(multiPairSwap)]
        fn multi_pair_swap(&self, swap_operations: MultiValueEncoded<SwapOperationType<Self::Api>>);
    }
}

mod egld_wrapper_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait EgldWrapper {
        #[payable("EGLD")]
        #[endpoint(wrapEgld)]
        fn wrap_egld(&self);

        #[payable("*")]
        #[endpoint(unwrapEgld)]
        fn unwrap_egld(&self);

        #[view(getWrappedEgldTokenId)]
        fn get_wrapped_egld_token_id(&self) -> TokenIdentifier;
    }
}

mod market_observer_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait MarketObserver {
        #[endpoint(onMarketChange)]
        fn on_market_change(&self, money_market: &ManagedAddress, account: &ManagedAddress, tokens: &BigUint);

        #[view(isFinalized)]
        fn is_finalized(&self) -> bool;
    }
}

mod rewards_booster_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait RewardsBooster {
        #[view(isRewardsBooster)]
        fn is_rewards_booster(&self) -> bool;

        #[view(getVersion)]
        fn get_version(&self) -> u8;
    }
}

mod rewards_booster_v1_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait RewardsBooster {
        #[endpoint(onMarketChange)]
        fn on_market_change(&self, money_market: &ManagedAddress, account: &ManagedAddress, tokens: &BigUint);
    }
}

mod rewards_booster_v2_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait RewardsBooster {
        #[endpoint(onMarketChange)]
        fn on_market_change(&self, money_market: &ManagedAddress, account: &ManagedAddress, tokens: &BigUint, prev_tokens: &BigUint);
    }
}
