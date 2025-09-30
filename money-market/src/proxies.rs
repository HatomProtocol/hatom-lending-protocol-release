multiversx_sc::imports!();

use super::{errors::*, events, storage};
use controller::{governance::ProxyTrait as _, market::ProxyTrait as _, policies::ProxyTrait as _, shared::ProxyTrait as _};

#[multiversx_sc::module]
pub trait ProxyModule: events::EventsModule + storage::StorageModule {
    // Other Money Market calls

    fn accrue_interest_in_other_money_market(&self, sc_address: &ManagedAddress) {
        self.get_other_money_market_proxy(sc_address).accrue_interest().execute_on_dest_context()
    }

    fn seize_in_other_money_market(&self, collateral_market: &ManagedAddress, liquidator: &ManagedAddress, borrower: &ManagedAddress, tokens: &BigUint) -> EsdtTokenPayment {
        self.get_other_money_market_proxy(collateral_market).seize(liquidator, borrower, tokens).execute_on_dest_context()
    }

    // Controller calls

    fn is_controller(&self, sc_address: &ManagedAddress) -> bool {
        self.get_controller_proxy(Some(sc_address.clone())).is_controller().execute_on_dest_context()
    }

    fn get_max_collateral_factor(&self) -> BigUint {
        self.get_controller_proxy(None).get_max_collateral_factor().execute_on_dest_context()
    }

    fn enter_market(&self, account: OptionalValue<ManagedAddress>, payment: &EsdtTokenPayment) {
        self.get_controller_proxy(None).enter_markets(account).with_esdt_transfer(payment.clone()).execute_on_dest_context()
    }

    fn seize_allowed(&self, collateral_market: &ManagedAddress, borrow_market: &ManagedAddress, borrower: &ManagedAddress, liquidator: &ManagedAddress) -> bool {
        self.get_controller_proxy(None).seize_allowed(collateral_market, borrow_market, borrower, liquidator).execute_on_dest_context()
    }

    fn set_account_collateral_tokens(&self, money_market: &ManagedAddress, account: &ManagedAddress, tokens: &BigUint) {
        self.get_controller_proxy(None).set_account_collateral_tokens(money_market, account, tokens).execute_on_dest_context()
    }

    fn get_account_collateral_tokens(&self, money_market_collateral: &ManagedAddress, account: &ManagedAddress) -> BigUint {
        self.get_controller_proxy(None).get_account_collateral_tokens(money_market_collateral, account).execute_on_dest_context()
    }

    fn tokens_to_seize(&self, borrow_market: &ManagedAddress, collateral_market: &ManagedAddress, amount: &BigUint) -> BigUint {
        self.get_controller_proxy(None).tokens_to_seize(borrow_market, collateral_market, amount).execute_on_dest_context()
    }

    fn mint_allowed(&self, money_market: &ManagedAddress, amount: &BigUint) -> bool {
        self.get_controller_proxy(None).mint_allowed(money_market, amount).execute_on_dest_context()
    }

    fn redeem_allowed(&self, money_market: &ManagedAddress, redeemer: &ManagedAddress, tokens: &BigUint) -> bool {
        self.get_controller_proxy(None).redeem_allowed(money_market, redeemer, tokens).execute_on_dest_context()
    }

    fn borrow_allowed(&self, money_market: &ManagedAddress, borrower: &ManagedAddress, amount: &BigUint) -> bool {
        self.get_controller_proxy(None).borrow_allowed(money_market, borrower, amount).execute_on_dest_context()
    }

    fn repay_borrow_allowed(&self, money_market: &ManagedAddress, borrower: &ManagedAddress) -> bool {
        self.get_controller_proxy(None).repay_borrow_allowed(money_market, borrower).execute_on_dest_context()
    }

    fn liquidate_borrow_allowed(&self, borrow_market: &ManagedAddress, collateral_market: &ManagedAddress, borrower: &ManagedAddress, amount: &BigUint) -> bool {
        self.get_controller_proxy(None).liquidate_borrow_allowed(borrow_market, collateral_market, borrower, amount).execute_on_dest_context()
    }

    fn controller_burn_tokens(&self, token_id: &TokenIdentifier, tokens: &BigUint) {
        self.get_controller_proxy(None).burn_tokens(token_id, tokens).execute_on_dest_context()
    }

    fn controller_transfer_tokens(&self, to: &ManagedAddress, payment: &EsdtTokenPayment) {
        self.get_controller_proxy(None).transfer_tokens(to, payment).execute_on_dest_context()
    }

    fn try_remove_account_market(&self, money_market: &ManagedAddress, account: &ManagedAddress) {
        self.get_controller_proxy(None).remove_account_market(money_market, OptionalValue::Some(account.clone())).execute_on_dest_context()
    }

    // Interest Rate Model calls

