multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::{common, constants::*, errors::*, events, model::*, proxies, storage};

#[multiversx_sc::module]
pub trait PriceModule: admin::AdminModule + events::EventsModule + proxies::ProxyModule + common::CommonModule + storage::StorageModule {
    /// Returns the token price in EGLD and in WAD units.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The identifier of the token to retrieve the price of.
    ///
    /// # Notes:
    ///
    /// - The WEGLD price always equals to one.
    /// - The SEGLD price is retrieved from the Liquid Staking smart contract.
    ///
    #[endpoint(getPrice)]
    fn get_price_in_egld(&self, token_id: &TokenIdentifier) -> BigUint {
        self.require_valid_token_identifier(token_id);

        if self.is_wrapped_egld(token_id) {
            // if the EGLD Wrapper Smart Contract is not paused, the Oracle assumes that WEGLD == EGLD
            self.require_egld_wrapper_not_paused();
            return BigUint::from(WAD);
        }

        // the Oracle fetches SEGLD price from Liquid Staking
        if self.is_ls_token(token_id) {
            let price = self.get_ls_token_price();
            self.set_last_price(token_id, &price);
            return price;
        }

        // the Oracle fetches sTAO price from TAO Liquid Staking and TAO token pricing
        if self.is_stao_token(token_id) {
            let fx = self.get_tao_ls_exchange_rate();
            let tao_token_id = self.get_tao_token_id();
            let tao_price = self.get_price_in_egld(&tao_token_id);

            let price = fx * tao_price / BigUint::from(WAD);
            self.set_last_price(token_id, &price);
            return price;
        }

        // if the token is not WEGLD, SEGLD nor STAO, it must have been supported
        self.require_supported_token(token_id);
        let pricing_method = self.get_pricing_method(token_id);
        let token_data = self.get_supported_token_data(token_id);

        match pricing_method {
            PricingMethod::None => {
                sc_panic!(ERROR_CANNOT_PRICE_TOKEN);
            },
            PricingMethod::Default => {
                // if the token pricing has been paused, it cannot be priced
                require!(!self.is_token_paused(token_id).get(), ERROR_TOKEN_PRICING_PAUSED);
                self.get_default_price_in_egld_internal(&token_data)
            },
            PricingMethod::Instantaneous => {
                let price = self.get_xexchange_instantaneous_price_in_egld_internal(&token_data);
                self.unreliable_pricing_method_event(token_id, &PricingMethod::Instantaneous);
                self.set_last_price(token_id, &price);
                price
            },
            PricingMethod::Safe => {
                let price = self.get_xexchange_safe_price_in_egld_internal(&token_data, false);
                self.unreliable_pricing_method_event(token_id, &PricingMethod::Safe);
                self.set_last_price(token_id, &price);
                price
            },
            PricingMethod::PriceAggregator => {
                let price = self.get_price_aggregator_price_in_egld_internal(&token_data);
                self.unreliable_pricing_method_event(token_id, &PricingMethod::PriceAggregator);
                self.set_last_price(token_id, &price);
                price
            },
        }
    }

    /// Checks if the reporter price is within the first anchor price bounds.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    /// - `reporter_price` - The price reported by the reporter (in EGLD).
    /// - `anchor_price` - The anchor price (in EGLD).
    ///
    fn is_within_first_anchor(&self, tolerances: &ToleranceData<Self::Api>, reporter_price: &BigUint, anchor_price: &BigUint) -> bool {
        let ToleranceData { first_upper_bound_ratio, first_lower_bound_ratio, .. } = tolerances;
        self.is_within_anchor_internal(reporter_price, anchor_price, first_upper_bound_ratio, first_lower_bound_ratio)
    }

    /// Checks if the reporter price is within the last anchor price bounds.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier.
    /// - `reporter_price` - The price reported by the reporter (in EGLD).
    /// - `anchor_price` - The anchor price (in EGLD).
    ///
    fn is_within_last_anchor(&self, tolerances: &ToleranceData<Self::Api>, reporter_price: &BigUint, anchor_price: &BigUint) -> bool {
        let ToleranceData { last_upper_bound_ratio, last_lower_bound_ratio, .. } = tolerances;
        self.is_within_anchor_internal(reporter_price, anchor_price, last_upper_bound_ratio, last_lower_bound_ratio)
    }

    fn is_within_anchor_internal(&self, reporter_price: &BigUint, anchor_price: &BigUint, upper_bound_ratio: &BigUint, lower_bound_ratio: &BigUint) -> bool {
        let wad = BigUint::from(WAD);
        let anchor_ratio = anchor_price * &wad / reporter_price;
        &anchor_ratio <= upper_bound_ratio && &anchor_ratio >= lower_bound_ratio
    }

    /// Returns the token price based on the `Default` algorithm, which compares the xExchange Safe price with the Price
    /// Aggregator price
    ///
    /// # Arguments:
    ///
    /// - `token_data` - The token data.
    ///
    fn get_default_price_in_egld_internal(&self, token_data: &TokenData<Self::Api>) -> BigUint {
        let TokenData { identifier: token_id, tolerances: opt_tolerances, .. } = token_data;

        let anchor_price = self.get_xexchange_safe_price_in_egld_internal(token_data, true);
        let reporter_price = self.get_price_aggregator_price_in_egld_internal(token_data);

        let tolerances = opt_tolerances.as_ref().unwrap();
        if self.is_within_first_anchor(tolerances, &reporter_price, &anchor_price) {
            self.has_unreliable_price(token_id).set(false);
            self.set_last_price(token_id, &reporter_price);
            return reporter_price;
        } else if self.is_within_last_anchor(tolerances, &reporter_price, &anchor_price) {
            require!(!self.has_unreliable_price(token_id).get(), ERROR_TOKEN_HAS_UNRELIABLE_PRICE);
            self.has_unreliable_price(token_id).set(true);
            self.first_anchor_surpassed_event(token_id, &reporter_price, &anchor_price);
            return self.last_price(token_id).get();
        }

        // pause the token pricing if the price is not within the first nor the last anchor
        self.is_token_paused(token_id).set(true);

        // emit events
        self.pause_token_event(token_id);
        self.last_anchor_surpassed_event(token_id, &reporter_price, &anchor_price);

        // retrieve last valid price
        self.last_price(token_id).get()
    }

