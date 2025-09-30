multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{common, constants::*, errors::*, events, model::*, prices, proxies, storage};

#[multiversx_sc::module]
pub trait GovernanceModule: admin::AdminModule + events::EventsModule + storage::StorageModule + common::CommonModule + prices::PriceModule + proxies::ProxyModule {
    /// Sets the Guardian of the Oracle.
    ///
    /// # Arguments:
    ///
    /// - `guardian` - The address of the new Guardian.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setGuardian)]
    fn set_guardian(&self, guardian: ManagedAddress) {
        self.require_admin();
        let old_guardian = self.get_guardian();
        self.guardian().set(&guardian);
        self.new_guardian_event(&old_guardian, &guardian);
    }

    /// Unpauses the token pricing.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    /// - `opt_first_anchor_tolerance` - Optional first anchor tolerance.
    /// - `opt_last_anchor_tolerance` - Optional last anchor tolerance.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or guardian.
    /// - The pricing might have been paused by the `Default` pricing algorithm.
    /// - The guardian can change the anchor tolerances if the token has been paused.
    ///
    #[allow_multiple_var_args]
    #[endpoint(unpauseToken)]
    fn unpause_token(&self, token_id: TokenIdentifier, opt_first_anchor_tolerance: OptionalValue<BigUint>, opt_last_anchor_tolerance: OptionalValue<BigUint>) {
        self.require_admin_or_guardian();
        self.require_supported_token(&token_id);
        self.require_token_paused(&token_id);

        match (opt_first_anchor_tolerance, opt_last_anchor_tolerance) {
            (OptionalValue::Some(first), OptionalValue::Some(last)) => {
                self.set_anchor_tolerances_internal(&token_id, &first, &last);
            },
            (OptionalValue::None, OptionalValue::None) => {
                // do nothing when both are None
            },
            _ => {
                // revert if only one is None
                sc_panic!(ERROR_UNEXPECTED_ANCHOR_TOLERANCES);
            },
        }

        self.check_default_pricing_method(&token_id);
        self.is_token_paused(&token_id).set(false);

        self.unpause_token_event(&token_id);
    }

    /// Pauses the token pricing.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or guardian.
    ///
    #[endpoint(pauseToken)]
    fn pause_token(&self, token_id: TokenIdentifier) {
        self.require_admin();
        self.require_supported_token(&token_id);
        self.require_token_not_paused(&token_id);
        self.is_token_paused(&token_id).set(true);
        self.pause_token_event(&token_id);
    }

    /// Allows pricing of tokens using the Price Aggregator Smart Contract as the price provider.
    ///
    /// # Arguments:
    ///
    /// - `price_aggregator_address` - The Price Aggregator address.
    /// - `round_duration_tolerance` - The round duration tolerance as a percentage of the fetched round duration as a
    ///   percentage in BPS.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - Since all token prices are expressed in USD, the Price Aggregator must provide a price for the EGLD/USD.
    /// - Checks all supported tokens can be priced using the Price Aggregator.
    ///
    #[endpoint(supportPriceAggregator)]
    fn support_price_aggregator(&self, price_aggregator_address: ManagedAddress, round_duration_tolerance: u64) {
        self.require_admin();

        require!(self.blockchain().is_smart_contract(&price_aggregator_address), ERROR_EXPECTED_SC);
        self.price_aggregator_address().set(&price_aggregator_address);

        let round_duration = self.get_round_duration();
        self.set_round_duration_internal(round_duration, round_duration_tolerance);

        let usd = ManagedBuffer::from(USD_SYMBOL);
        let egld = ManagedBuffer::from(EGLD_SYMBOL);
        self.get_price_aggregator_latest_price(&egld, &usd);

        for token_id in self.whitelisted_tokens().iter() {
            self.get_price_aggregator_latest_price(&token_id.ticker(), &usd);
        }

        self.support_price_aggregator_event(&price_aggregator_address);
    }

    /// Updates the current round duration by making a call to the Price Aggregator and using a given tolerance.
    ///
    /// # Arguments:
    ///
    /// - `round_duration_tolerance` - The round duration tolerance as a percentage of the fetched round duration and in BPS.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The Price Aggregator must have been already supported.
    ///
    #[endpoint(updateRoundDuration)]
    fn update_round_duration(&self, round_duration_tolerance: u64) {
        self.require_admin();
        let round_duration = self.get_round_duration();
        self.set_round_duration_internal(round_duration, round_duration_tolerance);
    }

