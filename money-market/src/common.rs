multiversx_sc::imports!();

use super::{constants::*, errors::*, events, proxies, storage};
use crate::storage::State;

#[multiversx_sc::module]
pub trait CommonModule: events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    // Checks

    /// A utility function to highlight that this smart contract is a Money Market.
    ///
    #[view(isMoneyMarket)]
    fn is_money_market(&self) -> bool {
        true
    }

    /// Checks whether the Hatom token has been already issued.
    ///
    #[view(isTokenIssued)]
    fn is_token_issued(&self) -> bool {
        !self.token_id().is_empty()
    }

    /// Checks whether the specified smart contract address is a controller.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_controller_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_controller(sc_address)
    }

    /// Checks whether the specified smart contract address is a staking module.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_staking_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_staking(sc_address)
    }

    /// Checks whether the specified smart contract address is an interest rate model.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_interest_rate_model_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_interest_rate_model(sc_address)
    }

    /// Checks whether the specified smart contract address is a trusted minter.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_trusted_minter_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_trusted_minter(sc_address)
    }

    // Requires

    /// Requires that the money market has already accrued interest.
    ///
    fn require_market_fresh(&self) {
        require!(self.blockchain().get_block_timestamp() == self.accrual_timestamp().get(), ERROR_MARKET_NOT_FRESH);
    }

    /// Requires that the money market is already active.
    ///
    fn require_active(&self) {
        require!(self.market_state().get() == State::Active, ERROR_MARKET_SHOULD_BE_ACTIVE);
    }

    /// Requires that the money market state is inactive.
    ///
    fn require_inactive(&self) {
        require!(self.market_state().get() == State::Inactive, ERROR_MARKET_SHOULD_BE_INACTIVE);
    }

    /// Requires a valid underlying payment.
    ///
    fn require_valid_underlying_payment(&self, underlying_id: &EgldOrEsdtTokenIdentifier, underlying_amount: &BigUint) {
        require!(underlying_id == &self.underlying_id().get(), ERROR_INVALID_UNDERLYING_PAYMENT);
        require!(underlying_amount > &BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);
    }

    /// Requires the Hatom token to be issued.
    ///
    fn require_token_issued(&self) {
        require!(self.is_token_issued(), ERROR_ISSUE_HATOM_TOKEN_FIRST);
    }

    /// Requires that the specified address is a trusted minter contract.
    ///
    fn require_trusted_minter(&self, sc_address: &ManagedAddress) {
        require!(self.trusted_minters_list().contains(sc_address), ERROR_NOT_A_TRUSTED_MINTER);
    }

    /// Requires that the specified address is not a trusted minter contract.
    ///
    fn require_not_trusted_minter(&self, sc_address: &ManagedAddress) {
        require!(!self.trusted_minters_list().contains(sc_address), ERROR_ALREADY_TRUSTED_MINTER);
    }

    /// Makes a requirement that tries to ensure that staking rewards are available if a specified amount of underlying is
    /// withdrawn from the money market by a borrow, redeem or reduce of reserves. However, it tries but cannot 100%
    /// guarantee that staking rewards will be available at a following interaction with the protocol.
    ///
    /// # Arguments:
    ///
    /// - `underlying_amount` - The amount of underlying that will be withdrawn from the protocol.
    ///
    fn try_ensure_staking_rewards(&self, underlying_amount: &BigUint) {
        let cash = self.cash().get();
        let staking_rewards = self.staking_rewards().get();
        require!(cash >= staking_rewards && *underlying_amount <= cash - staking_rewards, ERROR_INSUFFICIENT_BALANCE);
    }

    // Accrue Interest

    /// This method is one of the most important methods of the protocol, as it accrues the borrows interest and distributes
    /// that amount into reserves (including revenue and staking rewards). In order to do that, it solves the money market
    /// dynamics using an Euler scheme.
    ///
    #[endpoint(accrueInterest)]
    fn accrue_interest(&self) {
        let wad = BigUint::from(WAD);

        let t = self.blockchain().get_block_timestamp();
        let t_prev = self.accrual_timestamp().get();

        // no need to update, zero interest accumulated
        if t == t_prev {
            return ();
        }

        // get borrow rate from interest rate model
        let cash_prev = self.cash().get();
        let borrows_prev = self.total_borrows().get();
        let reserves_prev = self.total_reserves().get();
        let liquidity_prev = self.get_liquidity();
        let borrow_rate_prev = self.get_borrow_rate(&borrows_prev, &liquidity_prev);
        let rewards_prev = self.staking_rewards().get();
        let revenue_prev = self.revenue().get();
        let index_prev = self.get_borrow_index();

        // update total borrows
        let dt = t - t_prev;
        let borrow_rate_dt = &borrow_rate_prev * dt;
        let delta_borrows = &borrow_rate_dt * &borrows_prev / &wad;
        let new_borrows = &borrows_prev + &delta_borrows;
        self.total_borrows().set(&new_borrows);

        // a fraction of the accumulated interest go to the reserves
        let fr = self.reserve_factor().get();
        let delta_reserves = &fr * &delta_borrows / &wad;
        let new_reserves = reserves_prev + &delta_reserves;

        // but reserves are divided into staking rewards and revenue
        let fs = self.stake_factor().get();
        let delta_rewards = fs * &delta_reserves / &wad;
        let new_rewards = rewards_prev + &delta_rewards;

        let delta_revenue = &delta_reserves - &delta_rewards;
        let new_revenue = revenue_prev + delta_revenue;

        self.total_reserves().set(&new_reserves);
        self.staking_rewards().set(&new_rewards);
        self.revenue().set(&new_revenue);

        // track historical staking rewards as well
        self.historical_staking_rewards().update(|amount| *amount += &delta_rewards);

        // update borrow index
        let new_index = borrow_rate_dt * &index_prev / &wad + &index_prev;
        self.borrow_index().set(&new_index);

        // update timestamp
        self.accrual_timestamp().set(t);

        self.accrue_interest_event(&cash_prev, &delta_borrows, &new_index, &new_borrows);
    }

    /// Accrues interest if a sufficient amount of time has elapsed since the last accrual.
    ///
    #[endpoint(tryAccrueInterest)]
    fn try_accrue_interest(&self) {
        let t_prev = self.accrual_timestamp().get();
        let t = self.blockchain().get_block_timestamp();
        let accrual_time_threshold = self.accrual_time_threshold().get();
        if t - t_prev >= accrual_time_threshold {
            self.accrue_interest();
        }
    }

    // Rates

    /// Interacts with the Interest Rate Model, computes current rates and emits the updated rates event.
    ///
    fn emit_updated_rates(&self) {
        let borrows = self.total_borrows().get();
        let liquidity = self.get_liquidity();
        let reserve_factor = self.reserve_factor().get();
        let (borrow_rate, supply_rate) = self.get_rates(&borrows, &liquidity, &reserve_factor);
        self.updated_rates_event(&borrow_rate, &supply_rate)
    }

    // Reserves

    /// Adds an specified amount of underlying coming as a payment to the money market reserves.
    ///
    /// Notes:
    ///
    /// - The underlying amount is added as protocol revenue.
    /// - Must be paid with underlying.
    /// - Does not change the exchange rate.
    ///
    #[payable("*")]
    #[endpoint(addReserves)]
    fn add_reserves(&self) {
        let (underlying_id, underlying_amount) = self.call_value().egld_or_single_fungible_esdt();
        self.require_valid_underlying_payment(&underlying_id, &underlying_amount);

        self.accrue_interest();
        self.require_market_fresh();

        // update reserves, revenue and cash
        self.total_reserves().update(|amount| *amount += &underlying_amount);
        self.revenue().update(|amount| *amount += &underlying_amount);
        self.cash().update(|amount| *amount += &underlying_amount);

        let donor = self.blockchain().get_caller();
        let new_total_reserves = self.total_reserves().get();

        self.emit_updated_rates();
        self.reserves_added_event(&donor, &underlying_amount, &new_total_reserves);
    }

    // Conversions

    /// Translates an underlying amount to tokens.
    ///
    /// # Arguments:
    ///
    /// - `underlying_amount` - the amount of underlying to be converted to tokens.
    ///
    #[view(underlyingAmountToTokens)]
    fn underlying_amount_to_tokens(&self, underlying_amount: &BigUint) -> BigUint {
        let wad = BigUint::from(WAD);
        let fx = self.get_exchange_rate();
        let tokens = underlying_amount * &wad / fx;
        tokens
    }

    /// Translates tokens to an underlying amount.
    ///
    /// # Arguments:
    ///
    /// - `tokens` - the amount of tokens to be converted to underlying.
    ///
    #[view(tokensToUnderlyingAmount)]
    fn tokens_to_underlying_amount(&self, tokens: &BigUint) -> BigUint {
        let wad = BigUint::from(WAD);
        let fx = self.get_exchange_rate();
        let underlying_amount = fx * tokens / wad;
        underlying_amount
    }

    /// Translates an underlying amount to tokens using an updated exchange rate.
    ///
    /// # Arguments:
    ///
    /// - `underlying_amount` - the amount of underlying to be converted to tokens.
    ///
    #[endpoint(currentUnderlyingAmountToTokens)]
    fn current_underlying_amount_to_tokens(&self, underlying_amount: &BigUint) -> BigUint {
        self.accrue_interest();
        let tokens = self.underlying_amount_to_tokens(underlying_amount);
        tokens
    }

    /// Translates tokens to an underlying amount using an updated exchange rate.
    ///
    /// # Arguments:
    ///
    /// - `tokens` - the amount of tokens to be converted to underlying.
    ///
    #[endpoint(currentTokensToUnderlyingAmount)]
    fn current_tokens_to_underlying_amount(&self, tokens: &BigUint) -> BigUint {
        self.accrue_interest();
        let underlying_amount = self.tokens_to_underlying_amount(tokens);
        underlying_amount
    }

    // Sets

    /// Sets the underlying identifier iff not already set.
    ///
    /// # Arguments:
    ///
    /// - `underlying_id` - the underlying identifier.
    ///
    fn try_set_underlying_id(&self, underlying_id: &EgldOrEsdtTokenIdentifier) {
        require!(underlying_id.is_valid(), ERROR_INVALID_UNDERLYING_ID);
        if self.underlying_id().is_empty() {
            self.underlying_id().set(underlying_id);
            self.set_underlying_id_event(underlying_id);
        }
    }

    /// Sets the initial exchange rate iff not already set.
    ///
    /// # Arguments:
    ///
    /// - `initial_exchange_rate` - the initial exchange rate.
    ///
    fn try_set_initial_exchange_rate(&self, initial_exchange_rate: &BigUint) {
        require!(*initial_exchange_rate > BigUint::zero(), ERROR_INITIAL_FX_MUST_BE_GREATER_THAN_ZERO);
        if self.initial_exchange_rate().is_empty() {
            self.initial_exchange_rate().set(initial_exchange_rate);
            self.set_initial_exchange_rate_event(initial_exchange_rate);
        }
    }

    /// Tries to set the controller iff not already set.
    ///
    /// # Arguments:
    ///
    /// - `controller` - The address of the controller.
    ///
    fn try_set_controller(&self, controller: &ManagedAddress) {
        if self.controller().is_empty() {
            require!(self.is_controller_sc(controller), ERROR_NON_VALID_CONTROLLER_SC);
            let old_controller = self.get_controller();
            self.controller().set(controller);
            self.new_controller_event(&old_controller, controller);
        }
    }

    /// Sets the accrual timestamp iff not already set.
    ///
    fn try_set_accrual_timestamp(&self) {
        if self.accrual_timestamp().is_empty() {
            let timestamp = self.blockchain().get_block_timestamp();
            self.accrual_timestamp().set(timestamp);
            self.set_accrual_timestamp_event(timestamp);
        }
    }

    /// Tries to set the interest rate model iff not already set.
    ///
    /// # Arguments:
    ///
    /// - `interest_rate_model` - The address of the interest rate model.
    ///
    fn try_set_interest_rate_model(&self, interest_rate_model: &ManagedAddress) {
        if self.interest_rate_model().is_empty() {
            self.set_interest_rate_model_internal(interest_rate_model);
        }
    }

    fn set_interest_rate_model_internal(&self, new_interest_rate_model: &ManagedAddress) {
        require!(self.is_interest_rate_model_sc(new_interest_rate_model), ERROR_NON_VALID_INTEREST_RATE_MODEL_SC);

        // update state state
        self.accrue_interest();

        // make sure market is fresh when changing an interest rate model
        self.require_market_fresh();

        let old_interest_rate_model = self.get_interest_rate_model();
        self.interest_rate_model().set(new_interest_rate_model);

        let (r0, m1, m2, uo, r_max) = self.get_model_parameters();

        self.emit_updated_rates();
        self.new_interest_rate_model_event(&old_interest_rate_model, new_interest_rate_model, &r0, &m1, &m2, &uo, &r_max);
    }

    /// Tries to set the market state iff not already set.
    ///
    fn try_set_market_state(&self, market_state: &State) {
        if self.market_state().is_empty() {
            self.set_market_state_internal(market_state);
        }
    }

    fn set_market_state_internal(&self, new_market_state: &State) {
        require!(new_market_state != &State::Empty, ERROR_INVALID_MARKET_STATE);
        let old_market_state = self.market_state().get();
        self.market_state().set(new_market_state);
        self.set_market_state_event(&old_market_state, new_market_state);
    }

    /// Sets the account borrow snapshot for a given borrower, which includes the borrow amount and the borrow index at the
    /// time of the snapshot.
    ///
    fn set_account_borrow_snapshot(&self, borrower: &ManagedAddress, new_account_borrows: &BigUint, borrow_index: &BigUint) {
        let account_snapshot = storage::AccountSnapshot { borrow_amount: new_account_borrows.clone(), borrow_index: borrow_index.clone() };
        self.account_borrow_snapshot(borrower).set(&account_snapshot);
    }

    // Gets

    /// Returns the money market identifiers, i.e. the underlying identifier and the token identifier as a tuple.
    ///
    #[view(getMoneyMarketIdentifiers)]
    fn get_money_market_identifiers(&self) -> (EgldOrEsdtTokenIdentifier, TokenIdentifier) {
        self.require_token_issued();
        let underlying_id = self.underlying_id().get();
        let token_id = self.token_id().get();
        (underlying_id, token_id)
    }

    /// Returns the updated amount of borrows.
    ///
    #[endpoint(getCurrentTotalBorrows)]
    fn current_total_borrows(&self) -> BigUint {
        self.accrue_interest();
        self.total_borrows().get()
    }

    /// Returns the discounted total borrows to the money market inception. This can be used as a base to calculate amounts
    /// that depend on the borrows amounts, such as user rewards or discounts. Notice that it does not accrue interest.
    ///
    #[view(getBaseTotalBorrows)]
    fn get_base_total_borrows(&self) -> BigUint {
        let wad = BigUint::from(WAD);
        let total_borrows_t = self.total_borrows().get();
        let market_borrow_index = self.get_borrow_index();
        total_borrows_t * wad / market_borrow_index
    }

    /// Returns the updated amount of reserves.
    ///
    #[endpoint(getCurrentTotalReserves)]
    fn current_total_reserves(&self) -> BigUint {
        self.accrue_interest();
        self.total_reserves().get()
    }

    /// Returns the updated amount of staking rewards.
    ///
    #[endpoint(getCurrentStakingRewards)]
    fn get_current_staking_rewards(&self) -> BigUint {
        self.accrue_interest();
        self.staking_rewards().get()
    }

    /// Returns the updated amount of historical staking rewards.
    ///
    #[endpoint(getCurrentHistoricalStakingRewards)]
    fn get_current_historical_staking_rewards(&self) -> BigUint {
        self.accrue_interest();
        self.historical_staking_rewards().get()
    }

    /// Returns the updated amount of revenue.
    ///
    #[endpoint(getCurrentRevenue)]
    fn get_current_revenue(&self) -> BigUint {
        self.accrue_interest();
        self.revenue().get()
    }

    /// Returns the updated amount of liquidity. The liquidity is the cash plus the borrows minus the reserves.
    ///
    #[endpoint(getCurrentLiquidity)]
    fn get_current_liquidity(&self) -> BigUint {
        self.accrue_interest();
        self.get_liquidity()
    }

    /// Returns the amount of liquidity up to the last interaction that accrued interest.
    ///
    #[view(getLiquidity)]
    fn get_liquidity(&self) -> BigUint {
        let cash = self.cash().get();
        let borrows = self.total_borrows().get();
        let reserves = self.total_reserves().get();

        cash + borrows - reserves
    }

    /// Returns the reserve factor, i.e. the percentage of interest that is redirected to the reserves. We keep this method
    /// so that it matches with USH Money Market interface.
    ///
    #[view(getReserveFactor)]
    fn get_reserve_factor(&self) -> BigUint {
        if self.reserve_factor().is_empty() {
            BigUint::zero()
        } else {
            self.reserve_factor().get()
        }
    }

    /// Returns the address of the Interest Rate Model smart contract if set.
    ///
    #[view(getInterestRateModel)]
    fn get_interest_rate_model(&self) -> Option<ManagedAddress> {
        if self.interest_rate_model().is_empty() {
            None
        } else {
            let interest_rate_model = self.interest_rate_model().get();
            Some(interest_rate_model)
        }
    }

    /// Returns the address of the Controller smart contract if set.
    ///
    #[view(getController)]
    fn get_controller(&self) -> Option<ManagedAddress> {
        if self.controller().is_empty() {
            None
        } else {
            let controller = self.controller().get();
            Some(controller)
        }
    }

    /// Returns the address of the Staking smart contract if set.
    ///
    #[view(getStakingContract)]
    fn get_staking_contract(&self) -> Option<ManagedAddress> {
        if self.staking_contract().is_empty() {
            None
        } else {
            let staking_sc = self.staking_contract().get();
            Some(staking_sc)
        }
    }

    /// Returns the updated borrow amount of the given account.
    ///
    #[endpoint(getCurrentAccountBorrowAmount)]
    fn current_account_borrow_amount(&self, account: &ManagedAddress) -> BigUint {
        self.accrue_interest();
        self.get_account_borrow_amount(account)
    }

    /// Returns the borrow amount of the given account up to the last interaction that accrued interest or up to the current
    /// time if a sufficient amount of time has elapsed since the last accrual.
    ///
    #[endpoint(getReliableAccountBorrowAmount)]
    fn reliable_account_borrow_amount(&self, account: &ManagedAddress) -> BigUint {
        self.try_accrue_interest();
        self.get_account_borrow_amount(account)
    }

    /// Returns the borrow amount of the given account up to the last interaction that accrued interest.
    ///
    #[view(getStoredAccountBorrowAmount)]
    fn stored_account_borrow_amount(&self, account: &ManagedAddress) -> BigUint {
        self.get_account_borrow_amount(account)
    }

    /// Returns the discounted account borrows to the money market inception. This can be used to calculate amounts that
    /// depend on the borrows amounts, such as user rewards or discounts. Notice that it does not accrue interest.
    ///
    #[view(getBaseAccountBorrowAmount)]
    fn base_account_borrow_amount(&self, account: &ManagedAddress) -> BigUint {
        let wad = BigUint::from(WAD);
        let borrow_amount_t = self.get_account_borrow_amount(account);
        let market_borrow_index = self.get_borrow_index();
        borrow_amount_t * wad / market_borrow_index
    }

    /// Returns the account borrow using the market borrow index and the account snapshot up to the last interaction that
    /// accrued interest.
    ///
    fn get_account_borrow_amount(&self, borrower: &ManagedAddress) -> BigUint {
        let borrower_borrow_snapshot = self.get_account_borrow_snapshot(borrower);

        match borrower_borrow_snapshot {
            None => BigUint::zero(),
            Some(snapshot) => {
                // update underlying amount borrowed
                let market_borrow_index = self.get_borrow_index();
                let new_borrow_amount = snapshot.borrow_amount * market_borrow_index / snapshot.borrow_index;
                new_borrow_amount
            },
        }
    }

    /// Returns the account borrow snapshot, which includes the borrow amount and the borrow index updated up to the last
    /// time the user interacted with the protocol.
    ///
    fn get_account_borrow_snapshot(&self, account: &ManagedAddress) -> Option<storage::AccountSnapshot<Self::Api>> {
        if self.account_borrow_snapshot(account).is_empty() {
            None
        } else {
            let account_borrow_snapshot = self.account_borrow_snapshot(account).get();
            Some(account_borrow_snapshot)
        }
    }

    /// Returns the money market exchange rate and the borrow amount of the given account up to the last interaction that
    /// accrued interest, in one shot.
    ///
    /// # Arguments:
    ///
    /// - `account` - The account address to check.
    ///
    #[view(getAccountSnapshot)]
    fn get_account_snapshot(&self, account: &ManagedAddress) -> (BigUint, BigUint) {
        let borrow_amount = self.get_account_borrow_amount(account);
        let fx = self.get_exchange_rate();
        (borrow_amount, fx)
    }

    /// Returns the money market exchange rate and the borrow amount of the given account up to the last interaction that
    /// accrued interest or up to the current time if a sufficient amount of time has elapsed since the last accrual, in one
    /// shot.
    ///
    /// # Arguments:
    ///
    /// - `account` - The account address to check.
    ///
    #[endpoint(getReliableAccountSnapshot)]
    fn get_reliable_account_snapshot(&self, account: &ManagedAddress) -> (BigUint, BigUint) {
        let borrow_amount = self.reliable_account_borrow_amount(account);
        let fx = self.get_exchange_rate();
        (borrow_amount, fx)
    }

    /// Returns the borrow index of the money market up to the last interaction that accrued interest or its initial
    /// condition. Notice that the borrow index is a mechanism that allows updating all account borrows without having to
    /// loop into each account when there is an interaction with the protocol that accrues interests.
    ///
    #[view(getBorrowIndex)]
    fn get_borrow_index(&self) -> BigUint {
        if self.borrow_index().is_empty() {
            BigUint::from(WAD)
        } else {
            self.borrow_index().get()
        }
    }

    /// Returns the current money market exchange rate between underlying and tokens.
    ///
    #[endpoint(getCurrentExchangeRate)]
    fn get_current_exchange_rate(&self) -> BigUint {
        self.accrue_interest();
        self.get_exchange_rate()
    }

    /// Returns the money market exchange rate between underlying and tokens up to the last interaction that accrued
    /// interest.
    ///
    #[view(getStoredExchangeRate)]
    fn get_stored_exchange_rate(&self) -> BigUint {
        self.get_exchange_rate()
    }

    /// Returns the exchange rate between underlying and tokens. The exchange rate is calculated as the total liquidity in
    /// the money market divided by the total supply of tokens. When there are no tokens in circulation, the exchange rate is
    /// the initial condition.
    ///
    fn get_exchange_rate(&self) -> BigUint {
        let wad = BigUint::from(WAD);

        let total_supply = self.total_supply().get();
        if total_supply == BigUint::zero() {
            return self.initial_exchange_rate().get();
        }

        let liquidity = self.get_liquidity();

        liquidity * wad / total_supply
    }

    /// Returns the borrow rate per second up to the last interaction that accrued interest.
    ///
    #[view(getBorrowRatePerSecond)]
    fn borrow_rate_per_second(&self) -> BigUint {
        let prev_borrows = self.total_borrows().get();
        let prev_liquidity = self.get_liquidity();

        self.get_borrow_rate(&prev_borrows, &prev_liquidity)
    }

    /// Returns the supply rate per second up to the last interaction that accrued interest.
    ///
    #[view(getSupplyRatePerSecond)]
    fn supply_rate_per_second(&self) -> BigUint {
        let prev_borrows = self.total_borrows().get();
        let prev_liquidity = self.get_liquidity();
        let reserve_factor = self.reserve_factor().get();

        self.get_supply_rate(&prev_borrows, &prev_liquidity, &reserve_factor)
    }

    /// Returns the borrow rate and the supply rate per second up to the last interaction that accrued interest.
    ///
    #[view(getRatesPerSecond)]
    fn get_rates_per_second(&self) -> (BigUint, BigUint) {
        let prev_borrows = self.total_borrows().get();
        let prev_liquidity = self.get_liquidity();
        let reserve_factor = self.reserve_factor().get();

        self.get_rates(&prev_borrows, &prev_liquidity, &reserve_factor)
    }

    /// Returns the close factor, used to determine the maximum amount of a borrow that can be repaid during a liquidation.
    /// If not set, it returns the minimum allowed close factor.
    ///
    #[view(getCloseFactor)]
    fn get_close_factor(&self) -> BigUint {
        if self.close_factor().is_empty() {
            BigUint::from(MIN_CLOSE_FACTOR)
        } else {
            self.close_factor().get()
        }
    }

    /// Returns the current liquidation incentive. If not set, it returns the minimum allowed liquidation incentive, which is
    /// compliant with the default protocol seize share of 0% and the maximum collateral factor of 90%.
    ///
    #[view(getLiquidationIncentive)]
    fn get_liquidation_incentive(&self) -> BigUint {
        if self.liquidation_incentive().is_empty() {
            BigUint::from(MIN_LIQUIDATION_INCENTIVE)
        } else {
            self.liquidation_incentive().get()
        }
    }
}