    /// Returns the xExchange price of a token in EGLD, based on its paired liquidity pool reserves.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The identifier of the token for which to retrieve the price.
    ///
    #[endpoint(getXExchangeInstantaneousPriceInEgld)]
    fn get_xexchange_instantaneous_price_in_egld(&self, token_id: &TokenIdentifier) -> BigUint {
        self.require_supported_token(token_id);
        let token_data = self.get_supported_token_data(token_id);
        self.get_xexchange_instantaneous_price_in_egld_internal(&token_data)
    }

    fn get_xexchange_instantaneous_price_in_egld_internal(&self, token_data: &TokenData<Self::Api>) -> BigUint {
        let is_ush = self.is_ush_token(&token_data.identifier);

        let TokenData { identifier: token_id, token_type, xexchange_pair: opt_xexchange_pair, exp: exp_token, .. } = if is_ush {
            // For USH token, use the fallback token's data
            let ush_fallback_token_id = self.ush_fallback_token_id().get();
            &self.get_supported_token_data(&ush_fallback_token_id)
        } else {
            token_data
        };

        require!(token_type == &TokenType::Native, ERROR_UNEXPECTED_TOKEN_TYPE);
        let xexchange_pair = opt_xexchange_pair.as_ref().unwrap();

        self.require_egld_wrapper_not_paused();
        self.require_xexchange_not_paused(&xexchange_pair.address);

        let (reserves0, reserves1) = self.get_xexchange_reserves(&xexchange_pair.address);
        let price = if self.is_wrapped_egld(&xexchange_pair.token0) { reserves0 * WAD / reserves1 } else { reserves1 * WAD / reserves0 };
        require!(price > BigUint::zero(), ERROR_PRICE_IS_ZERO);

        self.xexchange_price_fetched_event(token_id, &price);

        // if the token is USH, we need to convert from fallback token's units to USH's units
        if is_ush {
            return price * exp_token / WAD;
        }

        price
    }

    /// Returns the safe price of a token in EGLD using the xExchange Protocol.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The identifier of the token for which the safe price is calculated.
    ///
    #[endpoint(getXExchangeSafePriceInEgld)]
    fn get_xexchange_safe_price_in_egld(&self, token_id: &TokenIdentifier) -> BigUint {
        self.require_supported_token(token_id);
        let token_data = self.get_supported_token_data(token_id);
        self.get_xexchange_safe_price_in_egld_internal(&token_data, false)
    }

    fn get_xexchange_safe_price_in_egld_internal(&self, token_data: &TokenData<Self::Api>, xexchange_pause_allowed: bool) -> BigUint {
        let is_ush = self.is_ush_token(&token_data.identifier);

        let TokenData { identifier: token_id, token_type, xexchange_pair: opt_xexchange_pair, exp: exp_token, .. } = if is_ush {
            // For USH token, use the fallback token's data
            let fallback_token_id = self.ush_fallback_token_id().get();
            &self.get_supported_token_data(&fallback_token_id)
        } else {
            token_data
        };

        require!(token_type == &TokenType::Native, ERROR_UNEXPECTED_TOKEN_TYPE);
        let xexchange_pair = opt_xexchange_pair.as_ref().unwrap();

        self.require_egld_wrapper_not_paused();
        if !xexchange_pause_allowed {
            self.require_xexchange_not_paused(&xexchange_pair.address);
        }

        let input = EsdtTokenPayment::new(token_id.clone(), 0, WAD.into());
        let price = self.update_and_get_xexchange_safe_price(&xexchange_pair.address, input);

        self.xexchange_safe_price_fetched_event(token_id, &price);

        // if the token is USH, we need to convert from fallback token's units to USH's units
        if is_ush {
            return price * exp_token / WAD;
        }

        price
    }

    /// Returns the price of a given token in EGLD, as reported by the Price Aggregator.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The identifier of the token.
    ///
    #[endpoint(getPriceAggregatorPriceInEgld)]
    fn get_price_aggregator_price_in_egld(&self, token_id: &TokenIdentifier) -> BigUint {
        self.require_supported_token(token_id);
        let token_data = self.get_supported_token_data(token_id);
        self.get_price_aggregator_price_in_egld_internal(&token_data)
    }

    fn get_price_aggregator_price_in_egld_internal(&self, token_data: &TokenData<Self::Api>) -> BigUint {
        let TokenData { identifier: token_id, unit_price, ticker, exp: exp_token, .. } = token_data;

        let exp_egld = BigUint::from(WAD);
        let usd = ManagedBuffer::from(USD_SYMBOL);
        let egld = ManagedBuffer::from(EGLD_SYMBOL);
        let egld_in_usd = self.get_price_aggregator_latest_price(&egld, &usd);
        let token_in_usd = self.get_price_aggregator_latest_price(ticker, &usd);

        // unit price is already in wad units
        let price = unit_price * &token_in_usd * exp_egld / (egld_in_usd * exp_token);
        require!(price > BigUint::zero(), ERROR_PRICE_IS_ZERO);

        self.price_aggregator_price_fetched_event(token_id, &price);

        price
    }
}
