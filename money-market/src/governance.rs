multiversx_sc::imports!();

use super::{common, constants::*, errors::*, events, proxies, storage};

#[multiversx_sc::module]
pub trait GovernanceModule: admin::AdminModule + common::CommonModule + events::EventsModule + storage::StorageModule + proxies::ProxyModule {
    /// Sets the staking smart contract address.
    ///
    /// # Arguments:
    ///
    /// - `new_staking` - The Staking smart contract address.
    ///
    #[endpoint(setStakingContract)]
    fn set_staking_contract(&self, new_staking: &ManagedAddress) {
        self.require_admin();

        require!(self.is_staking_sc(new_staking), ERROR_NON_VALID_STAKING_SC);

        let old_staking = self.get_staking_contract();
        self.staking_contract().set(new_staking);

        self.new_staking_contract_event(&old_staking, new_staking);
    }

    /// Sets a new reserve factor.
    ///
    /// # Arguments:
    ///
    /// - `new_reserve_factor` - The new reserve factor in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The new reserve factor must not exceed the maximum allowed.
    ///
    #[endpoint(setReserveFactor)]
    fn set_reserve_factor(&self, new_reserve_factor: &BigUint) {
        self.require_admin();

        require!(new_reserve_factor <= &BigUint::from(WAD), ERROR_RESERVE_FACTOR_TOO_HIGH);

        self.accrue_interest();
        self.require_market_fresh();

        let old_reserve_factor = self.reserve_factor().get();
        self.reserve_factor().set(new_reserve_factor);

        self.emit_updated_rates();
        self.new_reserve_factor_event(&old_reserve_factor, new_reserve_factor);
    }

    /// Sets a new stake factor, i.e. the portion of the reserves that is used as staking rewards.
    ///
    /// # Arguments:
    ///
    /// - `new_stake_factor` - The new reserve factor in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The new stake factor must not exceed the maximum allowed.
    ///
    #[endpoint(setStakeFactor)]
    fn set_stake_factor(&self, new_stake_factor: &BigUint) {
        self.require_admin();

        require!(new_stake_factor <= &BigUint::from(WAD), ERROR_STAKE_FACTOR_TOO_HIGH);

        self.accrue_interest();
        self.require_market_fresh();

        let old_stake_factor = self.stake_factor().get();
        self.stake_factor().set(new_stake_factor);

        self.new_stake_factor_event(&old_stake_factor, new_stake_factor);
    }

    /// Sets a new close factor used at liquidations.
    ///
    /// # Arguments:
    ///
    /// - `new_close_factor` - The new close factor in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setCloseFactor)]
    fn set_close_factor(&self, new_close_factor: &BigUint) {
        self.require_admin();

        require!(new_close_factor >= &BigUint::from(MIN_CLOSE_FACTOR), ERROR_CLOSE_FACTOR_TOO_LOW);
        require!(new_close_factor <= &BigUint::from(WAD), ERROR_CLOSE_FACTOR_TOO_HIGH);

        let old_close_factor = self.get_close_factor();
        self.close_factor().set(new_close_factor);

        self.new_close_factor_event(&old_close_factor, new_close_factor);
    }

    /// Sets a new liquidation incentive for liquidations.
    ///
    /// # Arguments
    ///
    /// - `new_liquidation_incentive` - the new liquidation incentive in wad
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - the new liquidation incentive should not be less than the amount that would yield losses for liquidators
    /// - the new liquidation incentive should be less than `1 / eff_ltv`, which is conservatively assumed to be `1 /
    ///   max_ltv`. Otherwise, there won't be a Risky region.
    ///
    #[endpoint(setLiquidationIncentive)]
    fn set_liquidation_incentive(&self, new_liquidation_incentive: &BigUint) {
        self.require_admin();

        let wad = BigUint::from(WAD);
        let max_ltv = self.get_max_collateral_factor();
        let min_li = BigUint::from(MIN_LIQUIDATION_INCENTIVE);
        let protocol_seize_share = self.protocol_seize_share().get();

        require!(new_liquidation_incentive * &(&wad - &protocol_seize_share) >= min_li * &wad, ERROR_LIQUIDATION_INCENTIVE_TOO_LOW);
        require!(new_liquidation_incentive * &max_ltv < &wad * &wad, ERROR_LIQUIDATION_INCENTIVE_TOO_HIGH);

        let old_liquidation_incentive = self.get_liquidation_incentive();
        self.liquidation_incentive().set(new_liquidation_incentive);

        self.new_liquidation_incentive_event(&old_liquidation_incentive, new_liquidation_incentive);
    }

