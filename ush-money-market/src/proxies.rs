multiversx_sc::imports!();

use super::{events, storage};

use controller::{governance::ProxyTrait as _, market::ProxyTrait as _, policies::ProxyTrait as _, shared::ProxyTrait as _};
use discount_rate_model::{commons::ProxyTrait as _, discount::ProxyTrait as _, models::ExchangeRateType, storage::ProxyTrait as _};
use money_market::{common::ProxyTrait as _, seize::ProxyTrait as _};
use ush_minter::{esdt::ProxyTrait as _, permissions::ProxyTrait as _};

use crate::errors::*;

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
        self.controller_proxy(sc_address.clone()).is_controller().execute_on_dest_context()
    }

    fn is_ush_market_observer(&self, sc_address: &ManagedAddress) -> bool {
        self.get_controller_proxy().is_ush_market_observer(sc_address).execute_on_dest_context()
    }

    fn is_deprecated_market(&self, sc_address: &ManagedAddress) -> bool {
        self.get_controller_proxy().is_deprecated(sc_address).execute_on_dest_context()
    }

    fn get_max_collateral_factor(&self) -> BigUint {
        self.get_controller_proxy().get_max_collateral_factor().execute_on_dest_context()
    }

    fn enter_market(&self, account: OptionalValue<ManagedAddress>, payment: &EsdtTokenPayment) {
        self.get_controller_proxy().enter_markets(account).with_esdt_transfer(payment.clone()).execute_on_dest_context()
    }

    fn seize_allowed(&self, collateral_market: &ManagedAddress, borrow_market: &ManagedAddress, borrower: &ManagedAddress, liquidator: &ManagedAddress) -> bool {
        self.get_controller_proxy().seize_allowed(collateral_market, borrow_market, borrower, liquidator).execute_on_dest_context()
    }

    fn set_account_collateral_tokens(&self, money_market: &ManagedAddress, account: &ManagedAddress, tokens: &BigUint) {
        self.get_controller_proxy().set_account_collateral_tokens(money_market, account, tokens).execute_on_dest_context()
    }

    fn get_account_collateral_tokens(&self, money_market_collateral: &ManagedAddress, account: &ManagedAddress) -> BigUint {
        self.get_controller_proxy().get_account_collateral_tokens(money_market_collateral, account).execute_on_dest_context()
    }

    fn tokens_to_seize(&self, borrow_market: &ManagedAddress, collateral_market: &ManagedAddress, amount: &BigUint) -> BigUint {
        self.get_controller_proxy().tokens_to_seize(borrow_market, collateral_market, amount).execute_on_dest_context()
    }

    fn mint_allowed(&self, money_market: &ManagedAddress, amount: &BigUint) -> bool {
        self.get_controller_proxy().mint_allowed(money_market, amount).execute_on_dest_context()
    }

    fn borrow_allowed(&self, money_market: &ManagedAddress, borrower: &ManagedAddress, amount: &BigUint) -> bool {
        self.get_controller_proxy().borrow_allowed(money_market, borrower, amount).execute_on_dest_context()
    }

    fn repay_borrow_allowed(&self, money_market: &ManagedAddress, borrower: &ManagedAddress) -> bool {
        self.get_controller_proxy().repay_borrow_allowed(money_market, borrower).execute_on_dest_context()
    }

    fn liquidate_borrow_allowed(&self, borrow_market: &ManagedAddress, collateral_market: &ManagedAddress, borrower: &ManagedAddress, amount: &BigUint) -> bool {
        self.get_controller_proxy().liquidate_borrow_allowed(borrow_market, collateral_market, borrower, amount).execute_on_dest_context()
    }

    fn controller_burn_tokens(&self, token_id: &TokenIdentifier, tokens: &BigUint) {
        self.get_controller_proxy().burn_tokens(token_id, tokens).execute_on_dest_context()
    }

    fn controller_transfer_tokens(&self, to: &ManagedAddress, payment: &EsdtTokenPayment) {
        self.get_controller_proxy().transfer_tokens(to, payment).execute_on_dest_context()
    }

    fn try_remove_account_market(&self, money_market: &ManagedAddress, account: &ManagedAddress) {
        self.get_controller_proxy().remove_account_market(money_market, OptionalValue::Some(account.clone())).execute_on_dest_context()
    }

    // Discount Rate Model calls

    fn is_discount_rate_model(&self, sc_address: &ManagedAddress) -> bool {
        self.discount_rate_model_proxy(sc_address.clone()).is_discount_rate_model().execute_on_dest_context()
    }

    fn get_ush_money_market(&self, sc_address: &ManagedAddress) -> ManagedAddress {
        self.discount_rate_model_proxy(sc_address.clone()).ush_money_market().execute_on_dest_context()
    }

    fn get_account_discount(&self, borrower: &ManagedAddress, borrow: &BigUint, fx_type: ExchangeRateType) -> BigUint {
        self.get_discount_rate_model_proxy().get_account_discount(borrower, borrow, fx_type).execute_on_dest_context()
    }

    // Staking calls

    fn is_staking(&self, sc_address: &ManagedAddress) -> bool {
        self.staking_proxy(sc_address.clone()).is_staking().execute_on_dest_context()
    }

    // Trusted Minters calls

    fn is_trusted_minter(&self, sc_address: &ManagedAddress) -> bool {
        self.trusted_minter_proxy(sc_address.clone()).is_trusted_minter().execute_on_dest_context()
    }

    // USH Minter calls

    fn is_ush_minter(&self, sc_address: &ManagedAddress) -> bool {
        self.ush_minter_proxy(sc_address.clone()).is_ush_minter().execute_on_dest_context()
    }

    fn get_ush_id(&self, sc_address: &ManagedAddress) -> TokenIdentifier {
        self.ush_minter_proxy(sc_address.clone()).get_ush_id().execute_on_dest_context()
    }

    fn is_facilitator(&self, address: &ManagedAddress) -> bool {
        self.get_ush_minter_proxy().is_facilitator(address).execute_on_dest_context()
    }

    fn ush_minter_mint(&self, amount: &BigUint, destination: OptionalValue<ManagedAddress>) -> EsdtTokenPayment {
        if amount == &BigUint::zero() {
            let ush_id = self.ush_id().get();
            return EsdtTokenPayment::new(ush_id, 0, BigUint::zero());
        }
        self.get_ush_minter_proxy().mint(amount.clone(), destination).execute_on_dest_context()
    }

    fn ush_minter_burn(&self, ush_payment: &EsdtTokenPayment) {
        if ush_payment.amount == BigUint::zero() {
            return;
        }
        self.get_ush_minter_proxy().burn().with_esdt_transfer(ush_payment.clone()).execute_on_dest_context()
    }

    // Proxies
    #[proxy]
    fn controller_proxy(&self, sc_address: ManagedAddress) -> controller::ProxyTo<Self::Api>;

    fn get_controller_proxy(&self) -> controller::ProxyTo<Self::Api> {
        let controller = self.controller().get();
        self.controller_proxy(controller)
    }

    #[proxy]
    fn discount_rate_model_proxy(&self, sc_address: ManagedAddress) -> discount_rate_model::ProxyTo<Self::Api>;

    fn get_discount_rate_model_proxy(&self) -> discount_rate_model::ProxyTo<Self::Api> {
        let discount_rate_model = self.discount_rate_model().get();
        self.discount_rate_model_proxy(discount_rate_model)
    }

    #[proxy]
    fn other_money_market_proxy(&self, sc_address: ManagedAddress) -> money_market::ProxyTo<Self::Api>;

    fn get_other_money_market_proxy(&self, sc_address: &ManagedAddress) -> money_market::ProxyTo<Self::Api> {
        self.other_money_market_proxy(sc_address.clone())
    }

    #[proxy]
    fn staking_proxy(&self, sc_address: ManagedAddress) -> staking_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn trusted_minter_proxy(&self, sc_address: ManagedAddress) -> trusted_minter_mod::ProxyTo<Self::Api>;

    #[proxy]
    fn ush_minter_proxy(&self, sc_address: ManagedAddress) -> ush_minter::ProxyTo<Self::Api>;

    fn get_ush_minter_proxy(&self) -> ush_minter::ProxyTo<Self::Api> {
        require!(!self.ush_minter().is_empty(), ERROR_UNDEFINED_USH_MINTER_SC);
        let minter = self.ush_minter().get();
        self.ush_minter_proxy(minter)
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