    fn is_interest_rate_model(&self, sc_address: &ManagedAddress) -> bool {
        self.get_interest_rate_model_proxy(Some(sc_address.clone())).is_interest_rate_model().execute_on_dest_context()
    }

    fn get_borrow_rate(&self, borrows: &BigUint, liquidity: &BigUint) -> BigUint {
        self.get_interest_rate_model_proxy(None).get_borrow_rate(borrows, liquidity).execute_on_dest_context()
    }

    fn get_rates(&self, borrows: &BigUint, liquidity: &BigUint, reserve_factor: &BigUint) -> (BigUint, BigUint) {
        self.get_interest_rate_model_proxy(None).get_rates(borrows, liquidity, reserve_factor).execute_on_dest_context()
    }

    fn get_supply_rate(&self, borrows: &BigUint, liquidity: &BigUint, reserve_factor: &BigUint) -> BigUint {
        self.get_interest_rate_model_proxy(None).get_supply_rate(borrows, liquidity, reserve_factor).execute_on_dest_context()
    }

    fn get_model_parameters(&self) -> (BigUint, BigUint, BigUint, BigUint, BigUint) {
        self.get_interest_rate_model_proxy(None).get_model_parameters().execute_on_dest_context()
    }

    // Staking calls

    fn is_staking(&self, sc_address: &ManagedAddress) -> bool {
        self.get_staking_proxy(Some(sc_address.clone())).is_staking().execute_on_dest_context()
    }

    // Trusted Minters calls

    fn is_trusted_minter(&self, trusted_minter: &ManagedAddress) -> bool {
        self.get_trusted_minter_proxy(trusted_minter).is_trusted_minter().execute_on_dest_context()
    }

    // Proxies
    #[proxy]
    fn controller_proxy(&self, sc_address: ManagedAddress) -> controller::ProxyTo<Self::Api>;

    fn get_controller_proxy(&self, sc_address: Option<ManagedAddress>) -> controller::ProxyTo<Self::Api> {
        match sc_address {
            Some(controller_address) => self.controller_proxy(controller_address),
            None => {
                require!(!self.controller().is_empty(), ERROR_UNDEFINED_CONTROLLER);
                let controller_address = self.controller().get();
                self.controller_proxy(controller_address)
            },
        }
    }

    #[proxy]
    fn interest_rate_model_proxy(&self, sc_address: ManagedAddress) -> interest_rate_model::ProxyTo<Self::Api>;

    fn get_interest_rate_model_proxy(&self, sc_address: Option<ManagedAddress>) -> interest_rate_model::ProxyTo<Self::Api> {
        match sc_address {
            Some(interest_rate_model_address) => self.interest_rate_model_proxy(interest_rate_model_address),
            None => {
                require!(!self.interest_rate_model().is_empty(), ERROR_UNDEFINED_INTEREST_RATE_MODEL);
                let interest_rate_model_address = self.interest_rate_model().get();
                self.interest_rate_model_proxy(interest_rate_model_address)
            },
        }
    }

    #[proxy]
    fn other_money_market_proxy(&self, sc_address: ManagedAddress) -> money_market_mod::ProxyTo<Self::Api>;

    fn get_other_money_market_proxy(&self, sc_address: &ManagedAddress) -> money_market_mod::ProxyTo<Self::Api> {
        self.other_money_market_proxy(sc_address.clone())
    }

    #[proxy]
    fn staking_proxy(&self, sc_address: ManagedAddress) -> staking_mod::ProxyTo<Self::Api>;

    fn get_staking_proxy(&self, sc_address: Option<ManagedAddress>) -> staking_mod::ProxyTo<Self::Api> {
        match sc_address {
            Some(address) => self.staking_proxy(address),
            None => {
                require!(!self.staking_contract().is_empty(), ERROR_UNDEFINED_STAKING_SC);
                let address = self.staking_contract().get();
                self.staking_proxy(address)
            },
        }
    }

    #[proxy]
    fn trusted_minter_proxy(&self, sc_address: ManagedAddress) -> trusted_minter_mod::ProxyTo<Self::Api>;

    fn get_trusted_minter_proxy(&self, sc_address: &ManagedAddress) -> trusted_minter_mod::ProxyTo<Self::Api> {
        self.trusted_minter_proxy(sc_address.clone())
    }
}

/// Can't simply import, we would have a circular dependency.
mod money_market_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait MoneyMarket {
        #[endpoint(accrueInterest)]
        fn accrue_interest(&self);

        #[endpoint(seize)]
        fn seize(&self, liquidator: &ManagedAddress, borrower: &ManagedAddress, tokens_to_seize: &BigUint) -> EsdtTokenPayment;
    }
}

mod staking_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait Staking {
        #[view(isStaking)]
        fn is_staking(&self) -> bool;
    }
}

mod trusted_minter_mod {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait TrustedMinter {
        #[view(isTrustedMinter)]
        fn is_trusted_minter(&self) -> bool;
    }
}
