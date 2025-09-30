multiversx_sc::imports!();

use super::{constants::*, errors::*, events, proxies, storage};

use crate::storage::{Status, SwapOperationType, SwapStep, SWAP_TOKENS_FIXED_INPUT_FUNC_NAME};

#[multiversx_sc::module]
pub trait SharedModule: admin::AdminModule + events::EventModule + proxies::ProxyModule + storage::StorageModule {
    // Checks

    /// A utility function to highlight that this smart contract is a Controller.
    ///
    #[view(isController)]
    fn is_controller(&self) -> bool {
        true
    }

    /// Checks whether the specified smart contract address is a money market.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_money_market_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_money_market(sc_address)
    }

    /// Checks whether the specified money market address has already been whitelisted.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the money market to check.
    ///
    #[view(isWhitelistedMoneyMarket)]
    fn is_whitelisted_money_market(&self, sc_address: &ManagedAddress) -> bool {
        self.whitelisted_markets().contains(sc_address)
    }

    /// Checks whether the specified token identifier has already been whitelisted.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier to check.
    ///
    #[view(isWhitelistedTokenId)]
    fn is_whitelisted_token_id(&self, token_id: &TokenIdentifier) -> bool {
        !self.money_markets(token_id).is_empty()
    }

    /// Checks whether the specified address is a Rewards Booster observer.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the market observer to check.
    ///
    #[view(isBoosterObserver)]
    fn is_booster_observer(&self, sc_address: &ManagedAddress) -> bool {
        self.booster_observer().get() == *sc_address
    }

    /// Checks whether the specified smart contract address is a rewards booster.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_rewards_booster_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_rewards_booster(sc_address)
    }

    /// Checks whether the specified address is a USH Market observer.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the market observer to check.
    ///
    #[view(isUshMarketObserver)]
    fn is_ush_market_observer(&self, sc_address: &ManagedAddress) -> bool {
        self.ush_market_observer().get() == *sc_address
    }

    /// Checks whether the specified smart contract address is a USH money market.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_ush_market_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_ush_market(sc_address)
    }

    /// Checks whether the specified smart contract address is a price oracle.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn is_price_oracle_sc(&self, sc_address: &ManagedAddress) -> bool {
        self.blockchain().is_smart_contract(sc_address) && self.is_price_oracle(sc_address)
    }

    /// Checks whether the specified money market is deprecated.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market to check.
    ///
    #[endpoint(isDeprecated)]
    fn is_deprecated(&self, money_market: &ManagedAddress) -> bool {
        let b0 = self.update_and_get_collateral_factor(money_market) == BigUint::zero();
        let b1 = self.get_borrow_status(money_market) == Status::Paused;
        let b2 = self.get_reserve_factor(money_market) == BigUint::from(WAD);
        b0 && b1 && b2
    }

    /// Checks whether the specified money market contains a rewards batch for a given rewards token.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market.
    /// - `rewards_token_id` - The ID of the rewards token.
    ///
    fn market_has_token_rewards_batch(&self, money_market: &ManagedAddress, rewards_token_id: &EgldOrEsdtTokenIdentifier) -> bool {
        let market_rewards_batches = self.rewards_batches(money_market);
        market_rewards_batches.iter().any(|batch| batch.token_id == *rewards_token_id)
    }

    /// Checks there exists at least one active rewards batch for a given rewards token.
    ///
    /// # Arguments:
    ///
    /// - `rewards_token_id` - The ID of the rewards token.
    ///
    fn token_has_active_rewards_batch(&self, rewards_token_id: &EgldOrEsdtTokenIdentifier) -> bool {
        let money_markets = self.get_whitelisted_markets();
        let current_timestamp = self.blockchain().get_block_timestamp();
        for money_market in money_markets.iter() {
            let market_rewards_batches = self.rewards_batches(&money_market);
            let active_rewards_batch = market_rewards_batches.iter().any(|batch| batch.token_id == *rewards_token_id && current_timestamp < batch.end_time);
            if active_rewards_batch {
                return true;
            }
        }
        false
    }

    // Requires

    /// Requires that the given smart contract address is a whitelisted money market.
    ///
    /// # Arguments:
    ///
    /// - `sc_address` - The address of the smart contract to check.
    ///
    fn require_whitelisted_money_market(&self, sc_address: &ManagedAddress) {
        require!(self.is_whitelisted_money_market(sc_address), ERROR_NON_WHITELISTED_MARKET);
    }

    /// Requires that the caller is the admin or the pause guardian, if it is set.
    ///
    fn require_admin_or_guardian(&self) {
        let admin = self.get_admin();
        let caller = self.blockchain().get_caller();

        match self.get_pause_guardian() {
            None => {
                require!(caller == admin, ERROR_ONLY_ADMIN);
            },
            Some(pause_guardian) => {
                require!(caller == admin || caller == pause_guardian, ERROR_ONLY_ADMIN_OR_GUARDIAN);
            },
        }
    }

    /// Requires that the caller is the admin or the rewards manager, if it is set.
    ///
    fn require_admin_or_rewards_manager(&self) {
        let admin = self.get_admin();
        let caller = self.blockchain().get_caller();

        match self.get_rewards_manager() {
            None => {
                require!(caller == admin, ERROR_ONLY_ADMIN);
            },
            Some(rewards_manager) => {
                require!(caller == admin || caller == rewards_manager, ERROR_ONLY_ADMIN_OR_REWARDS_MANAGER);
            },
        }
    }

    /// Requires a valid collateral factor decrease.
    ///
    /// # Arguments:
    ///
    /// - `new_ltv` - The new collateral factor.
    /// - `old_ltv` - The old collateral factor.
    ///
    fn require_valid_collateral_factor_decrease(&self, new_ltv: &BigUint, old_ltv: &BigUint) {
        if new_ltv >= old_ltv {
            return;
        }
        let max_allowed_decrease = BigUint::min(MAX_COLLATERAL_FACTOR_DECREASE.into(), old_ltv.clone());
        let min_allowed_ltv = old_ltv - &max_allowed_decrease;
        require!(new_ltv >= &min_allowed_ltv, ERROR_EXCEEDED_MAXIMUM_DECREASE);
    }

    // Gets

    /// Gets a whitelist or set of supported money market addresses as an array.
    ///
    #[view(getWhitelistedMarkets)]
    fn get_whitelisted_markets(&self) -> ManagedVec<ManagedAddress> {
        self.whitelisted_markets().iter().collect()
    }

    /// Gets the the set of money markets addresses in which the account has entered as an array. An account is considered to
    /// be in the market if it has deposited collateral or took a borrow. Currently, after a borrow is fully repaid, the
    /// account is still considered to be in the market.
    ///
    #[view(getAccountMarkets)]
    fn get_account_markets(&self, account: &ManagedAddress) -> ManagedVec<ManagedAddress> {
        self.account_markets(account).iter().collect()
    }

    /// Gets the maximum number of money markets that can be entered per account.
    ///
    fn get_max_markets_per_account(&self) -> usize {
        if self.max_markets_per_account().is_empty() {
            0usize
        } else {
            self.max_markets_per_account().get()
        }
    }

    /// Returns all whitelisted money markets if the provided money markets are empty. Otherwise, it returns the provided
    /// money markets.
    ///
    fn validate_money_markets(&self, money_markets: ManagedVec<ManagedAddress>) -> ManagedVec<ManagedAddress> {
        if money_markets.is_empty() {
            return self.get_whitelisted_markets();
        }

        for market in money_markets.iter() {
            self.require_whitelisted_money_market(&market);
        }
        money_markets
    }

    /// Returns the next rewards batch ID for a given money market and updates it.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    /// # Notes:
    ///
    /// - The counter starts at 1.
    ///
    fn get_next_rewards_batch_id(&self, money_market: &ManagedAddress) -> usize {
        let rewards_batch_id = self.next_rewards_batch_id(money_market).update(|id| {
            *id += 1usize;
            *id
        });

        rewards_batch_id
    }

    /// Gets the maximum collateral factor allowed
    ///
    #[view(getMaxCollateralFactor)]
    fn get_max_collateral_factor(&self) -> BigUint {
        BigUint::from(MAX_COLLATERAL_FACTOR)
    }

    /// Gets the amount of Hatom tokens deposited as collateral for a given money market and account.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `account` - The account we wish to analyze.
    ///
    #[view(getAccountTokens)]
    fn get_account_collateral_tokens(&self, money_market: &ManagedAddress, account: &ManagedAddress) -> BigUint {
        let mapper = self.account_collateral_tokens(money_market, account);
        if mapper.is_empty() {
            BigUint::zero()
        } else {
            mapper.get()
        }
    }

    /// Gets the total amount of collateral tokens deposited into the controller for a specific money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market for which to retrieve the total collateral tokens.
    ///
    /// # Notes:
    ///
    /// - If the market has no collateral, returns 0.
    ///
    #[view(getTotalCollateralTokens)]
    fn get_total_collateral_tokens(&self, money_market: &ManagedAddress) -> BigUint {
        self.total_collateral_tokens(money_market).get()
    }

    /// Gets the up to date collateral factor for a specified money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    #[endpoint(updateAndGetCollateralFactor)]
    fn update_and_get_collateral_factor(&self, money_market: &ManagedAddress) -> BigUint {
        let (cf, _) = self.update_and_get_collateral_factors(money_market);
        cf
    }

    /// Gets the up to date USH borrower collateral factor for a specified money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    #[endpoint(updateAndGetUshBorrowerCollateralFactor)]
    fn update_and_get_ush_borrower_collateral_factor(&self, money_market: &ManagedAddress) -> BigUint {
        let (_, uf) = self.update_and_get_collateral_factors(money_market);
        uf
    }

    /// Updates the collateral factors if possible and returns their updated values.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    #[endpoint(updateAndGetCollateralFactors)]
    fn update_and_get_collateral_factors(&self, money_market: &ManagedAddress) -> (BigUint, BigUint) {
        let cf = self.collateral_factor(money_market).get();
        let uf = self.ush_borrower_collateral_factor(money_market).get();

        if self.next_collateral_factors(money_market).is_empty() {
            return (cf, uf);
        }

        let current_timestamp = self.blockchain().get_block_timestamp();
        let (start_timestamp, next_cf, next_uf) = self.next_collateral_factors(money_market).get();

        if current_timestamp < start_timestamp {
            return (cf, uf);
        }

        self.next_collateral_factors(money_market).clear();
        self.collateral_factor(money_market).set(&next_cf);
        self.ush_borrower_collateral_factor(money_market).set(&next_uf);

        self.clear_next_collateral_factors_event();
        self.new_collateral_factor_event(money_market, &cf, &next_cf);
        self.new_ush_borrower_collateral_factor_event(money_market, &uf, &next_uf);

        (next_cf, next_uf)
    }

    /// Gets the current liquidity cap for a given money market, if there is one.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    fn get_liquidity_cap(&self, money_market: &ManagedAddress) -> Option<BigUint> {
        let mapper = self.liquidity_cap(money_market);
        if mapper.is_empty() {
            None
        } else {
            let liquidity_cap = mapper.get();
            Some(liquidity_cap)
        }
    }

    /// Gets the current borrow cap for a given money market, if there is one.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    fn get_borrow_cap(&self, money_market: &ManagedAddress) -> Option<BigUint> {
        let mapper = self.borrow_cap(money_market);
        if mapper.is_empty() {
            None
        } else {
            let borrow_cap = mapper.get();
            Some(borrow_cap)
        }
    }

    /// Gets the address of the pause guardian, if one has been set.
    ///
    fn get_pause_guardian(&self) -> Option<ManagedAddress> {
        if self.pause_guardian().is_empty() {
            None
        } else {
            let pause_guardian = self.pause_guardian().get();
            Some(pause_guardian)
        }
    }

    /// Gets the address of the rewards manager, if one has been set.
    ///
    fn get_rewards_manager(&self) -> Option<ManagedAddress> {
        if self.rewards_manager().is_empty() {
            None
        } else {
            let rewards_manager = self.rewards_manager().get();
            Some(rewards_manager)
        }
    }

    /// Gets the current minting status at a given money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    /// # Notes:
    ///
    /// - By default, mint is active (returns the first enum value).
    ///
    #[view(getMintStatus)]
    fn get_mint_status(&self, money_market: &ManagedAddress) -> storage::Status {
        self.require_whitelisted_money_market(money_market);
        self.mint_status(money_market).get()
    }

    /// Gets the current borrowing status at a given money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    /// # Notes:
    ///
    /// - By default, borrow is active (returns the first enum value).
    ///
    #[view(getBorrowStatus)]
    fn get_borrow_status(&self, money_market: &ManagedAddress) -> storage::Status {
        self.require_whitelisted_money_market(money_market);
        self.borrow_status(money_market).get()
    }

    /// Gets the current seizing status at a given money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    /// # Notes:
    ///
    /// - By default, seize is active (returns the first enum value).
    ///
    #[view(getSeizeStatus)]
    fn get_seize_status(&self, money_market: &ManagedAddress) -> storage::Status {
        self.require_whitelisted_money_market(money_market);
        self.seize_status(money_market).get()
    }

    /// Gets the current global seizing status at a given money market.
    ///
    /// # Notes:
    ///
    /// - By default, global seize is active (returns the first enum value).
    ///
    #[view(getGlobalSeizeStatus)]
    fn get_global_seize_status(&self) -> storage::Status {
        self.global_seize_status().get()
    }

    /// Gets the accrued rewards for a given account's address and rewards token ID.
    ///
    /// # Arguments:
    ///
    /// - `supplier` - A reference to a `ManagedAddress` representing the supplier's address.
    /// - `rewards_token_id` - A reference to an `EgldOrEsdtTokenIdentifier` representing the rewards token's ID.
    ///
    #[view(getAccountAccruedRewards)]
    fn get_account_accrued_rewards(&self, supplier: &ManagedAddress, rewards_token_id: &EgldOrEsdtTokenIdentifier) -> BigUint {
        self.account_accrued_rewards(supplier, rewards_token_id).get()
    }

    /// Gets the rewards index for a given money market, batch ID, and account.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - A reference to a `ManagedAddress` representing the money market's address.
    /// - `batch_id` - A reference to a `usize` representing the batch ID.
    /// - `account` - A reference to a `ManagedAddress` representing the account's address.
    ///
    fn get_account_batch_rewards_index(&self, money_market: &ManagedAddress, batch_id: &usize, account: &ManagedAddress) -> Option<BigUint> {
        let mapper = self.account_batch_rewards_index(money_market, batch_id, account);
        if mapper.is_empty() {
            None
        } else {
            let account_index = mapper.get();
            Some(account_index)
        }
    }

    /// Gets the Booster Observer address iff it has been set.
    ///
    fn get_booster_observer(&self) -> Option<ManagedAddress> {
        if self.booster_observer().is_empty() {
            None
        } else {
            let booster_observer = self.booster_observer().get();
            Some(booster_observer)
        }
    }

    /// Gest the USH Market Observer address iff it has been set.
    ///
    fn get_ush_market_observer(&self) -> Option<ManagedAddress> {
        if self.ush_market_observer().is_empty() {
            None
        } else {
            let ush_market = self.ush_market_observer().get();
            Some(ush_market)
        }
    }

    // Sets

    /// Sets the next collateral factors for a given money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `next_cf` - The next collateral factor.
    /// - `next_uf` - The next USH borrower collateral factor.
    ///
    fn set_next_collateral_factors(&self, money_market: &ManagedAddress, next_cf: &BigUint, next_uf: &BigUint) {
        let timestamp = self.blockchain().get_block_timestamp() + TIMELOCK_COLLATERAL_FACTOR_DECREASE;
        self.next_collateral_factors(money_market).set((timestamp, next_cf.clone(), next_uf.clone()));
        self.new_next_collateral_factors_event(timestamp, next_cf, next_uf);
    }

    /// Sets the maximum number of markets per account.
    ///
    /// # Arguments:
    ///
    /// `new_max_markets_per_account` - The new maximum number of markets per account.
    ///
    /// # Notes:
    ///
    /// - Requires that the new maximum number of markets per account is greater than the current maximum number of markets
    ///   per account.
    ///
    fn set_max_markets_per_account_internal(&self, new_max_markets_per_account: usize) {
        let old_max_markets_per_account = self.get_max_markets_per_account();
        require!(new_max_markets_per_account <= MAX_MARKETS_PER_ACCOUNT, ERROR_MAX_MARKETS_TOO_HIGH);
        require!(new_max_markets_per_account > old_max_markets_per_account, ERROR_MAX_MARKETS_TOO_LOW);
        self.max_markets_per_account().set(new_max_markets_per_account);
        self.new_max_markets_per_account_event(old_max_markets_per_account, new_max_markets_per_account);
    }

    // Market related methods

    /// Checks whether an account is allowed to enter a market based on the number of markets it has already deposited
    /// collateral or took a borrow.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `account` - The account we wish to add to the market.
    ///
    fn enter_market_allowed(&self, money_market: &ManagedAddress, account: &ManagedAddress) {
        self.require_whitelisted_money_market(money_market);
        let account_markets_mapper = self.account_markets(account);
        if account_markets_mapper.contains(money_market) {
            return;
        }
        require!(account_markets_mapper.len() < self.get_max_markets_per_account(), ERROR_TOO_MANY_MARKETS);
    }

    /// Handles internal logic for entering a market by updating the collateral and market information for a given account.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `account` - The account we wish to add to the market.
    /// - `tokens` - The amount of collateral tokens to add to the market.
    ///
    /// # Notes:
    ///
    /// - Updates the amount of collateral tokens held by the given account.
    /// - Updates the total amount of collateral tokens held by the given money market.
    /// - Adds the given money market to the list of assets deposited as collateral by the given account.
    /// - Adds the given account to the list of members for the given money market.
    ///
    fn enter_market_internal(&self, money_market: &ManagedAddress, account: &ManagedAddress, tokens: &BigUint) {
        // check if the account is allowed to enter the market
        self.enter_market_allowed(money_market, account);

        // update account collateral tokens
        let account_collateral_tokens_mapper = self.account_collateral_tokens(money_market, account);
        let old_tokens = account_collateral_tokens_mapper.get();
        account_collateral_tokens_mapper.update(|_tokens| *_tokens += tokens);

        // update total collateral tokens
        self.total_collateral_tokens(money_market).update(|_tokens| *_tokens += tokens);

        // track in which assets an account has deposited collateral
        self.account_markets(account).insert(money_market.clone());

        // we also track market members, i.e. accounts that belong to a given market
        self.market_members(money_market).insert(account.clone());

        // notify observers there has been a change in this market
        self.notify_market_observers(money_market, account, &old_tokens);

        self.enter_market_event(money_market, account, tokens);
    }

    /// Whitelisted money markets can burn their own tokens deposited at the controller.
    ///
    /// # Arguments:
    ///
    /// - `token_id` - The token identifier for the Hatom token.
    /// - `tokens` - The amount of tokens to be burnt.
    ///
    /// # Notes:
    ///
    /// - Can only be called by a whitelisted money market.
    /// - A money market can only burn Hatom tokens corresponding to their own token type.
    /// - There is no need to update the total collateral tokens for the money market because it is assumed that they have
    ///   already exited the market and are being redeemed.
    ///
    #[endpoint(burnTokens)]
    fn burn_tokens(&self, token_id: &TokenIdentifier, tokens: &BigUint) {
        if tokens == &BigUint::zero() {
            return;
        }

        let caller = self.blockchain().get_caller();
        self.require_whitelisted_money_market(&caller);

        require!(self.money_markets(token_id).get() == caller, ERROR_ONLY_MONEY_MARKET_CAN_BURN);

        let sc_address = self.blockchain().get_sc_address();
        let sc_balance = self.blockchain().get_esdt_balance(&sc_address, &token_id, 0);
        require!(tokens <= &sc_balance, ERROR_INSUFFICIENT_BALANCE);

        self.send().esdt_local_burn(token_id, 0, tokens);
    }

    /// Whitelisted money markets can transfer their own tokens to a given account.
    ///
    /// # Arguments:
    ///
    /// - `to` - The address of the account to which the tokens will be transferred.
    /// - `token_payment` - The token payment to be transferred.
    ///
    /// # Notes:
    ///
    /// - Can only be called by a whitelisted money market.
    /// - A money market can only transfer Hatom tokens corresponding to their own token type.
    /// - There is no need to update the total collateral tokens for the money market because it is assumed that they have
    ///   already exited the market and are being transferred.
    ///
    #[endpoint(transferTokens)]
    fn transfer_tokens(&self, to: &ManagedAddress, token_payment: &EsdtTokenPayment) {
        if token_payment.amount == BigUint::zero() {
            return;
        }

        let caller = self.blockchain().get_caller();
        self.require_whitelisted_money_market(&caller);

        require!(self.money_markets(&token_payment.token_identifier).get() == caller, ERROR_ONLY_MONEY_MARKET_CAN_TRANSFER);

        self.send().direct_non_zero_esdt_payment(to, token_payment);
    }

    /// Computes the amount of Hatom tokens to be seized given an underlying repayment amount performed by the liquidator.
    /// Takes into consideration the liquidation incentive, such that the liquidator gets tokens at a discount.
    ///
    /// # Arguments:
    ///
    /// - `borrow_market` - The money market where the borrower has borrow its underlying.
    /// - `collateral_market` - The money market where the borrower has collateral which is intended to be seized.
    /// - `amount` - The amount of underlying being repaid by the liquidator.
    ///
    #[endpoint(tokensToSeize)]
    fn tokens_to_seize(&self, borrow_market: &ManagedAddress, collateral_market: &ManagedAddress, amount: &BigUint) -> BigUint {
        // for exponential math
        let wad = BigUint::from(WAD);

        // no need to fetch prices if markets are the same
        let (borrow_price, collateral_price) = if borrow_market != collateral_market {
            let borrow_price = self.get_underlying_price(borrow_market);
            let collateral_price = self.get_underlying_price(collateral_market);
            (borrow_price, collateral_price) // [wad]
        } else {
            (wad.clone(), wad.clone())
        };

        // exchange rate [wad]
        let fx = self.get_stored_exchange_rate(collateral_market);

        // liquidation incentive [wad]
        let li = self.get_liquidation_incentive(collateral_market);

        let num = &li * &borrow_price; // [wad ^ 2]
        let den = &collateral_price * &fx / &wad; // [wad]
        let ratio = &num / &den; // [wad]

        let seized_tokens = amount * &ratio / &wad;

        seized_tokens
    }

    /// Swaps a given amount of tokens using a given swap path and returns the amount of resulting tokens. The path can be
    /// traversed in forward or backward mode.
    ///
    fn custom_swap(&self, path: &ManagedVec<SwapStep<Self::Api>>, fwd: bool, token_in: &TokenIdentifier, amount_in: &BigUint, token_out: &TokenIdentifier) -> BigUint {
        require!(!path.is_empty(), ERROR_INVALID_SWAP_PATH);

        let swap_fixed_input_endpoint = ManagedBuffer::from(SWAP_TOKENS_FIXED_INPUT_FUNC_NAME);
        let mut operations: MultiValueEncoded<SwapOperationType<Self::Api>> = MultiValueEncoded::new();

        for i in 0..path.len() {
            let j = if fwd { i } else { path.len() - i - 1 };
            let SwapStep { pair_address, output_token_id, input_token_id } = path.get(j);
            let token_wanted = if fwd { output_token_id } else { input_token_id };
            let swap_operation: SwapOperationType<Self::Api> = (pair_address, swap_fixed_input_endpoint.clone(), token_wanted, BigUint::from(1u64)).into();
            operations.push(swap_operation);
        }

        let token_out = EgldOrEsdtTokenIdentifier::esdt(token_out.clone());
        let token_out_prev = self.blockchain().get_sc_balance(&token_out, 0);

        self.multi_pair_swap(operations, token_in, amount_in);

        let token_out_post = self.blockchain().get_sc_balance(&token_out, 0);
        require!(token_out_post > token_out_prev, ERROR_UNEXPECTED_SWAP_AMOUNT);

        token_out_post - token_out_prev
    }

    /// Notifies market changes to all market observers.
    ///
    /// # Arguments
    ///
    /// - `money_market` - The address of the market where the collateral has changed.
    /// - `account` - The address of the account that has changed its collateral.
    /// - `prev_tokens` - The amount of collateral tokens the account had before the change.
    ///
    fn notify_market_observers(&self, money_market: &ManagedAddress, account: &ManagedAddress, prev_tokens: &BigUint) {
        let tokens = self.get_account_collateral_tokens(money_market, account);

        if let Some(booster_observer) = self.get_booster_observer() {
            let version = self.get_rewards_booster_version(&booster_observer);
            match version {
                1 => {
                    self.on_market_change_booster_v1(&booster_observer, money_market, account, &tokens);
                },
                2 => {
                    self.on_market_change_booster_v2(&booster_observer, money_market, account, &tokens, &prev_tokens);
                },
                _ => sc_panic!(ERROR_INVALID_BOOSTER_VERSION),
            }
        }

        if let Some(ush_market_observer) = self.get_ush_market_observer() {
            self.on_market_change_ush_market(&ush_market_observer, account);
        }
    }
}