    fn set_round_duration_internal(&self, round_duration: u64, round_duration_tolerance: u64) {
        let eff_round_duration = round_duration * (BPS + round_duration_tolerance) / BPS;
        self.round_duration().set(eff_round_duration);
        self.updated_round_duration_event(eff_round_duration);
    }

    /// Supports a native token for pricing. Native tokens are tokens that can be priced by xExchange.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    /// - `token_decimals` - The token decimals.
    /// - `xexchange_pair_address` - The xExchange Pair address.
    /// - `first_anchor_tolerance` - The first anchor tolerance in wad.
    /// - `last_anchor_tolerance` - The last anchor tolerance in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - Needs both xExchange and Price Aggregator support.
    /// - The pair address must be a xExchange pair contract in which either the first or second token identifier is WEGLD.
    /// - The token identifier that is not the WEGLD token identifier is the token identifier for which the price will be
    ///   provided.
    /// - Assumes that the other token in Price Aggregator price feed is always USD.
    /// - Sets the Default method as pricing method and makes sure it is working properly.
    /// - Tokens cannot be removed, they can only be modified through this endpoint. This is because the Controller checks if
    ///   a token is already supported when supporting its corresponding market. Removing a token here would break such
    ///   check. Notice that the only parameter that could have been incorrectly set is the token decimals. All other
    ///   parameters are cross check with the xExchange pair contract.
    ///
    #[endpoint(supportNativeToken)]
    fn support_native_token(&self, token_id: TokenIdentifier, token_decimals: usize, xexchange_pair_address: ManagedAddress, first_anchor_tolerance: BigUint, last_anchor_tolerance: BigUint) {
        self.require_admin();
        self.require_valid_token_identifier_subset(&token_id);

        // USH token cannot be supported nor modified as a native token
        require!(!self.is_ush_token(&token_id), ERROR_UNEXPECTED_TOKEN_ID);

        let token_data = self.get_native_token_data(&token_id, token_decimals, xexchange_pair_address, &first_anchor_tolerance, &last_anchor_tolerance);

        self.supported_tokens(&token_id).set(&token_data);
        self.whitelisted_tokens().insert(token_id);

        // make sure Default algorithm is working properly
        self.set_pricing_method_internal(&token_data, &PricingMethod::Default);

        self.support_token_event(&token_data);
    }

    /// Computes the native token data using information from xExchange and given tolerances.
    ///
    fn get_native_token_data(&self, token_id: &TokenIdentifier, token_decimals: usize, xexchange_pair_address: ManagedAddress, first_anchor_tolerance: &BigUint, last_anchor_tolerance: &BigUint) -> TokenData<Self::Api> {
        let (first_token_id, second_token_id) = self.get_xexchange_pair_tokens(&xexchange_pair_address);
        require!(self.is_wrapped_egld(&first_token_id) || self.is_wrapped_egld(&second_token_id), ERROR_EXPECTED_WEGLD);
        require!(&first_token_id == token_id || &second_token_id == token_id, ERROR_INVALID_XEXCHANGE_PAIR);

        let pair = ExchangePair { address: xexchange_pair_address, token0: first_token_id, token1: second_token_id };
        let tolerances = self.get_anchor_tolerances(first_anchor_tolerance, last_anchor_tolerance);

        let token_data = TokenData {
            token_type: TokenType::Native,
            identifier: token_id.clone(),
            unit_price: BigUint::from(WAD),
            ticker: token_id.ticker(),
            decimals: token_decimals,
            exp: BigUint::from(10u32).pow(token_decimals as u32),
            xexchange_pair: Some(pair),
            tolerances: Some(tolerances),
        };

        token_data
    }

    /// Supports sEGLD pricing using the EGLD Liquid Staking smart contract as the price provider.
    ///
    /// # Arguments:
    ///
    /// - `liquid_staking_sc` - The EGLD Liquid Staking smart contract address.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(supportEgldLsToken)]
    fn support_egld_ls_token(&self, liquid_staking_sc: ManagedAddress) {
        self.require_admin();

        require!(self.liquid_staking().is_empty(), ERROR_ALREADY_SUPPORTED_TOKEN);
        require!(self.is_liquid_staking_sc(&liquid_staking_sc), ERROR_NON_VALID_LS_SC);

        self.liquid_staking().set(&liquid_staking_sc);
        let ls_token_id = self.get_ls_token_id();
        self.ls_token_id().set(&ls_token_id);

        self.support_ls_token_event(&ls_token_id);
    }

