multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{constants::*, errors::*, events, model::*, proxies, storage};

#[multiversx_sc::module]
pub trait CommonModule: admin::AdminModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    // Checks

    /// A utility function to highlight that this smart contract is a Price Oracle.
    ///
    #[view(isPriceOracle)]
    fn is_price_oracle(&self) -> bool {
        true
    }

    /// Checks whether a given token identifier is Wrapped EGLD.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    #[inline]
    fn is_wrapped_egld(&self, token_id: &TokenIdentifier) -> bool {
        *token_id == self.wegld_id().get()
    }

    /// Checks whether a given token identifier matches the Liquid Staked EGLD token identifier.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    #[inline]
    fn is_ls_token(&self, token_id: &TokenIdentifier) -> bool {
        *token_id == self.ls_token_id().get()
    }

    /// Checks whether a given token identifier matches the Liquid Staked TAO token identifier.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    #[inline]
    fn is_stao_token(&self, token_id: &TokenIdentifier) -> bool {
        *token_id == self.stao_token_id().get()
    }

    /// Checks whether a given token identifier matches the USH token identifier.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    #[inline]
    fn is_ush_token(&self, token_id: &TokenIdentifier) -> bool {
        *token_id == self.ush_token_id().get()
    }

    /// Checks whether the given smart contract is a Liquid Staking Smart Contract.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract.
    ///
    #[inline]
    fn is_liquid_staking_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_liquid_staking(sc_address)
    }

    /// Checks whether the given smart contract is a TAO Liquid Staking Smart Contract.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract.
    ///
    #[inline]
    fn is_tao_liquid_staking_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_tao_liquid_staking(sc_address)
    }

    /// Checks whether the specified smart contract address is the USH minter module.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    #[inline]
    fn is_ush_minter_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_ush_minter(sc_address)
    }

    /// Checks whether the given token is supported or not.
    ///
    #[inline]
    fn is_supported_token(&self, token_id: &TokenIdentifier) -> bool {
        !self.supported_tokens(token_id).is_empty()
    }

    /// Checks whether the xExchange Pair smart contract is paused or not.
    ///
    #[inline]
    fn is_xexchange_paused(&self, pair_address: &ManagedAddress) -> bool {
        let pair_state = self.get_xexchange_state(pair_address);
        pair_state == PairState::Inactive || pair_state == PairState::PartialActive
    }

    /// Requires that the caller is the admin or the guardian, if it is set.
    ///
    fn require_admin_or_guardian(&self) {
        let admin = self.get_admin();
        let caller = self.blockchain().get_caller();

        match self.get_guardian() {
            None => {
                require!(caller == admin, ERROR_ONLY_ADMIN);
            },
            Some(guardian) => {
                require!(caller == admin || caller == guardian, ERROR_ONLY_ADMIN_OR_GUARDIAN);
            },
        }
    }

    /// Requires that the provided token identifier is a valid ESDT token identifier.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier to check.
    ///
    #[inline]
    fn require_valid_token_identifier(&self, token_id: &TokenIdentifier) {
        require!(token_id.is_valid_esdt_identifier(), ERROR_INVALID_TOKEN_ID);
    }

    /// Requires that the EGLD Wrapper smart contract is not paused.
    ///
    #[inline]
    fn require_egld_wrapper_not_paused(&self) {
        require!(!self.is_egld_wrapper_paused(), ERROR_EGLD_WRAPPER_PAUSED);
    }

    /// Requires that the xExchange pair smart contract is not paused.
    ///
    fn require_xexchange_not_paused(&self, pair_address: &ManagedAddress) {
        require!(!self.is_xexchange_paused(pair_address), ERROR_XEXCHANGE_PAUSED);
    }

    /// Requires that either the EGLD Wrapper or the xExchange pair is paused.
    ///
    fn require_egld_wrapper_or_xexchange_paused(&self, pair_address: &ManagedAddress) {
        require!(self.is_egld_wrapper_paused() || self.is_xexchange_paused(pair_address), ERROR_EGLD_WRAPPER_NOR_XEXCHANGE_PAUSED);
    }

    /// Requires that the provided token identifier is a valid ESDT token identifier and it is not WEGLD, SEGLD nor SwTAO.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier to check.
    ///
    fn require_valid_token_identifier_subset(&self, token_id: &TokenIdentifier) {
        self.require_valid_token_identifier(token_id);
        require!(!self.is_wrapped_egld(token_id) && !self.is_ls_token(token_id) && !self.is_stao_token(token_id), ERROR_INVALID_TOKEN_ID);
    }

    /// Requires that a xExchange pair exists for the given token identifier.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier for which to check for a xExchange pair.
    ///
    #[inline]
    fn require_supported_token(&self, token_id: &TokenIdentifier) {
        require!(self.is_supported_token(token_id), ERROR_UNSUPPORTED_TOKEN);
    }

    /// Requires that a token is paused.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    #[inline]
    fn require_token_paused(&self, token_id: &TokenIdentifier) {
        require!(self.is_token_paused(token_id).get(), ERROR_TOKEN_NOT_PAUSED);
    }

    /// Requires that a token is not paused.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    #[inline]
    fn require_token_not_paused(&self, token_id: &TokenIdentifier) {
        require!(!self.is_token_paused(token_id).get(), ERROR_TOKEN_PAUSED);
    }

    // Gets

    /// Returns the token data assuming that it has been already supported.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    #[inline]
    fn get_supported_token_data(&self, token_id: &TokenIdentifier) -> TokenData<Self::Api> {
        self.supported_tokens(token_id).get()
    }

    /// Returns the first and second token identifiers from a given xExchange pair.
    ///
    /// # Arguments:
    ///
    /// - `pair_address` - The address of the xExchange pair.
    ///
    fn get_xexchange_pair_tokens(&self, pair_address: &ManagedAddress) -> (TokenIdentifier, TokenIdentifier) {
        require!(self.blockchain().is_smart_contract(pair_address), ERROR_EXPECTED_SC);
        let first_token_id = self.get_xexchange_first_token_id(pair_address);
        let second_token_id = self.get_xexchange_second_token_id(pair_address);
        (first_token_id, second_token_id)
    }

    /// Returns the pricing method for a given ESDT token.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The ESDT token identifier.
    ///
    #[inline]
    fn get_pricing_method(&self, token_id: &TokenIdentifier) -> PricingMethod {
        self.pricing_method(token_id).get()
    }

    /// Computes and returns the upper and lower bounds for a given anchor tolerance.
    ///
    /// # Arguments:
    ///
    /// - `anchor_tolerance` - The new anchor tolerance ratio (in WAD).
    ///
    fn get_bounds(&self, anchor_tolerance: &BigUint) -> (BigUint, BigUint) {
        let wad = BigUint::from(WAD);
        let upper_bound = &wad + anchor_tolerance;
        let lower_bound = &wad * &wad / &upper_bound;
        (upper_bound, lower_bound)
    }

    /// Gets the last valid and used price for a given token.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    #[inline]
    fn get_last_price(&self, token_id: &TokenIdentifier) -> BigUint {
        self.last_price(token_id).get()
    }

    /// Gets the address of the pause guardian.
    ///
    fn get_guardian(&self) -> Option<ManagedAddress> {
        if self.guardian().is_empty() {
            None
        } else {
            let pause_guardian = self.guardian().get();
            Some(pause_guardian)
        }
    }

    /// Returns the whitelisted fallback tokens.
    ///
    #[inline]
    fn get_whitelisted_fallback_tokens(&self) -> ManagedVec<TokenIdentifier> {
        #[rustfmt::skip]
         let tokens = ManagedVec::from_iter([
            TokenIdentifier::from(USDC_TOKEN_ID_M),
            TokenIdentifier::from(USDT_TOKEN_ID_M),
            TokenIdentifier::from(USDC_TOKEN_ID_D),
            TokenIdentifier::from(USDT_TOKEN_ID_D),
        ]);
        tokens
    }

    // Sets

    /// Sets the EGLD Wrapper address iff not already set.
    ///
    /// # Arguments:
    ///
    /// - `egld_wrapper` - The EGLD Wrapper address.
    ///
    fn try_set_egld_wrapper(&self, egld_wrapper: &ManagedAddress) {
        if self.egld_wrapper().is_empty() {
            let wegld_id = self.get_wegld_id(egld_wrapper);
            self.egld_wrapper().set(egld_wrapper);
            self.wegld_id().set(&wegld_id);
            self.set_egld_wrapper_event(egld_wrapper, &wegld_id);
        }
    }

    /// Sets the xExchange pricing method iff not already set.
    ///
    /// # Arguments:
    ///
    /// - `xexchange_pricing_method` - The xExchange pricing method.
    ///
    fn try_set_xexchange_pricing_method(&self, xexchange_pricing_method: ExchangePricingMethod) {
        if self.xexchange_pricing_method().is_empty() {
            self.set_xexchange_pricing_method_internal(xexchange_pricing_method);
        }
    }

    /// Sets the xExchange pricing method.
    ///
    /// # Arguments:
    ///
    /// - `xexchange_pricing_method` - The xExchange pricing method.
    ///
    fn set_xexchange_pricing_method_internal(&self, xexchange_pricing_method: ExchangePricingMethod) {
        require!(xexchange_pricing_method != ExchangePricingMethod::None, ERROR_UNEXPECTED_XEXCHANGE_PRICING_METHOD);
        self.xexchange_pricing_method().set(xexchange_pricing_method);
        self.set_xexchange_pricing_method_event(xexchange_pricing_method);
    }

    /// Sets the last known price for a given token.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The identifier of the token.
    /// - `price` - The last known price for the token (in EGLD).
    ///
    fn set_last_price(&self, token_id: &TokenIdentifier, price: &BigUint) {
        self.last_price(token_id).set(price);
        self.last_price_event(token_id, price);
    }
}
