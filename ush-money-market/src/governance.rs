multiversx_sc::imports!();

use super::{commons, constants::*, errors::*, events, proxies, storage, storage::State};

#[multiversx_sc::module]
pub trait GovernanceModule: admin::AdminModule + commons::CommonsModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Activates the USH Money Market.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(activate)]
    fn activate(&self) {
        self.require_admin();
        self.require_hush_issued();
        self.require_not_finalized_state();

        // should be a market observer to get notified when account collaterals change
        // the following check implicitly verifies that the USH money market has been whitelisted at Controller
        let sc_address = self.blockchain().get_sc_address();
        require!(self.is_ush_market_observer(&sc_address), ERROR_NOT_A_MARKET_OBSERVER);

        // should be a USH facilitator to be able to mint and burn USH
        require!(self.is_facilitator(&sc_address), ERROR_NOT_FACILITATOR);

        // market should have minting role
        let hush_id = self.hush_id().get();
        let flags = self.blockchain().get_esdt_local_roles(&hush_id);
        require!(flags.has_role(&EsdtLocalRole::Mint), ERROR_MISSING_MARKET_ROLES);

        // borrow rate should have been set
        require!(!self.borrow_rate().is_empty(), ERROR_UNDEFINED_BORROW_RATE);

        // should have set a discount rate model
        require!(!self.discount_rate_model().is_empty(), ERROR_UNDEFINED_DISCOUNT_RATE_MODEL);

        self.set_ush_market_state_internal(State::Active);
    }

    /// Finalizes the Market. From this point onwards it can be removed from the Controller as an Observer.
    ///
    #[endpoint(finalize)]
    fn finalize(&self) {
        self.require_admin();

        let sc_address = self.blockchain().get_sc_address();
        require!(self.is_deprecated_market(&sc_address), ERROR_MARKET_NOT_DEPRECATED);
        require!(self.market_borrowers().is_empty(), ERROR_MARKET_HAS_BORROWERS);

        self.set_ush_market_state_internal(State::Finalized);
    }

    /// Updates the Staking smart contract address.
    ///
    /// # Arguments:
    ///
    /// - `staking_sc` - The Staking smart contract address.
    ///
    #[endpoint(setStakingContract)]
    fn set_staking_contract(&self, staking_sc: &ManagedAddress) {
        self.require_admin();
        require!(self.is_staking_sc(staking_sc), ERROR_INVALID_STAKING_SC);
        self.staking_sc().set(staking_sc);
        self.set_staking_contract_event(staking_sc);
    }

    /// Updates the stake factor, i.e. the portion of the reserves that is used as staking rewards.
    ///
    /// # Arguments:
    ///
    /// - `stake_factor` - The new reserve factor in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The new stake factor must not exceed the maximum allowed.
    ///
    #[endpoint(setStakeFactor)]
    fn set_stake_factor(&self, stake_factor: BigUint) {
        self.require_admin();

        require!(stake_factor <= BigUint::from(WAD), ERROR_STAKE_FACTOR_TOO_HIGH);

        self.accrue_interest();
        self.require_market_fresh();

        self.stake_factor().set(&stake_factor);

        self.set_stake_factor_event(&stake_factor);
    }

    /// Updates the close factor used at liquidations.
    ///
    /// # Arguments:
    ///
    /// - `close_factor` - The new close factor in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setCloseFactor)]
    fn set_close_factor(&self, close_factor: BigUint) {
        self.require_admin();
        require!(close_factor >= BigUint::from(MIN_CLOSE_FACTOR), ERROR_CLOSE_FACTOR_TOO_LOW);
        require!(close_factor <= BigUint::from(WAD), ERROR_CLOSE_FACTOR_TOO_HIGH);
        self.close_factor().set(&close_factor);
        self.set_close_factor_event(&close_factor);
    }

    /// Updates the liquidation incentive for liquidations.
    ///
    /// # Arguments
    ///
    /// - `liquidation_incentive` - The new liquidation incentive in wad.
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - the new liquidation incentive should not be less than the amount that would yield losses for liquidators
    /// - the new liquidation incentive should be less than `1 / eff_ltv`, which is conservatively assumed to be `1 /
    ///   max_ltv`. Otherwise, there won't be a Risky region.
    ///
    #[endpoint(setLiquidationIncentive)]
    fn set_liquidation_incentive(&self, liquidation_incentive: BigUint) {
        self.require_admin();

        let wad = BigUint::from(WAD);
        let max_ltv = self.get_max_collateral_factor();
        let min_li = BigUint::from(MIN_LIQUIDATION_INCENTIVE);
        let protocol_seize_share = self.protocol_seize_share().get();

        require!(&liquidation_incentive * &(&wad - &protocol_seize_share) >= min_li * &wad, ERROR_LIQUIDATION_INCENTIVE_TOO_LOW);
        require!(&liquidation_incentive * &max_ltv < &wad * &wad, ERROR_LIQUIDATION_INCENTIVE_TOO_HIGH);

        self.liquidation_incentive().set(&liquidation_incentive);

        self.set_liquidation_incentive_event(&liquidation_incentive);
    }

    /// Updates the protocol seize share, i.e. the portion of the seized amount that is kept by the protocol.
    ///
    /// # Arguments
    ///
    /// - `protocol_seize_share` - The new protocol seize share in wad.
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - the new protocol seize share cannot exceed the amount that would yield losses for liquidators
    ///
    #[endpoint(setProtocolSeizeShare)]
    fn set_protocol_seize_share(&self, protocol_seize_share: BigUint) {
        self.require_admin();

        let wad = BigUint::from(WAD);
        let min_li = BigUint::from(MIN_LIQUIDATION_INCENTIVE);
        let liquidation_incentive = self.get_liquidation_incentive();

        require!(liquidation_incentive * (&wad - &protocol_seize_share) >= min_li * wad, ERROR_PROTOCOL_SEIZE_SHARE_TOO_HIGH);

        self.protocol_seize_share().set(&protocol_seize_share);

        self.set_protocol_seize_share_event(&protocol_seize_share);
    }

    /// Updates the borrow rate.
    ///
    /// # Arguments
    ///
    /// - `borrow_apr` - The new borrow APR in wad.
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - the borrow rate cannot increase nor decrease too much
    /// - there is a time delay to increase the borrow rate
    /// - the borrow rate cannot be set to zero
    ///
    #[endpoint(setBorrowApr)]
    fn set_borrow_apr(&self, borrow_apr: BigUint) {
        self.require_admin();

        // compute borrow rate per second
        let borrow_rate = borrow_apr / SECONDS_PER_YEAR;
        let timestamp = self.blockchain().get_block_timestamp();

        // setting a zero borrow rate is not allowed
        require!(borrow_rate != BigUint::zero(), ERROR_BORROW_RATE_CANNOT_BE_ZERO);

        if self.borrow_rate().is_empty() {
            // if it is the first time the borrow rate is set, it must be less than the maximum initial borrow rate
            require!(borrow_rate <= BigUint::from(MAX_INITIAL_BORROW_RATE), ERROR_INVALID_INITIAL_BORROW_RATE);
        } else {
            let old_borrow_rate = self.borrow_rate().get();
            require!(borrow_rate != old_borrow_rate, ERROR_EQUAL_BORROW_RATE);
            require!(self.is_borrow_rate_change_allowed(&old_borrow_rate, &borrow_rate), ERROR_INVALID_BORROW_RATE_UPDATE);
            if borrow_rate > old_borrow_rate {
                // ensure increases in the borrow rate are appropriately timed and within acceptable limits
                require!(timestamp - self.last_borrow_rate_update().get() >= BORROW_RATE_DELAY, ERROR_BORROW_RATE_UPDATE_TOO_SOON);
            }
        }

        self.accrue_interest();
        self.require_market_fresh();

        self.borrow_rate().set(&borrow_rate);
        self.last_borrow_rate_update().set(timestamp);

        self.set_borrow_rate_event(&borrow_rate);
    }

    /// Updates the Discount Rate Model.
    ///
    /// # Arguments
    ///
    /// - `discount_rate_model` - The Discount Rate Model smart contract address.
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - the provided address must be a valid Discount Rate Model smart contract
    ///
    #[endpoint(setDiscountRateModel)]
    fn set_discount_rate_model(&self, discount_rate_model: &ManagedAddress) {
        self.require_admin();

        require!(self.is_discount_rate_model_sc(discount_rate_model), ERROR_INVALID_DISCOUNT_RATE_MODEL_SC);

        // make sure the discount rate model has been initialized with the correct USH Money Market
        let sc_address = self.blockchain().get_sc_address();
        let ush_money_market = self.get_ush_money_market(discount_rate_model);
        require!(sc_address == ush_money_market, ERROR_UNEXPECTED_MARKET_AT_DISCOUNT_RATE_MODEL_SC);

        self.accrue_interest();
        self.require_market_fresh();

        self.discount_rate_model().set(discount_rate_model);
        self.set_discount_rate_model_event(discount_rate_model);
    }

    /// Withdraws an specified amount of USH from the money market reserves (revenue part) to the admin account.
    ///
    /// # Arguments:
    ///
    /// - `opt_ush_amount` - The amount of USH to withdraw. If not provided, the entire revenue is withdrawn.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The USH amount is directed to the admin account.
    ///
    #[endpoint(reduceReserves)]
    fn reduce_reserves(&self, opt_ush_amount: OptionalValue<BigUint>) {
        self.require_admin();

        self.accrue_interest();
        self.require_market_fresh();

        let revenue = self.revenue().get();
        let ush_amount = opt_ush_amount.into_option().unwrap_or_else(|| revenue.clone());

        require!(ush_amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);

        require!(ush_amount <= revenue, ERROR_AMOUNT_EXCEEDS_REVENUE);

        // update reserves and revenue
        self.total_reserves().update(|amount| *amount -= &ush_amount);
        self.revenue().update(|amount| *amount -= &ush_amount);

        // mint USH to the admin
        let admin = self.get_admin();
        self.ush_minter_mint(&ush_amount, OptionalValue::Some(admin));

        self.reserves_reduced_event(&ush_amount);
    }

    /// Updates the accrual time threshold.
    ///
    /// # Arguments:
    ///
    /// - `accrual_time_threshold` - The new accrual time threshold in seconds.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setAccrualTimeThreshold)]
    fn set_accrual_time_threshold(&self, accrual_time_threshold: u64) {
        self.require_admin();

        require!(accrual_time_threshold <= MAX_ACCRUAL_TIME_THRESHOLD, ERROR_ACCRUAL_TIME_THRESHOLD_TOO_HIGH);

        self.accrue_interest();
        self.require_market_fresh();

        self.accrual_time_threshold().set(accrual_time_threshold);

        self.set_accrual_time_threshold_event(accrual_time_threshold);
    }

    /// Whitelists a trusted minter contract, i.e. a contract that can mint and enter market in the name of someone else.
    ///
    /// # Arguments:
    ///
    /// - `trusted_minter` - the new trusted minter to whitelist
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - `trusted_minter` must be a trusted smart contract
    /// - `trusted_minter` must not be already trusted
    ///
    #[endpoint(addTrustedMinter)]
    fn add_trusted_minter(&self, trusted_minter: ManagedAddress) {
        self.require_admin();
        self.require_not_trusted_minter(&trusted_minter);
        require!(self.is_trusted_minter_sc(&trusted_minter), ERROR_INVALID_TRUSTED_MINTER_SC);
        self.trusted_minters_list().add(&trusted_minter);
        self.add_trusted_minter_event(&trusted_minter);
    }

    /// Removes a trusted minter contract address from the whitelist of trusted minters contracts.
    ///
    /// # Arguments:
    ///
    /// - `trusted_minter` - the trusted minter to remove
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - `trusted_minter` must has been already trusted
    ///
    #[endpoint(removeTrustedMinter)]
    fn remove_trusted_minter(&self, trusted_minter: ManagedAddress) {
        self.require_admin();
        self.require_trusted_minter(&trusted_minter);
        self.trusted_minters_list().remove(&trusted_minter);
        self.remove_trusted_minter_event(&trusted_minter);
    }
}