    /// Supports SwTAO pricing using the TAO Liquid Staking smart contract as the price provider.
    ///
    /// # Arguments:
    ///
    /// - `liquid_staking_sc` - The TAO Liquid Staking smart contract address.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The TAO token must have been already supported in order to support sTAO token pricing.
    ///
    #[endpoint(supportTaoLsToken)]
    fn support_tao_ls_token(&self, liquid_staking_sc: ManagedAddress) {
        self.require_admin();

        require!(self.tao_liquid_staking().is_empty(), ERROR_ALREADY_SUPPORTED_TOKEN);
        require!(self.is_tao_liquid_staking_sc(&liquid_staking_sc), ERROR_NON_VALID_LS_SC);

        self.tao_liquid_staking().set(&liquid_staking_sc);
        let stao_token_id = self.get_stao_token_id();
        self.stao_token_id().set(&stao_token_id);

        // make sure TAO is supported and its pricing is working properly
        let tao_token_id = self.get_tao_token_id();
        require!(tao_token_id != stao_token_id, ERROR_UNEXPECTED_TOKEN_ID);
        self.get_price_in_egld(&tao_token_id);

        self.support_stao_token_event(&stao_token_id);
    }

    /// Supports USH pricing using the Price Aggregator and a fallback token (USDC) as the price providers.
    ///
    /// # Arguments:
    ///
    /// - `ush_minter` - The USH Minter smart contract address.
    /// - `fallback_token_id` - The fallback token identifier, currently USDC.
    /// - `first_anchor_tolerance` - The first anchor tolerance in wad.
    /// - `last_anchor_tolerance` - The last anchor tolerance in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(supportUshToken)]
    fn support_ush_token(&self, ush_minter: ManagedAddress, fallback_token_id: TokenIdentifier, first_anchor_tolerance: BigUint, last_anchor_tolerance: BigUint) {
        self.require_admin();

        // validate USH Minter
        require!(self.ush_minter().is_empty(), ERROR_ALREADY_SUPPORTED_TOKEN);
        require!(self.is_ush_minter_sc(&ush_minter), ERROR_NON_VALID_USH_MINTER_SC);

        // support fallback token
        self.set_ush_fallback_token_internal(&fallback_token_id);

        // validate and support USH token
        let ush_token_id = self.get_ush_id(&ush_minter);
        self.require_valid_token_identifier_subset(&ush_token_id);

        let tolerances = self.get_anchor_tolerances(&first_anchor_tolerance, &last_anchor_tolerance);
        let token_data = TokenData {
            token_type: TokenType::Synthetic,
            identifier: ush_token_id.clone(),
            unit_price: WAD.into(),
            ticker: USD_SYMBOL.into(),
            decimals: 18,
            exp: WAD.into(),
            xexchange_pair: None, // will eventually have its own xExchange pair, once it is accepted as collateral
            tolerances: Some(tolerances),
        };

        // store USH token
        self.ush_minter().set(&ush_minter);
        self.ush_token_id().set(&ush_token_id);

        self.supported_tokens(&ush_token_id).set(&token_data);
        self.whitelisted_tokens().insert(ush_token_id);

        // make sure Default algorithm is working properly
        self.set_pricing_method_internal(&token_data, &PricingMethod::Default);

        self.support_ush_token_event(&ush_minter, &token_data);
    }

    /// Sets a new fallback token for USH pricing.
    ///
    /// # Arguments:
    ///
    /// - `fallback_token_id` - The new fallback token identifier.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The fallback token must be supported and whitelisted as a valid fallback token.
    ///
    #[endpoint(setUshFallbackToken)]
    fn set_ush_fallback_token(&self, fallback_token_id: TokenIdentifier) {
        self.require_admin();

        require!(!self.ush_token_id().is_empty(), ERROR_UNSUPPORTED_USH_TOKEN);

        let current_fallback_token_id = self.ush_fallback_token_id().get();
        require!(current_fallback_token_id != fallback_token_id, ERROR_SAME_FALLBACK_TOKEN);
        self.set_ush_fallback_token_internal(&fallback_token_id);

        // make sure that the Default pricing method is working properly with the new fallback token. Notice that we might
        // have to unpause USH pricing because the current fallback token might have paused it before.
        let ush_token_id = self.ush_token_id().get();
        self.check_default_pricing_method(&ush_token_id);
    }