    /// Sets a new protocol seize share, i.e. the portion of the seized amount that is kept by the protocol.
    ///
    /// # Arguments
    ///
    /// - `new_protocol_seize_share` - the new protocol seize share in wad
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - the new protocol seize share cannot exceed the amount that would yield losses for liquidators
    ///
    #[endpoint(setProtocolSeizeShare)]
    fn set_protocol_seize_share(&self, new_protocol_seize_share: &BigUint) {
        self.require_admin();

        let wad = BigUint::from(WAD);
        let min_li = BigUint::from(MIN_LIQUIDATION_INCENTIVE);
        let liquidation_incentive = self.get_liquidation_incentive();

        require!(liquidation_incentive * (&wad - new_protocol_seize_share) >= min_li * wad, ERROR_PROTOCOL_SEIZE_SHARE_TOO_HIGH);

        let old_protocol_seize_share = self.protocol_seize_share().get();
        self.protocol_seize_share().set(new_protocol_seize_share);

        self.new_protocol_seize_share_event(&old_protocol_seize_share, new_protocol_seize_share);
    }

    /// Sets a new Interest Rate Model.
    ///
    /// # Arguments:
    ///
    /// - `new_interest_rate_model` - The address of the new Interest Rate Model smart contract.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The provided address must be a valid Interest Rate Model smart contract.
    ///
    #[endpoint(setInterestRateModel)]
    fn set_interest_rate_model(&self, new_interest_rate_model: &ManagedAddress) {
        self.require_admin();
        self.set_interest_rate_model_internal(new_interest_rate_model);
    }

    /// Withdraws an specified amount of underlying from the money market reserves (revenue part) to the admin account.
    ///
    /// # Arguments:
    ///
    /// - `underlying_amount` - The amount of underlying to withdraw.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The underlying amount is directed to the admin account.
    ///
    #[endpoint(reduceReserves)]
    fn reduce_reserves(&self, opt_underlying_amount: OptionalValue<BigUint>) {
        self.require_admin();

        self.accrue_interest();
        self.require_market_fresh();

        let revenue = self.revenue().get();
        let underlying_amount = opt_underlying_amount.into_option().unwrap_or_else(|| revenue.clone());

        require!(underlying_amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);

        self.try_ensure_staking_rewards(&underlying_amount);

        require!(underlying_amount <= revenue, ERROR_AMOUNT_EXCEEDS_REVENUE);

        self.total_reserves().update(|amount| *amount -= &underlying_amount);
        self.revenue().update(|amount| *amount -= &underlying_amount);
        self.cash().update(|amount| *amount -= &underlying_amount);

        let admin = self.get_admin();
        let underlying_id = self.underlying_id().get();
        let new_total_reserves = self.total_reserves().get();

        self.send().direct(&admin, &underlying_id, 0, &underlying_amount);

        self.emit_updated_rates();
        self.reserves_reduced_event(&admin, &underlying_amount, &new_total_reserves);
    }

    /// Sets a new accrual time threshold.
    ///
    /// # Arguments:
    ///
    /// - `new_accrual_time_threshold` - The new accrual time threshold in seconds.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setAccrualTimeThreshold)]
    fn set_accrual_time_threshold(&self, new_accrual_time_threshold: u64) {
        self.require_admin();

        require!(new_accrual_time_threshold <= MAX_ACCRUAL_TIME_THRESHOLD, ERROR_ACCRUAL_TIME_THRESHOLD_TOO_HIGH);

        self.accrue_interest();
        self.require_market_fresh();

        let old_accrual_time_threshold = self.accrual_time_threshold().get();
        self.accrual_time_threshold().set(new_accrual_time_threshold);

        self.set_accrual_time_threshold_event(old_accrual_time_threshold, new_accrual_time_threshold);
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
        require!(self.is_trusted_minter_sc(&trusted_minter), ERROR_NON_VALID_TRUSTED_MINTER_SC);
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
