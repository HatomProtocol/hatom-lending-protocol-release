multiversx_sc::imports!();

use super::{constants::*, errors::*, storage};

use controller::{shared::ProxyTrait as _, storage::ProxyTrait as _};
use money_market::common::ProxyTrait as _;
use multiversx_sc::storage::StorageKey;

#[multiversx_sc::module]
pub trait ProxyModule: storage::StorageModule {
    // USH Market calls

    fn is_ush_market(&self, sc_address: &ManagedAddress) -> bool {
        self.ush_money_market_proxy(sc_address.clone()).is_ush_market().execute_on_dest_context()
    }

    fn get_controller(&self) -> Option<ManagedAddress> {
        self.get_ush_money_market_proxy().get_controller().execute_on_dest_context()
    }

    // Money Market calls

    fn get_money_market_identifiers(&self, money_market: &ManagedAddress) -> (EgldOrEsdtTokenIdentifier, TokenIdentifier) {
        self.get_money_market_proxy(money_market).get_money_market_identifiers().execute_on_dest_context()
    }

    fn get_stored_exchange_rate(&self, money_market: &ManagedAddress) -> BigUint {
        self.get_money_market_proxy(money_market).get_stored_exchange_rate().execute_on_dest_context()
    }

    // Controller calls

    fn is_whitelisted_money_market(&self, money_market: &ManagedAddress) -> bool {
        self.get_controller_proxy().is_whitelisted_money_market(money_market).execute_on_dest_context()
    }

    fn get_price_oracle(&self) -> ManagedAddress {
        self.get_controller_proxy().price_oracle().execute_on_dest_context()
    }

    fn get_account_collateral_tokens(&self, controller: &ManagedAddress, money_market: &ManagedAddress, account: &ManagedAddress) -> BigUint {
        let mut storage_key = StorageKey::new(b"account_collateral_tokens");
        storage_key.append_item(money_market);
        storage_key.append_item(account);
        SingleValueMapper::new_from_address(controller.clone(), storage_key).get()
    }

    fn get_ush_borrower_collateral_factor(&self, controller: &ManagedAddress, money_market: &ManagedAddress) -> BigUint {
        let mut storage_key = StorageKey::new(b"ush_borrower_collateral_factor");
        storage_key.append_item(&money_market);
        SingleValueMapper::new_from_address(controller.clone(), storage_key).get()
    }

    // Oracle calls

    fn get_underlying_price(&self, oracle: &ManagedAddress, underlying_id: EgldOrEsdtTokenIdentifier) -> BigUint {
        if underlying_id.is_egld() {
            return BigUint::from(WAD);
        }

        let mut storage_key = StorageKey::new(b"last_price");
        storage_key.append_item(&underlying_id.unwrap_esdt());

        let price = SingleValueMapper::new_from_address(oracle.clone(), storage_key).get();
        require!(price > BigUint::zero(), ERROR_ORACLE_FAILED_RETRIEVE_UNDERLYING_PRICE);
        price
    }

    // Proxies

    #[proxy]
    fn ush_money_market_proxy(&self, sc_address: ManagedAddress) -> ush_money_market_mod::ProxyTo<Self::Api>;

    fn get_ush_money_market_proxy(&self) -> ush_money_market_mod::ProxyTo<Self::Api> {
        let ush_money_market = self.ush_money_market().get();
        self.ush_money_market_proxy(ush_money_market)
    }

    #[proxy]
    fn money_market_proxy(&self, sc_address: ManagedAddress) -> money_market::ProxyTo<Self::Api>;

    fn get_money_market_proxy(&self, sc_address: &ManagedAddress) -> money_market::ProxyTo<Self::Api> {
        self.money_market_proxy(sc_address.clone())
    }

    #[proxy]
    fn controller_proxy(&self, sc_address: ManagedAddress) -> controller::ProxyTo<Self::Api>;

    fn get_controller_proxy(&self) -> controller::ProxyTo<Self::Api> {
        let controller = self.controller().get();
        self.controller_proxy(controller)
    }
}

// Mods

mod ush_money_market_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait MoneyMarket {
        #[view(isUshMarket)]
        fn is_ush_market(&self) -> bool;

        #[view(getController)]
        fn get_controller(&self) -> Option<ManagedAddress>;
    }
}