    /// Sets a new fallback token for USH pricing.
    ///
    /// # Arguments:
    ///
    /// - `fallback_token_id` - The new fallback token identifier.
    ///
    fn set_ush_fallback_token_internal(&self, fallback_token_id: &TokenIdentifier) {
        self.require_supported_token(fallback_token_id);

        // check if the fallback token is whitelisted
        require!(self.get_whitelisted_fallback_tokens().contains(fallback_token_id), ERROR_INVALID_FALLBACK_TOKEN);

        // make sure the fallback token is currently unpaused. If needed, unpause it before setting it as fallback token.
        require!(!self.is_token_paused(fallback_token_id).get(), ERROR_FALLBACK_TOKEN_PRICING_PAUSED);

        // make sure the fallback token has a reliable price
        self.check_default_pricing_method(fallback_token_id);

        // set the fallback token
        self.ush_fallback_token_id().set(fallback_token_id);

        self.set_ush_fallback_token_event(fallback_token_id);
    }

    /// Sets a pricing method for the given token.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The ESDT token identifier.
    /// - `pricing_method` - The pricing method.
    ///
    #[endpoint(setPricingMethod)]
    fn set_pricing_method(&self, token_id: TokenIdentifier, pricing_method: PricingMethod) {
        self.require_admin();
        self.require_supported_token(&token_id);
        let token_data = self.supported_tokens(&token_id).get();
        self.set_pricing_method_internal(&token_data, &pricing_method);
        self.pricing_method_event(&token_id, &pricing_method);
    }

    fn set_pricing_method_internal(&self, token_data: &TokenData<Self::Api>, pricing_method: &PricingMethod) {
        let TokenData { identifier: token_id, xexchange_pair: opt_pair, .. } = token_data;

        match pricing_method {
            PricingMethod::None => {
                sc_panic!(ERROR_UNEXPECTED_PRICING_METHOD);
            },
            PricingMethod::Default => {
                self.check_default_pricing_method(token_id);
            },
            PricingMethod::Instantaneous => {
                require!(self.xexchange_pricing_method().get() != ExchangePricingMethod::SafePriceOnly, ERROR_XEXCHANGE_SAFE_PRICE_ONLY);
                self.get_xexchange_instantaneous_price_in_egld_internal(token_data);
                self.unreliable_pricing_method_event(token_id, pricing_method);
            },
            PricingMethod::Safe => {
                self.get_xexchange_safe_price_in_egld_internal(token_data, false);
                self.unreliable_pricing_method_event(token_id, pricing_method);
            },
            PricingMethod::PriceAggregator => {
                if !self.is_ush_token(token_id) {
                    self.require_egld_wrapper_or_xexchange_paused(&opt_pair.as_ref().unwrap().address);
                } else {
                    // if EGLD wrapper is paused, we don't need to check for anything else
                    if !self.is_egld_wrapper_paused() {
                        // store current fallback token
                        let ush_fallback_token_id = self.ush_fallback_token_id().get();

                        for fallback_token_id in self.get_whitelisted_fallback_tokens().into_iter() {
                            // notice there will always be at least one supported token
                            if !self.is_supported_token(&fallback_token_id) {
                                continue;
                            }

                            // get the fallback token data
                            let fallback_token_data = self.get_supported_token_data(&fallback_token_id);

                            // check if the fallback token has a reliable price
                            let has_reliable_price = self.is_default_price_reliable(&fallback_token_data);

                            // if a fallback token does not have a reliable price, we can't use the price aggregator as we
                            // can't be sure the EGLD price in USD is working properly
                            require!(has_reliable_price, ERROR_CANNOT_USE_PRICE_AGGREGATOR);

                            // check if USH is depegged from this fallback token, meaning that the fallback tokens is not
                            // pegged to the USD anymore
                            self.ush_fallback_token_id().set(&fallback_token_id);
                            let is_depegged = !self.is_default_price_reliable(token_data);

                            // check if the xExchange pair is paused
                            let pair_address = fallback_token_data.xexchange_pair.unwrap().address;
                            let is_xexchange_paused = self.is_xexchange_paused(&pair_address);

                            // if the xExchange pair is not paused or the token is not depegged, switch to that fallback
                            // token instead
                            require!(is_xexchange_paused || is_depegged, ERROR_CHANGE_FALLBACK_TOKEN);
                        }

                        // restore fallback token
                        self.ush_fallback_token_id().set(ush_fallback_token_id);
                    }
                }

                self.get_price_aggregator_price_in_egld_internal(token_data);
                self.unreliable_pricing_method_event(token_id, pricing_method);
            },
        }

        self.pricing_method(token_id).set(pricing_method);
    }

