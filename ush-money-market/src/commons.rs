multiversx_sc::imports!();

use super::{
    constants::*,
    errors::*,
    events, proxies,
    storage::{self, AccountSnapshot, DiscountStrategy, InteractionType, State},
};

use discount_rate_model::models::ExchangeRateType;

#[multiversx_sc::module]
pub trait CommonsModule: events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    // Checks

    /// A utility function to highlight that this smart contract implements the Money Market api that Controller requires.
    /// This function has been added such that this smart contract can be used as a market on the Lending Protocol.
    ///
    #[view(isMoneyMarket)]
    fn is_money_market(&self) -> bool {
        true
    }

    /// A utility function to highlight that this smart contract implements the USH Market Observer api that Controller
    /// requires. This function has been added such that this smart contract can be used as a market observer on the Lending
    /// Protocol.
    ///
    #[view(isUshMarket)]
    fn is_ush_market(&self) -> bool {
        true
    }

    /// Checks whether the current state of the smart contract is active.
    ///
    #[view(isActive)]
    fn is_active(&self) -> bool {
        self.state().get() == State::Active
    }

    /// Checks whether the current state of the smart contract is finalized.
    ///
    #[view(isFinalized)]
    fn is_finalized(&self) -> bool {
        self.state().get() == State::Finalized
    }

    /// Checks whether the Hatom token has been already issued.
    ///
    #[view(isHushIssued)]
    fn is_hush_issued(&self) -> bool {
        !self.hush_id().is_empty()
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

    /// Checks whether the specified smart contract address is the USH minter module.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_ush_minter_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_ush_minter(sc_address)
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

    /// Checks whether the specified smart contract address is an discount rate model.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_discount_rate_model_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_discount_rate_model(sc_address)
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

    /// Checks if the borrow rate change is allowed or not.
    ///
    /// # Arguments:
    ///
    /// - `from` - The current borrow rate.
    /// - `to` - The new borrow rate.
    ///
    fn is_borrow_rate_change_allowed(&self, from: &BigUint, to: &BigUint) -> bool {
        let max_borrow_rate_change = from * &BigUint::from(MAX_BORROW_RATE_CHANGE) / BigUint::from(BPS);
        let delta_borrow_rate = if from < to { to - from } else { from - to };
        delta_borrow_rate <= max_borrow_rate_change
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
        require!(self.state().get() == State::Active, ERROR_MARKET_SHOULD_BE_ACTIVE);
    }

    /// Requires that the money market state is inactive.
    ///
    fn require_inactive(&self) {
        require!(self.state().get() == State::Inactive, ERROR_MARKET_SHOULD_BE_INACTIVE);
    }

    /// Requires that the current state of the smart contract is not finalized.
    ///
    #[inline]
    fn require_not_finalized_state(&self) {
        require!(!self.is_finalized(), ERROR_MARKET_HAS_FINALIZED_STATE);
    }

    /// Requires a valid USH payment.
    ///
    fn require_valid_ush_payment(&self, ush_id: &TokenIdentifier, ush_amount: &BigUint) {
        require!(ush_id == &self.ush_id().get(), ERROR_INVALID_USH_PAYMENT);
        require!(ush_amount > &BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);
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

    /// Requires that Hatom USH has been issued.
    ///
    fn require_hush_issued(&self) {
        require!(self.is_hush_issued(), ERROR_ISSUE_HATOM_USH_FIRST);
    }

    /// Requires that Hatom USH has not been issued yet.
    ///
    fn require_hush_not_issued(&self) {
        require!(!self.is_hush_issued(), ERROR_HATOM_USH_ALREADY_ISSUED);
    }

    /// Requires USH to be eligible as collateral.
    ///
    fn require_eligible_as_collateral(&self) {
        require!(self.eligible_as_collateral().get(), ERROR_HATOM_USH_NOT_ELIGIBLE_AS_COLLATERAL);
    }

    /// Requires that the caller is the Controller smart contract.
    ///
    fn require_controller(&self) {
        let caller = self.blockchain().get_caller();
        require!(caller == self.controller().get(), ERROR_CALLER_MUST_BE_CONTROLLER_SC);
    }

    /// Requires that the caller is the Staking smart contract.
    ///
    fn require_staking_sc(&self) {
        require!(!self.staking_sc().is_empty(), ERROR_UNDEFINED_STAKING_SC);
        let caller = self.blockchain().get_caller();
        require!(caller == self.staking_sc().get(), ERROR_CALLER_MUST_BE_STAKING_SC);
    }

    // Utility

    /// Takes a numerator and denominator and returns the smallest integer greater than or equal to the quotient.
    ///
    fn ceil_div(&self, num: BigUint, den: BigUint) -> BigUint {
        require!(den > BigUint::zero(), ERROR_DIVISION_BY_ZERO);
        return (num + (&den - 1u64)) / den;
    }

    // Accrue Interest

    /// This method is one of the most important methods of the protocol, as it accrues the borrows interest and distributes
    /// that amount into reserves (including revenue and staking rewards). In order to do that, it solves the money market
    /// dynamics using an Euler scheme.
    ///
    #[endpoint(accrueInterest)]
    fn accrue_interest(&self) {
        let wad = BigUint::from(WAD);

        let t_prev = self.accrual_timestamp().get();
        let t = self.blockchain().get_block_timestamp();

        // do nothing
        if t == t_prev {
            return ();
        }

        let borrow_rate = self.borrow_rate().get();
        let effective_borrows = self.effective_borrows().get();

        let dt = t - t_prev;
        let borrow_rate_dt = borrow_rate * dt;
        let delta_borrows = &borrow_rate_dt * &effective_borrows / &wad;

        let mut total_borrows = self.total_borrows().get();
        total_borrows += &delta_borrows;
        self.total_borrows().set(&total_borrows);

        self.effective_borrows().update(|amount| *amount += &delta_borrows);

        let mut borrow_index = self.get_borrow_index();
        borrow_index += self.ceil_div(&borrow_index * &borrow_rate_dt, wad.clone());
        self.borrow_index().set(&borrow_index);

        // interest goes to the reserves
        self.total_reserves().update(|amount| *amount += &delta_borrows);

        // reserves are divided into staking rewards and revenue
        let fs = self.stake_factor().get();
        let delta_rewards = fs * &delta_borrows / &wad;
        let delta_revenue = &delta_borrows - &delta_rewards;

        self.revenue().update(|amount| *amount += &delta_revenue);
        self.staking_rewards().update(|amount| *amount += &delta_rewards);
        self.historical_staking_rewards().update(|amount| *amount += &delta_rewards);

        // update accrual timestamp
        self.accrual_timestamp().set(t);

        self.accrue_interest_event(&delta_borrows, &borrow_index, &total_borrows);
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

    // Updates

    /// Tries to remove a borrower from the market borrowers list if the borrower has no outstanding borrow.
    ///
    fn try_remove_market_borrower(&self, account: &ManagedAddress) {
        let principal_amount = self.account_principal(account).get();
        if principal_amount == BigUint::zero() {
            self.market_borrowers().swap_remove(account);
        }
    }

    // Reserves

    /// Adds an specified amount of USH coming as a payment to the USH money market reserves.
    ///
    /// Notes:
    ///
    /// - The USH amount is added as protocol revenue.
    /// - Must be paid with USH.
    ///
    #[payable("*")]
    #[endpoint(addReserves)]
    fn add_reserves(&self) {
        let (ush_id, ush_amount) = self.call_value().single_fungible_esdt();
        self.require_valid_ush_payment(&ush_id, &ush_amount);

        let donor = self.blockchain().get_caller();

        self.accrue_interest();
        self.require_market_fresh();

        // update reserves and revenue
        self.total_reserves().update(|amount| *amount += &ush_amount);
        self.revenue().update(|amount| *amount += &ush_amount);

        // burn donated USH as it will be minted again when the revenue is withdrawn
        let ush_payment = EsdtTokenPayment::new(ush_id, 0, ush_amount.clone());
        self.ush_minter_burn(&ush_payment);

        self.reserves_added_event(&donor, &ush_amount);
    }

    // Conversions

    /// Translates a USH amount to HUSH tokens.
    ///
    /// # Arguments:
    ///
    /// - `ush_amount` - the amount of USH to be converted to HUSH.
    ///
    #[view(ushToHush)]
    fn ush_to_hush(&self, ush_amount: &BigUint) -> BigUint {
        let wad = BigUint::from(WAD);
        let fx = self.get_exchange_rate();
        let tokens = ush_amount * &wad / fx;
        tokens
    }

    /// Translates HUSH tokens to USH amount.
    ///
    /// # Arguments:
    ///
    /// - `tokens` - the amount of HUSH to be converted to USH.
    ///
    #[view(hushToUsh)]
    fn hush_to_ush(&self, tokens: &BigUint) -> BigUint {
        let wad = BigUint::from(WAD);
        let fx = self.get_exchange_rate();
        let underlying_amount = fx * tokens / wad;
        underlying_amount
    }

    // Sets

    /// Sets the Controller smart contract address.
    ///
    /// # Arguments:
    ///
    /// - `controller` - The Controller smart contract address.
    ///
    fn set_controller(&self, controller: &ManagedAddress) {
        require!(self.is_controller_sc(controller), ERROR_INVALID_CONTROLLER_SC);
        self.controller().set(controller);
        self.set_controller_event(controller);
    }

    /// Sets the USH minter smart contract address.
    ///
    /// # Arguments:
    ///
    /// - `ush_minter` - The USH minter smart contract address.
    ///
    fn set_ush_minter(&self, ush_minter: &ManagedAddress) {
        require!(self.is_ush_minter_sc(ush_minter), ERROR_INVALID_USH_MINTER_SC);

        let ush_id = self.get_ush_id(ush_minter);

        self.ush_id().set(&ush_id);
        self.ush_minter().set(ush_minter);

        self.set_ush_minter_event(ush_minter, &ush_id);
    }

    /// Sets the accrual timestamp.
    ///
    fn set_accrual_timestamp(&self) {
        let timestamp = self.blockchain().get_block_timestamp();
        self.accrual_timestamp().set(timestamp);
        self.set_accrual_timestamp_event(timestamp);
    }

    /// Sets the USH market state.
    ///
    /// # Arguments:
    ///
    /// - `ush_market_state` - The USH market state.
    ///
    fn set_ush_market_state_internal(&self, ush_market_state: State) {
        require!(ush_market_state != State::Empty, ERROR_INVALID_MARKET_STATE);
        self.state().set(&ush_market_state);
        self.set_market_state_event(ush_market_state);
    }

    /// Sets the account borrow snapshot for a given borrower, which includes the borrow amount and the borrow index at the
    /// time of the snapshot.
    ///
    fn set_account_borrow_snapshot(&self, borrower: &ManagedAddress, borrow_amount: &BigUint, borrow_index: &BigUint, discount: &BigUint) {
        let account_snapshot = AccountSnapshot::new(borrow_amount, borrow_index, discount);
        self.account_borrow_snapshot(borrower).set(&account_snapshot);
    }

    // Gets

    /// Returns the money market identifiers, i.e. the underlying identifier and the token identifier as a tuple.
    ///
    #[view(getMoneyMarketIdentifiers)]
    fn get_money_market_identifiers(&self) -> (EgldOrEsdtTokenIdentifier, TokenIdentifier) {
        self.require_hush_issued();
        let ush_id = self.ush_id().get();
        let hush_id = self.hush_id().get();
        (EgldOrEsdtTokenIdentifier::esdt(ush_id), hush_id)
    }

    /// Returns the updated amount of borrows.
    ///
    #[endpoint(getCurrentTotalBorrows)]
    fn current_total_borrows(&self) -> BigUint {
        self.accrue_interest();
        self.total_borrows().get()
    }

    /// Returns the total principal such that it can be used as a base to calculate amounts that depend on the borrows
    /// amounts, such as user rewards.
    ///
    #[view(getBaseTotalBorrows)]
    fn get_base_total_borrows(&self) -> BigUint {
        self.total_principal().get()
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

    /// Returns the amount of liquidity, which in this market equals the amount of HUSH in circulation, i.e. the USH being
    /// used as collateral.
    ///
    #[view(getLiquidity)]
    fn get_liquidity(&self) -> BigUint {
        let total_supply = self.total_supply().get();
        self.hush_to_ush(&total_supply)
    }

    /// Returns a fixed reserve factor fixed to 100%. This function is used by the Controller to verify if a money market is
    /// deprecated or not.
    ///
    #[view(getReserveFactor)]
    fn get_reserve_factor(&self) -> BigUint {
        BigUint::from(WAD)
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

    /// Returns the account principal such that it can be used to calculate amounts that depend on the borrows amounts, such
    /// as user rewards.
    ///
    #[view(getBaseAccountBorrowAmount)]
    fn base_account_borrow_amount(&self, account: &ManagedAddress) -> BigUint {
        self.account_principal(account).get()
    }

    /// Returns the account borrow using the market borrow index and the account snapshot up to the last interaction that
    /// accrued interest.
    ///
    fn get_account_borrow_amount(&self, borrower: &ManagedAddress) -> BigUint {
        match self.get_account_borrow_snapshot(borrower) {
            None => BigUint::zero(),
            Some(snapshot) => {
                let wad = BigUint::from(WAD);
                let market_index = self.get_borrow_index();
                let AccountSnapshot { borrow_amount: borrow_prev, borrow_index: account_index, discount, .. } = snapshot;
                let borrow = borrow_prev * (market_index * (&wad - &discount) / account_index + discount) / wad;
                borrow
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

    /// Returns the market exchange rate (fixed to one) and the borrow amount of the given account up to the last interaction
    /// that accrued interest, in one shot.
    ///
    /// # Arguments:
    ///
    /// - `account` - The account's address.
    ///
    #[view(getAccountSnapshot)]
    fn get_account_snapshot(&self, account: &ManagedAddress) -> (BigUint, BigUint) {
        let borrow_amount = self.get_account_borrow_amount(account);
        let fx = self.get_exchange_rate();
        (borrow_amount, fx)
    }

    /// Returns the market exchange rate (fixed to one) and the borrow amount of the given account up to the last interaction
    /// that accrued interest or up to the current time if a sufficient amount of time has elapsed since the last accrual, in
    /// one shot.
    ///
    /// # Arguments:
    ///
    /// - `account` - The account's address.
    ///
    #[endpoint(getReliableAccountSnapshot)]
    fn get_reliable_account_snapshot(&self, account: &ManagedAddress) -> (BigUint, BigUint) {
        let borrow_amount = self.reliable_account_borrow_amount(account);
        let fx = self.get_exchange_rate();
        (borrow_amount, fx)
    }

    /// Returns the borrow index of the market up to the last interaction that accrued interest or its initial condition.
    /// Notice that the borrow index is a mechanism that allows updating all account borrows without having to loop into each
    /// account when there is an interaction with the protocol that accrues interests.
    ///
    #[view(getBorrowIndex)]
    fn get_borrow_index(&self) -> BigUint {
        if self.borrow_index().is_empty() {
            BigUint::from(WAD)
        } else {
            self.borrow_index().get()
        }
    }

    /// Returns the exchange rate between underlying and tokens (collateral). Since USH will be used as collateral, the
    /// exchange rate is fixed to one.
    ///
    #[view(getStoredExchangeRate)]
    fn get_stored_exchange_rate(&self) -> BigUint {
        self.get_exchange_rate()
    }

    /// Returns a fixed exchange rate to one.
    ///
    #[view(getExchangeRate)]
    fn get_exchange_rate(&self) -> BigUint {
        BigUint::from(EXCHANGE_RATE)
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

    /// Updates the account discount rate.
    ///
    /// # Arguments:
    ///
    /// - `opt_account` - The account's address. If not provided, the caller is used.
    ///
    #[endpoint(updateAccountDiscountRate)]
    fn update_account_discount_rate(&self, opt_account: OptionalValue<ManagedAddress>) {
        self.accrue_interest();
        let account = opt_account.into_option().unwrap_or_else(|| self.blockchain().get_caller());
        require!(self.market_borrowers().contains(&account), ERROR_ACCOUNT_NOT_BORROWER);
        self.update_borrows_data(&account, &BigUint::zero(), InteractionType::EnterOrExitMarket, DiscountStrategy::UpdatedExchangeRate);
    }

    /// Updates account and protocol borrow data given a borrower, an amount of USH and the interaction type.
    ///
    fn update_borrows_data(&self, borrower: &ManagedAddress, ush_amount: &BigUint, interaction_type: InteractionType, discount_strategy: DiscountStrategy) -> (BigUint, BigUint, BigUint) {
        let wad = BigUint::from(WAD);

        // get current market index
        let market_index = self.get_borrow_index();

        // compute account borrow amounts
        let opt_snapshot = self.get_account_borrow_snapshot(borrower);
        let (account_index, current_borrow, old_borrow, old_discount) = match opt_snapshot {
            Some(snapshot) => {
                let AccountSnapshot { borrow_amount: old_borrow, borrow_index: account_index, discount: old_discount } = snapshot;
                let current_borrow = &old_borrow * &(&market_index * &(&wad - &old_discount) / &account_index + &old_discount) / &wad;
                (account_index, current_borrow, old_borrow, old_discount)
            },
            None => (market_index.clone(), BigUint::zero(), BigUint::zero(), BigUint::zero()),
        };

        let (ush_effective_amount, new_borrow, total_borrows) = match interaction_type {
            InteractionType::Borrow => {
                // compute new account borrow
                let new_borrow = current_borrow + ush_amount;

                // update total borrows
                let mut total_borrows = self.total_borrows().get();
                total_borrows += ush_amount;

                (ush_amount.clone(), new_borrow, total_borrows)
            },
            InteractionType::RepayBorrow => {
                // Because of truncation errors, it might happen that the total borrows is smaller than the account borrows:
                // maybe all other borrowers have a really small amount of borrow or there are no other borrowers. In this
                // case, we make the account borrows equal the total borrows. All borrowers left will be able to pay their
                // borrows but without actually having to pay anything.
                let current_total_borrows = self.total_borrows().get();
                let current_borrow = BigUint::min(current_total_borrows, current_borrow);

                // what is being actually repaid
                let ush_repayment_amount = BigUint::min(current_borrow.clone(), ush_amount.clone());

                // compute new account borrow
                let new_borrow = &current_borrow - &ush_repayment_amount;

                // compute total borrows
                let mut total_borrows = self.total_borrows().get();
                total_borrows -= &ush_repayment_amount;

                (ush_repayment_amount, new_borrow, total_borrows)
            },
            InteractionType::EnterOrExitMarket => {
                require!(ush_amount == &BigUint::zero(), ERROR_AMOUNT_MUST_BE_ZERO);

                // compute new account borrow
                let new_borrow = current_borrow;

                // get total borrows
                let total_borrows = self.total_borrows().get();

                (BigUint::zero(), new_borrow, total_borrows)
            },
        };

        // update account principal
        self.account_principal(borrower).set(&new_borrow);

        // update total principal
        if new_borrow >= old_borrow {
            let delta_borrow = &new_borrow - &old_borrow;
            self.total_principal().update(|amount| *amount += &delta_borrow);
        } else {
            let delta_borrow = &old_borrow - &new_borrow;
            self.total_principal().update(|amount| *amount -= &delta_borrow);
        }

        // compute new account discount depending on the discount strategy
        let discount = match discount_strategy {
            DiscountStrategy::PreviousDiscount => old_discount.clone(),
            DiscountStrategy::CachedExchangeRate => self.get_account_discount(borrower, &new_borrow, ExchangeRateType::Cached),
            DiscountStrategy::UpdatedExchangeRate => self.get_account_discount(borrower, &new_borrow, ExchangeRateType::Updated),
        };

        // update account borrow snapshot
        self.set_account_borrow_snapshot(borrower, &new_borrow, &market_index, &discount);

        // update total borrows
        self.total_borrows().set(&total_borrows);

        // update effective borrows: the positive contribution goes first
        let mut effective_borrows = self.effective_borrows().get();

        // positive contribution
        effective_borrows += (&wad - &discount) * &new_borrow / &wad;

        // negative contribution
        let den = account_index * &wad;
        let num = (wad - old_discount) * market_index * old_borrow;
        effective_borrows -= BigUint::min(effective_borrows.clone(), self.ceil_div(num, den));

        self.effective_borrows().set(&effective_borrows);

        (ush_effective_amount, new_borrow, total_borrows)
    }
}