    /// Sets a new first and last anchor tolerances for a given token.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    /// - `first_anchor_tolerance` - The new first anchor tolerance in wad.
    /// - `last_anchor_tolerance` - The new last anchor tolerance in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The token must have been already supported.
    /// - An anchor tolerance is the maximum allowed price change between the anchor price and the reporter price.
    /// - An anchor tolerance equal to 0 implies that the anchor price and reporter price should be equal.
    /// - An anchor tolerance equal to bps = 1e4 = 100%, allows an upwards 100% price change and 50% downwards price change.
    /// - Anchor tolerances can be higher than 100%.
    ///
    #[endpoint(setAnchorTolerances)]
    fn set_anchor_tolerances(&self, token_id: TokenIdentifier, first_anchor_tolerance: BigUint, last_anchor_tolerance: BigUint) {
        self.require_admin();
        self.require_supported_token(&token_id);
        self.set_anchor_tolerances_internal(&token_id, &first_anchor_tolerance, &last_anchor_tolerance);
    }

    fn set_anchor_tolerances_internal(&self, token_id: &TokenIdentifier, first_anchor_tolerance: &BigUint, last_anchor_tolerance: &BigUint) {
        let tolerances = self.get_anchor_tolerances(first_anchor_tolerance, last_anchor_tolerance);
        let mut token_data = self.get_supported_token_data(token_id);
        token_data.tolerances = Some(tolerances.clone());
        self.supported_tokens(token_id).set(token_data);
        self.anchor_tolerances_event(token_id, &tolerances);
    }

    fn get_anchor_tolerances(&self, first_anchor_tolerance: &BigUint, last_anchor_tolerance: &BigUint) -> ToleranceData<Self::Api> {
        require!(first_anchor_tolerance >= &MIN_FIRST_ANCHOR_TOLERANCE && first_anchor_tolerance <= &MAX_FIRST_ANCHOR_TOLERANCE, ERROR_UNEXPECTED_FIRST_ANCHOR_TOLERANCE);
        require!(last_anchor_tolerance >= &MIN_LAST_ANCHOR_TOLERANCE && last_anchor_tolerance <= &MAX_LAST_ANCHOR_TOLERANCE, ERROR_UNEXPECTED_LAST_ANCHOR_TOLERANCE);
        require!(last_anchor_tolerance >= first_anchor_tolerance, ERROR_UNEXPECTED_ANCHOR_TOLERANCES);

        let (first_upper_bound_ratio, first_lower_bound_ratio) = self.get_bounds(first_anchor_tolerance);
        let (last_upper_bound_ratio, last_lower_bound_ratio) = self.get_bounds(last_anchor_tolerance);

        let tolerances = ToleranceData {
            first_upper_bound_ratio,
            first_lower_bound_ratio,
            last_upper_bound_ratio,
            last_lower_bound_ratio,
        };

        tolerances
    }

    // Utility

    /// Checks if the default pricing method is working properly, i.e. verifies that the token price is reliable (within the
    /// first anchor tolerance). During this process, if possible, the unreliable price is set to false.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    ///
    fn check_default_pricing_method(&self, token_id: &TokenIdentifier) {
        let token_data = self.get_supported_token_data(token_id);
        require!(self.is_default_price_reliable(&token_data), ERROR_TOKEN_PRICE_NOT_RELIABLE);
        self.get_default_price_in_egld_internal(&token_data);
    }

    /// Utility function to check if the default price is reliable.
    ///
    /// # Arguments:
    ///
    /// - `token_data` - The token data.
    ///
    fn is_default_price_reliable(&self, token_data: &TokenData<Self::Api>) -> bool {
        let TokenData { tolerances: opt_tolerances, .. } = token_data;
        let tolerances = opt_tolerances.as_ref().unwrap();
        let anchor_price = self.get_xexchange_safe_price_in_egld_internal(token_data, true);
        let reporter_price = self.get_price_aggregator_price_in_egld_internal(token_data);
        self.is_within_first_anchor(tolerances, &reporter_price, &anchor_price)
    }
}
