multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::{constants::*, errors::*, events, guardian, policies, proxies, rewards, risk_profile, shared, storage};

use crate::storage::{MarketType, RewardsBatch, RewardsBooster, State, SwapStep};

#[multiversx_sc::module]
pub trait GovernanceModule: admin::AdminModule + events::EventModule + guardian::GuardianModule + policies::PolicyModule + proxies::ProxyModule + rewards::RewardsModule + risk_profile::RiskProfileModule + shared::SharedModule + storage::StorageModule {
    /// Incorporates a money market in a list of accepted money markets (a whitelist). This action will add support for the
    /// provided money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The provided address must be a valid money market smart contract.
    /// - The money market should not has already been supported in the past.
    ///
    #[endpoint(supportMarket)]
    fn support_market(&self, money_market: &ManagedAddress) {
        self.require_admin();

        // must be a money market smart contract
        require!(self.is_money_market_sc(money_market), ERROR_INVALID_MONEY_MARKET_SC);

        // should not be supported
        require!(!self.is_whitelisted_money_market(money_market), ERROR_ALREADY_SUPPORTED_MARKET);

        // add to list
        self.whitelisted_markets().insert(money_market.clone());

        // populate useful mappers
        let (underlying_id, token_id) = self.get_money_market_identifiers(money_market);
        self.money_markets(&token_id).set(money_market);
        self.identifiers(money_market).set((underlying_id, token_id));

        // make sure pricing is available
        self.get_underlying_price(money_market);

        // make sure close factor has been set
        require!(self.get_close_factor(money_market) > BigUint::zero(), ERROR_MISSING_CLOSE_FACTOR);

        // make sure liquidation incentive has been set
        require!(self.get_liquidation_incentive(money_market) > BigUint::zero(), ERROR_MISSING_LIQUIDATION_INCENTIVE);

        self.support_money_market_event(money_market);
    }

    /// Sets the maximum number of money markets that can be entered per account.
    ///
    /// # Arguments:
    ///
    /// - `new_max_markets_per_account` - The new maximum number of money markets that can be entered per account.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - Must be higher than the current maximum.
    ///
    #[endpoint(setMaxMarketsPerAccount)]
    fn set_max_markets_per_account(&self, new_max_markets_per_account: usize) {
        self.require_admin();
        self.set_max_markets_per_account_internal(new_max_markets_per_account);
    }

    /// Sets the collateral factors or loan to values for a given money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `new_cf` - The new collateral factor in wad.
    /// - `new_uf` - The new USH borrower collateral factor in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The provided market must be a whitelisted money market.
    /// - The new collateral factors must not exceed their maximum allowed.
    /// - The new collateral factor cannot be lower than the previous one by more than the maximum allowed decrease.
    /// - The USH borrower collateral factor cannot exceed the collateral factor at any time.
    /// - A collateral factor of zero should be configured when a market is deprecated.
    ///
    #[endpoint(setCollateralFactors)]
    fn set_collateral_factors(&self, money_market: &ManagedAddress, new_cf: &BigUint, new_uf: &BigUint) {
        self.require_admin();
        self.require_whitelisted_money_market(money_market);

        let max_cf = BigUint::from(MAX_COLLATERAL_FACTOR);
        require!(new_cf <= &max_cf, ERROR_COLLATERAL_FACTOR_TOO_HIGH);
        require!(new_cf >= new_uf, ERROR_USH_BORROWER_COLLATERAL_FACTOR_TOO_HIGH);

        // note that deprecated markets set a collateral factor equal to zero
        // make sure the price oracle can price the underlying when the new collateral factor is != 0
        if new_cf != &BigUint::zero() {
            self.get_underlying_price(money_market);
        }

        // get current valid values
        let (cf, uf) = self.update_and_get_collateral_factors(money_market);

        if new_cf < &cf && new_uf < &uf {
            self.require_valid_collateral_factor_decrease(new_cf, &cf);
            self.require_valid_collateral_factor_decrease(new_uf, &uf);

            self.set_next_collateral_factors(money_market, new_cf, new_uf);
        } else if new_cf < &cf && new_uf >= &uf {
            // since uf <= new_uf <= new_cf < cf => new_uf < cf, so there is no need to verify
            self.require_valid_collateral_factor_decrease(new_cf, &cf);

            // can be instantly set
            self.ush_borrower_collateral_factor(money_market).set(new_uf);
            self.new_ush_borrower_collateral_factor_event(money_market, &uf, new_uf);

            self.set_next_collateral_factors(money_market, new_cf, new_uf);
        } else if new_cf >= &cf && new_uf < &uf {
            self.require_valid_collateral_factor_decrease(new_uf, &uf);

            // can be instantly set
            self.collateral_factor(money_market).set(new_cf);
            self.new_collateral_factor_event(money_market, &cf, new_cf);

            self.set_next_collateral_factors(money_market, new_cf, new_uf);
        } else {
            // remove any pending changes
            self.next_collateral_factors(money_market).clear();
            self.clear_next_collateral_factors_event();

            // can be instantly set
            self.collateral_factor(money_market).set(new_cf);
            self.new_collateral_factor_event(money_market, &cf, new_cf);

            // can be instantly set
            self.ush_borrower_collateral_factor(money_market).set(new_uf);
            self.new_ush_borrower_collateral_factor_event(money_market, &uf, new_uf);
        }
    }

    /// Sets the pricing Oracle smart contract address.
    ///
    /// # Arguments:
    ///
    /// - `new_price_oracle` - The address of the pricing oracle smart contract.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The provided address must be a valid oracle smart contract.
    ///
    #[endpoint(setPriceOracle)]
    fn set_price_oracle(&self, new_price_oracle: &ManagedAddress) {
        self.require_admin();

        require!(self.is_price_oracle_sc(new_price_oracle), ERROR_INVALID_ORACLE_SC);

        let old_price_oracle_address = self.get_price_oracle();
        self.price_oracle().set(new_price_oracle);

        // make sure it can price all whitelisted markets
        for market in self.whitelisted_markets().iter() {
            self.get_underlying_price(&market);
        }

        self.new_price_oracle_event(&old_price_oracle_address, new_price_oracle);
    }

    /// Sets a liquidity cap for a given money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `new_liquidity_cap` - The new liquidity cap in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The provided address must be a whitelisted money market.
    ///
    #[endpoint(setLiquidityCap)]
    fn set_liquidity_cap(&self, money_market: &ManagedAddress, new_liquidity_cap: &BigUint) {
        self.require_admin();
        self.require_whitelisted_money_market(money_market);
        let old_liquidity_cap = self.get_liquidity_cap(money_market);
        self.liquidity_cap(money_market).set(new_liquidity_cap);
        self.new_liquidity_cap_event(money_market, &old_liquidity_cap, new_liquidity_cap);
    }

    /// Sets a borrow cap for a given money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `new_borrow_cap` - The new borrow cap in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The provided address must be a whitelisted money market.
    ///
    #[endpoint(setBorrowCap)]
    fn set_borrow_cap(&self, money_market: &ManagedAddress, new_borrow_cap: &BigUint) {
        self.require_admin();
        self.require_whitelisted_money_market(money_market);
        let old_borrow_cap = self.get_borrow_cap(money_market);
        self.borrow_cap(money_market).set(new_borrow_cap);
        self.new_borrow_cap_event(money_market, &old_borrow_cap, new_borrow_cap);
    }

    /// Sets the maximum amount of rewards batches per money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `new_max_rewards_batches` - The new maximum amount of rewards batches.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The provided address must be a whitelisted money market.
    ///
    #[endpoint(setMaxRewardsBatches)]
    fn set_max_rewards_batches(&self, money_market: &ManagedAddress, new_max_rewards_batches: usize) {
        self.require_admin();
        self.require_whitelisted_money_market(money_market);

        let old_max_rewards_batches = self.max_rewards_batches(money_market).get();
        require!(new_max_rewards_batches <= MAX_REWARDS_BATCHES, ERROR_MAX_REWARDS_BATCHES_TOO_HIGH);
        require!(new_max_rewards_batches > old_max_rewards_batches, ERROR_MAX_REWARDS_BATCHES_TOO_LOW);

        self.max_rewards_batches(money_market).set(new_max_rewards_batches);

        self.new_max_rewards_batches_event(money_market, old_max_rewards_batches, new_max_rewards_batches);
    }

    /// Sets the maximum slippage allowed for configuration swaps.
    ///
    /// # Arguments:
    ///
    /// - `new_max_slippage` - The new maximum slippage allowed.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setMaxSlippage)]
    fn set_max_slippage(&self, new_max_slippage: &BigUint) {
        self.require_admin();

        let old_max_slippage = self.max_slippage().get();
        require!(new_max_slippage <= &BigUint::from(MAX_SLIPPAGE), ERROR_MAX_SLIPPAGE_TOO_HIGH);
        self.max_slippage().set(new_max_slippage);

        self.new_max_slippage_event(&old_max_slippage, new_max_slippage);
    }

    /// Adds a rewards batch to the specified money market. EGLD or ESDT tokens are supported.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `market_type` - Distribute rewards for suppliers (`Supply`) or lenders (`Borrows`).
    /// - `period` - The period of time in seconds in which rewards are distributed.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or rewards manager.
    /// - The provided address must be whitelisted money market.
    /// - Should be paid with the rewards token.
    ///
    #[payable("*")]
    #[endpoint(setRewardsBatch)]
    fn set_rewards_batch(&self, money_market: &ManagedAddress, market_type: MarketType, period: u64) -> usize {
        self.require_admin_or_rewards_manager();
        self.require_whitelisted_money_market(money_market);

        require!(period > 0u64, ERROR_ZERO_REWARDS_BATCH_PERIOD);

        let mut rewards_batches_mapper = self.rewards_batches(money_market);
        let max_rewards_batches = self.max_rewards_batches(money_market).get();
        require!(rewards_batches_mapper.len() < max_rewards_batches, ERROR_TOO_MANY_REWARDS_BATCHES);

        let (rewards_token_id, amount) = self.call_value().egld_or_single_fungible_esdt();

        if let Some(token_id) = rewards_token_id.as_esdt_option() {
            require!(!self.is_whitelisted_token_id(&token_id), ERROR_INVALID_REWARDS_TOKEN_ID);
        }

        require!(amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);

        let wad = BigUint::from(WAD);
        let batch_id = self.get_next_rewards_batch_id(money_market);
        let timestamp = self.blockchain().get_block_timestamp();
        let speed = &amount * &wad / period;
        require!(speed > BigUint::zero(), ERROR_ZERO_REWARDS_BATCH_SPEED);

        let batch = RewardsBatch {
            id: batch_id,
            money_market: money_market.clone(),
            market_type: market_type.clone(),
            token_id: rewards_token_id,
            amount,
            distributed_amount: BigUint::zero(),
            speed,
            index: &wad * &wad,
            last_time: timestamp,
            end_time: timestamp + period,
        };

        let pos_id = rewards_batches_mapper.push(&batch);
        self.rewards_batch_position(money_market, &batch_id).set(pos_id);

        self.set_rewards_batch_event(&self.blockchain().get_caller(), &batch);

        if market_type == MarketType::Supply {
            self.update_supply_rewards_batches_state(money_market);
        } else {
            self.update_borrow_rewards_batches_state(money_market);
        }

        batch_id
    }

    /// Adds an amount of reward token to an existing rewards batch maintaining the same speed.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `batch_id` - the rewards batch identifier
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or rewards manager.
    ///
    #[payable("*")]
    #[endpoint(addRewardsBatch)]
    fn add_rewards_batch(&self, money_market: &ManagedAddress, batch_id: usize) {
        self.require_admin_or_rewards_manager();
        self.require_whitelisted_money_market(money_market);

        let rewards_batch_position_mapper = self.rewards_batch_position(money_market, &batch_id);
        require!(!rewards_batch_position_mapper.is_empty(), ERROR_INVALID_REWARDS_BATCH_ID);
        let pos_id = rewards_batch_position_mapper.get();

        let mut rewards_batches_mapper = self.rewards_batches(money_market);
        let rewards_batch = rewards_batches_mapper.get(pos_id);

        let (rewards_token_id, amount) = self.call_value().egld_or_single_fungible_esdt();
        require!(rewards_token_id == rewards_batch.token_id, ERROR_INVALID_PAYMENT);
        require!(amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);

        // this will update all rewards batches from a given money market up to this point
        if rewards_batch.market_type == MarketType::Supply {
            self.update_supply_rewards_batches_state(money_market);
        } else {
            self.update_borrow_rewards_batches_state(money_market);
        }

        // after updating it, get it again
        let mut updated_rewards_batch = rewards_batches_mapper.get(pos_id);

        // update
        let wad = BigUint::from(WAD);
        let t = self.blockchain().get_block_timestamp();
        let additional_dt = &amount * &wad / &updated_rewards_batch.speed;
        let dt = match BigUint::to_u64(&additional_dt) {
            None => sc_panic!(ERROR_UNEXPECTED_REWARDS_BATCH_PERIOD),
            Some(dt) => {
                require!(dt > 0u64, ERROR_ZERO_REWARDS_BATCH_PERIOD);
                dt
            },
        };

        if t > updated_rewards_batch.end_time {
            // if batch has already expired, make it "active"
            updated_rewards_batch.last_time = t;
            updated_rewards_batch.end_time = t + dt;
        } else {
            updated_rewards_batch.end_time += dt;
        }
        updated_rewards_batch.amount += amount;

        // store
        rewards_batches_mapper.set(pos_id, &updated_rewards_batch);

        self.add_rewards_batch_event(&self.blockchain().get_caller(), &updated_rewards_batch);
    }

    /// Cancel a specified rewards batch. Remaining tokens are sent back to a beneficiary.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - the address of the money market smart contract.
    /// - `batch_id` - the rewards batch identifier
    /// - `opt_to` - the beneficiary address for the remaining tokens (optional)
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or rewards manager.
    /// - The caller is selected if no beneficiary is given.
    ///
    #[endpoint(cancelRewardsBatch)]
    fn cancel_rewards_batch(&self, money_market: &ManagedAddress, batch_id: usize, opt_to: OptionalValue<ManagedAddress>) {
        self.require_admin_or_rewards_manager();
        self.require_whitelisted_money_market(money_market);

        let rewards_batch_position_mapper = self.rewards_batch_position(money_market, &batch_id);
        require!(!rewards_batch_position_mapper.is_empty(), ERROR_INVALID_REWARDS_BATCH_ID);
        let pos_id = rewards_batch_position_mapper.get();

        let mut rewards_batches_mapper = self.rewards_batches(money_market);
        let rewards_batch = rewards_batches_mapper.get(pos_id);

        let t = self.blockchain().get_block_timestamp();
        require!(rewards_batch.end_time > t, ERROR_REWARDS_BATCH_EXPIRED);

        // this will update all rewards batches from a given money market up to this point
        if rewards_batch.market_type == MarketType::Supply {
            self.update_supply_rewards_batches_state(money_market);
        } else {
            self.update_borrow_rewards_batches_state(money_market);
        }

        // after updating it, get it again
        let mut updated_rewards_batch = rewards_batches_mapper.get(pos_id);

        // get the amount left
        let wad = BigUint::from(WAD);
        let amount_left = &updated_rewards_batch.speed * (&updated_rewards_batch.end_time - t) / &wad;

        // update
        updated_rewards_batch.end_time = t;
        updated_rewards_batch.amount -= &amount_left;

        // store
        rewards_batches_mapper.set(pos_id, &updated_rewards_batch);

        // get beneficiary
        let caller = self.blockchain().get_caller();
        let to = match opt_to {
            OptionalValue::None => caller,
            OptionalValue::Some(to) => to,
        };

        // make sure there is balance in the contract
        let sc_balance = self.blockchain().get_sc_balance(&updated_rewards_batch.token_id, 0);
        require!(amount_left <= sc_balance, ERROR_INSUFFICIENT_BALANCE);
        self.send().direct(&to, &updated_rewards_batch.token_id, 0, &amount_left);

        self.cancel_rewards_batch_event(&self.blockchain().get_caller(), &updated_rewards_batch);
    }

    /// Removes a specified rewards batch from the array of rewards batches iff it has been fully distributed.
    ///
    /// # Arguments
    ///
    /// - `money_market` - the address of the money market smart contract.
    /// - `batch_id` - the rewards batch identifier
    ///
    /// # Notes
    ///
    /// - can be called by anyone
    /// - takes into consideration possible rounding errors but it is conservative
    ///
    #[endpoint(removeRewardsBatch)]
    fn remove_rewards_batch(&self, money_market: &ManagedAddress, batch_id: usize) {
        self.require_whitelisted_money_market(money_market);

        let rewards_batch_position_mapper = self.rewards_batch_position(money_market, &batch_id);
        require!(!rewards_batch_position_mapper.is_empty(), ERROR_INVALID_REWARDS_BATCH_ID);
        let pos_id = rewards_batch_position_mapper.get();
        let rewards_batch = self.rewards_batches(money_market).get(pos_id);

        // take into consideration possible rounding errors
        require!(rewards_batch.distributed_amount >= rewards_batch.amount, ERROR_REWARDS_NOT_FULLY_DISTRIBUTED);

        // remove rewards batch
        self.remove_rewards_batch_internal(money_market, batch_id, pos_id);
    }

    /// Removes a specified rewards batch from the array of rewards batches iff it has been fully distributed within a given
    /// tolerance amount.
    ///
    /// # Arguments
    ///
    /// - `money_market` - the address of the money market smart contract.
    /// - `batch_id` - the rewards batch identifier
    /// - `tolerance` - the tolerance in wad, such that 1 wad = 100%.
    ///
    /// # Notes
    ///
    /// - can only be called by the admin or rewards manager.
    ///
    #[endpoint(adminRemoveRewardsBatch)]
    fn admin_remove_rewards_batch(&self, money_market: &ManagedAddress, batch_id: usize, tolerance: &BigUint) {
        self.require_admin_or_rewards_manager();
        self.require_whitelisted_money_market(money_market);

        let rewards_batch_position_mapper = self.rewards_batch_position(money_market, &batch_id);
        require!(!rewards_batch_position_mapper.is_empty(), ERROR_INVALID_REWARDS_BATCH_ID);
        let pos_id = rewards_batch_position_mapper.get();
        let rewards_batch = self.rewards_batches(money_market).get(pos_id);

        // take into consideration possible rounding errors
        if rewards_batch.amount > rewards_batch.distributed_amount {
            let wad = BigUint::from(WAD);
            let timestamp = self.blockchain().get_block_timestamp();
            require!(timestamp > rewards_batch.end_time, ERROR_REWARDS_BATCH_NOT_EXPIRED);
            require!(tolerance >= &BigUint::from(MIN_REWARDS_BATCH_TOLERANCE) && tolerance <= &wad, ERROR_REWARDS_BATCH_TOLERANCE_OUT_OF_RANGE);
            require!(rewards_batch.distributed_amount * wad >= rewards_batch.amount * tolerance, ERROR_REWARDS_NOT_FULLY_DISTRIBUTED);
        }

        self.remove_rewards_batch_internal(money_market, batch_id, pos_id);
    }

    /// Removes a specified rewards batch from the array of rewards batches.
    ///
    fn remove_rewards_batch_internal(&self, money_market: &ManagedAddress, batch_id: usize, pos_id: usize) {
        let mut rewards_batches_mapper = self.rewards_batches(money_market);
        let last_pos_id = rewards_batches_mapper.len();
        let last_batch_id = rewards_batches_mapper.get(last_pos_id).id;

        // remove batch at pos id and swap last to pos id
        rewards_batches_mapper.swap_remove(pos_id);

        // update last batch position id
        self.rewards_batch_position(money_market, &last_batch_id).set(pos_id);

        // clear position for removed batch
        self.rewards_batch_position(money_market, &batch_id).clear();

        self.remove_rewards_batch_event(money_market, batch_id);
    }

    /// Updates a given rewards batch based on a new speed. The new speed of rewards also changes the remaining distribution
    /// time period.
    ///
    ///
    /// # Arguments:
    ///
    /// - `money_market` - the address of the money market smart contract.
    /// - `batch_id` - The rewards batch identifier.
    /// - `new_speed` - The new speed of rewards in wad.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or rewards manager.
    ///
    #[endpoint(updateRewardsBatchSpeed)]
    fn update_rewards_batch_speed(&self, money_market: &ManagedAddress, batch_id: usize, new_speed: &BigUint) {
        self.require_admin_or_rewards_manager();
        self.require_whitelisted_money_market(money_market);

        require!(*new_speed > BigUint::zero(), ERROR_ZERO_REWARDS_BATCH_SPEED);

        let rewards_batch_position_mapper = self.rewards_batch_position(money_market, &batch_id);
        require!(!rewards_batch_position_mapper.is_empty(), ERROR_INVALID_REWARDS_BATCH_ID);
        let pos_id = rewards_batch_position_mapper.get();

        let mut rewards_batches_mapper = self.rewards_batches(money_market);
        let rewards_batch = rewards_batches_mapper.get(pos_id);

        require!(rewards_batch.speed != *new_speed, ERROR_UNEXPECTED_REWARDS_BATCH_SPEED);

        let t = self.blockchain().get_block_timestamp();
        require!(rewards_batch.end_time > t, ERROR_REWARDS_BATCH_EXPIRED);

        // this will update all rewards batches from a given money market up to this point
        if rewards_batch.market_type == MarketType::Supply {
            self.update_supply_rewards_batches_state(money_market);
        } else {
            self.update_borrow_rewards_batches_state(money_market);
        }

        // after updating it, get it again
        let mut updated_rewards_batch = rewards_batches_mapper.get(pos_id);

        // update
        let old_dt = updated_rewards_batch.end_time - t;
        let new_dt = updated_rewards_batch.speed * old_dt / new_speed;
        let dt = match BigUint::to_u64(&new_dt) {
            None => sc_panic!(ERROR_UNEXPECTED_REWARDS_BATCH_PERIOD),
            Some(dt) => {
                require!(dt > 0u64, ERROR_ZERO_REWARDS_BATCH_PERIOD);
                dt
            },
        };
        updated_rewards_batch.speed = new_speed.clone();
        updated_rewards_batch.end_time = t + dt;

        // store
        rewards_batches_mapper.set(pos_id, &updated_rewards_batch);

        self.update_rewards_batch_speed_event(&self.blockchain().get_caller(), &updated_rewards_batch);
    }

    /// Updates a given rewards batch based on a new period. The new period also changes the speed of rewards.
    ///
    ///
    /// # Arguments:
    ///
    /// - `money_market` - the address of the money market smart contract.
    /// - `batch_id` - The rewards batch identifier.
    /// - `new_dt` - The new period.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or rewards manager.
    ///
    #[endpoint(updateRewardsBatchRemainingPeriod)]
    fn update_rewards_batch_remaining_period(&self, money_market: &ManagedAddress, batch_id: usize, new_dt: u64) {
        self.require_admin_or_rewards_manager();
        self.require_whitelisted_money_market(money_market);

        require!(new_dt > 0u64, ERROR_ZERO_REWARDS_BATCH_PERIOD);

        let rewards_batch_position_mapper = self.rewards_batch_position(money_market, &batch_id);
        require!(!rewards_batch_position_mapper.is_empty(), ERROR_INVALID_REWARDS_BATCH_ID);
        let pos_id = rewards_batch_position_mapper.get();

        let mut rewards_batches_mapper = self.rewards_batches(money_market);
        let rewards_batch = rewards_batches_mapper.get(pos_id);

        let t = self.blockchain().get_block_timestamp();
        require!(rewards_batch.end_time > t, ERROR_REWARDS_BATCH_EXPIRED);

        let old_dt = rewards_batch.end_time - t;
        require!(old_dt != new_dt, ERROR_UNEXPECTED_REWARDS_BATCH_PERIOD);

        // this will update all rewards batches from a given money market up to this point
        if rewards_batch.market_type == MarketType::Supply {
            self.update_supply_rewards_batches_state(money_market);
        } else {
            self.update_borrow_rewards_batches_state(money_market);
        }

        // after updating it, get it again
        let mut updated_rewards_batch = rewards_batches_mapper.get(pos_id);

        // update
        let new_speed = updated_rewards_batch.speed * old_dt / BigUint::from(new_dt);
        require!(new_speed > BigUint::zero(), ERROR_ZERO_REWARDS_BATCH_SPEED);
        updated_rewards_batch.end_time = t + new_dt;
        updated_rewards_batch.speed = new_speed;

        // store
        rewards_batches_mapper.set(pos_id, &updated_rewards_batch);

        self.update_rewards_batch_remaining_period_event(&self.blockchain().get_caller(), &updated_rewards_batch);
    }

    /// Claims the undistributed rewards for a given rewards token.
    ///
    /// # Arguments:
    ///
    /// - `rewards_token_id` - the rewards token identifier
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - The rewards token must have undistributed rewards.
    /// - Undistributed rewards might originate at markets without collateral or borrows, or because of truncation errors.
    ///
    #[endpoint(claimUndistributedRewards)]
    fn claim_undistributed_rewards(&self, rewards_token_id: &EgldOrEsdtTokenIdentifier) {
        self.require_admin();

        let amount = self.undistributed_rewards(rewards_token_id).take();

        require!(amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);

        let admin = self.get_admin();
        self.send().direct(&admin, rewards_token_id, 0, &amount);

        self.claim_undistributed_rewards_event(&admin, &rewards_token_id, &amount);
    }

    /// Adds support for boosting rewards batches by converting the rewards batch tokens into Hatom's governance tokens with
    /// a premium.
    ///
    /// # Arguments:
    ///
    /// - `governance_token_id` - the governance token identifier
    /// - `egld_wrapper` - the address of the EGLD wrapper smart contract
    /// - `router` - the address of the router smart contract
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(supportRewardsBatchBoosting)]
    fn support_rewards_batch_boosting(&self, governance_token_id: &TokenIdentifier, egld_wrapper: &ManagedAddress, router: &ManagedAddress) {
        self.require_admin();

        let wegld_id = self.get_wegld_id(egld_wrapper);
        self.egld_wrapper().set_if_empty(egld_wrapper);
        self.wegld_id().set_if_empty(&wegld_id);
        self.governance_token_id().set_if_empty(governance_token_id);
        self.router().set_if_empty(router);

        self.rewards_batch_boosting_supported().set(true);

        self.support_rewards_batch_boosting_event();
    }

    /// Enables support for boosting rewards batches.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(enableRewardsBatchBoosting)]
    fn enable_rewards_batch_boosting(&self) {
        self.require_admin();
        require!(self.rewards_batch_boosting_supported().get(), ERROR_REWARDS_BATCH_BOOST_NOT_ENABLED);
        self.boosting_state().set(State::Active);
        self.enable_rewards_batch_boosting_event();
    }

    /// Disables support for boosting rewards batches.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(disableRewardsBatchBoosting)]
    fn disable_rewards_batch_boosting(&self) {
        self.require_admin();
        self.boosting_state().set(State::Inactive);
        self.disable_rewards_batch_boosting_event();
    }

    /// Boosts the rewards of a given rewards token by converting the rewards tokens into Hatom's governance token with a
    /// premium.
    ///
    /// # Arguments:
    ///
    /// - `premium` - the premium in wad, such that 1 wad = 100%.
    /// - `fwd_swap_amount` - the amount of tokens to swap.
    /// - `fwd_swap_path` - the swap path to convert the rewards batch tokens into Hatom's governance tokens.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or rewards manager.
    /// - If rewards token is EGLD, swaps will add a EGLD => WEGLD step first. Also, the swap path needs to use the WEGLD
    ///   token identifier.
    ///
    #[payable("*")]
    #[endpoint(boostRewards)]
    fn boost_rewards(&self, premium: BigUint, fwd_swap_amount: BigUint, fwd_swap_path: ManagedVec<SwapStep<Self::Api>>) {
        self.require_admin_or_rewards_manager();

        require!(self.boosting_state().get() == State::Active, ERROR_BOOSTING_NOT_ACTIVE);

        require!(premium <= MAX_PREMIUM, ERROR_INVALID_PREMIUM);

        let (rewards_token_id, mut amount) = self.call_value().egld_or_single_fungible_esdt();
        require!(amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);

        require!(fwd_swap_amount > BigUint::zero() && fwd_swap_amount <= amount, ERROR_INVALID_SWAP_AMOUNT);

        require!(self.token_has_active_rewards_batch(&rewards_token_id), ERROR_INVALID_REWARDS_TOKEN_ID);

        let booster_mapper = self.rewards_booster(&rewards_token_id);
        require!(booster_mapper.is_empty(), ERROR_REWARDS_TOKEN_ALREADY_BOOSTED);

        // if rewards token is EGLD then add a EGLD => WEGLD step first
        let swap_token_id = if rewards_token_id.is_egld() {
            self.wrap_egld(&fwd_swap_amount);
            self.wegld_id().get()
        } else {
            rewards_token_id.clone().unwrap_esdt()
        };

        // the output token
        let governance_token_id = self.governance_token_id().get();

        // swap rewards batch tokens into governance token
        let bwd_swap_amount = self.custom_swap(&fwd_swap_path, true, &swap_token_id, &fwd_swap_amount, &governance_token_id);

        // swap governance token into rewards batch tokens
        let fwd_bwd_swap_amount = self.custom_swap(&fwd_swap_path, false, &governance_token_id, &bwd_swap_amount, &swap_token_id);

        // because of slippage, the amount of tokens we get back from the second swap might be less than the amount we put in
        // the first swap
        require!(fwd_swap_amount >= fwd_bwd_swap_amount, ERROR_EXPECTED_SLIPPAGE);
        let delta_amount = &fwd_swap_amount - &fwd_bwd_swap_amount;

        // make sure we don't lose too much money
        let wad = BigUint::from(WAD);
        let max_slippage = self.max_slippage().get();
        let max_slippage_amount = fwd_swap_amount * &max_slippage / wad;
        require!(delta_amount <= max_slippage_amount, ERROR_TOO_MUCH_SLIPPAGE);

        // lost some tokens due to slippage
        amount -= &delta_amount;

        // if rewards token is EGLD, unwrap WEGLD into EGLD
        if rewards_token_id.is_egld() {
            self.unwrap_egld(&fwd_bwd_swap_amount);
        }

        // create booster
        let booster = RewardsBooster {
            token_id: rewards_token_id,
            premium,
            amount_left: amount,
            distributed_amount: BigUint::zero(),
            swap_path: fwd_swap_path,
        };

        // store
        booster_mapper.set(&booster);

        self.boost_rewards_event(&self.blockchain().get_caller(), &booster);
    }

    /// Updates the premium of a given booster and, if a payment is provided, adds it to the booster's amount.
    ///
    /// # Arguments:
    ///
    /// - `rewards_token_id` - the rewards token identifier for which we wish to update its booster.
    /// - `premium` - the premium in wad, such that 1 wad = 100%.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or rewards manager.
    /// - Cannot change the swap path. That requires canceling the booster and creating a new one.
    ///
    #[payable("*")]
    #[endpoint(updateBooster)]
    fn update_booster(&self, rewards_token_id: EgldOrEsdtTokenIdentifier, premium: BigUint) {
        self.require_admin_or_rewards_manager();

        let booster_mapper = self.rewards_booster(&rewards_token_id);
        require!(!booster_mapper.is_empty(), ERROR_INVALID_REWARDS_TOKEN_ID);
        let mut booster = booster_mapper.get();

        require!(premium <= MAX_PREMIUM, ERROR_INVALID_PREMIUM);

        // if there is no payment, `egld_or_single_fungible_esdt` returns a payment of 0 EGLD
        let (token_id, amount) = self.call_value().egld_or_single_fungible_esdt();

        if amount > BigUint::zero() {
            require!(token_id == rewards_token_id, ERROR_INVALID_PAYMENT);
            require!(self.token_has_active_rewards_batch(&rewards_token_id), ERROR_INVALID_REWARDS_TOKEN_ID);
            booster.amount_left += &amount;
        }

        booster.premium = premium;
        booster_mapper.set(&booster);

        self.update_booster_event(&self.blockchain().get_caller(), &booster);
    }

    /// Cancels a given booster and sends the remaining tokens back to the caller.
    ///
    /// # Arguments:
    ///
    /// - `rewards_token_id` - the rewards token identifier for which we wish to cancel its booster.
    /// - `opt_to` - the beneficiary address for the remaining tokens (optional).
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or rewards manager.
    ///
    #[endpoint(cancelBooster)]
    fn cancel_booster(&self, rewards_token_id: EgldOrEsdtTokenIdentifier, opt_to: OptionalValue<ManagedAddress>) {
        self.require_admin_or_rewards_manager();

        let booster_mapper = self.rewards_booster(&rewards_token_id);
        require!(!booster_mapper.is_empty(), ERROR_INVALID_REWARDS_TOKEN_ID);

        // get beneficiary
        let caller = self.blockchain().get_caller();
        let to = match opt_to {
            OptionalValue::None => caller,
            OptionalValue::Some(to) => to,
        };

        let RewardsBooster { amount_left, .. } = booster_mapper.get();

        // make sure there is balance in the contract
        if amount_left > BigUint::zero() {
            let sc_balance = self.blockchain().get_sc_balance(&rewards_token_id, 0);
            require!(amount_left <= sc_balance, ERROR_INSUFFICIENT_BALANCE);
            self.send().direct(&to, &rewards_token_id, 0, &amount_left);
        }

        booster_mapper.clear();

        self.cancel_booster_event(&self.blockchain().get_caller(), &rewards_token_id);
    }

    /// Updates the collateral or account tokens of a given account in a given money market, which is useful at liquidations.
    /// The general idea is that the account is removing collateral, which should update the total collateral tokens and the
    /// account's collateral tokens.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `account` - The address of the account we wish to update.
    /// - `tokens` - The number of Hatom's tokens to set as collateral.
    ///
    /// # Notes:
    ///
    /// - Can only be called by a whitelisted money market.
    /// - The provided address must be a whitelisted money market.
    /// - Makes sure the mappers `account_markets` and `market_members` remain updated.
    ///
    #[endpoint(setAccountTokens)]
    fn set_account_collateral_tokens(&self, money_market: &ManagedAddress, account: &ManagedAddress, new_tokens: &BigUint) {
        let caller = self.blockchain().get_caller();
        self.require_whitelisted_money_market(&caller);
        self.require_whitelisted_money_market(money_market);

        // update total collateral tokens
        let account_collateral_tokens_mapper = self.account_collateral_tokens(money_market, account);
        let old_tokens = account_collateral_tokens_mapper.get();
        if &old_tokens > new_tokens {
            let delta_tokens = &old_tokens - new_tokens;
            self.total_collateral_tokens(money_market).update(|tokens| *tokens -= delta_tokens);
        } else {
            let delta_tokens = new_tokens - &old_tokens;
            self.total_collateral_tokens(money_market).update(|tokens| *tokens += delta_tokens);
        }

        // update account collateral tokens
        account_collateral_tokens_mapper.set(new_tokens);

        let (underlying_owed, _) = self.get_account_snapshot(money_market, account);
        if new_tokens == &BigUint::zero() && underlying_owed == BigUint::zero() {
            // remove account from market if it does not hold collateral neither an outstanding borrow: this is particularly
            // useful for borrowers being liquidated and having their collateral seized
            self.account_markets(account).swap_remove(money_market);
            self.market_members(money_market).swap_remove(account);
        } else {
            // otherwise, make sure the account is in the market: this is particularly useful for liquidators having their
            // collateral increased (legacy case)
            self.account_markets(account).insert(money_market.clone());
            self.market_members(money_market).insert(account.clone());
        }

        // notify observers there has been a change in this market
        self.notify_market_observers(money_market, account, &old_tokens);
    }

    /// Sets the Rewards Manager of the protocol.
    ///
    /// # Arguments:
    ///
    /// - `new_rewards_manager` - The address of the new Rewards Manager.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setRewardsManager)]
    fn set_rewards_manager(&self, new_rewards_manager: &ManagedAddress) {
        self.require_admin();
        let old_rewards_manager = self.get_rewards_manager();
        self.rewards_manager().set(new_rewards_manager);
        self.new_rewards_manager_event(&old_rewards_manager, new_rewards_manager);
    }

    /// Sets the Guardian of the protocol.
    ///
    /// # Arguments:
    ///
    /// - `new_pause_guardian` - The address of the new Guardian.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setPauseGuardian)]
    fn set_pause_guardian(&self, new_pause_guardian: &ManagedAddress) {
        self.require_admin();
        let old_pause_guardian = self.get_pause_guardian();
        self.pause_guardian().set(new_pause_guardian);
        self.new_pause_guardian_event(&old_pause_guardian, new_pause_guardian);
    }

    /// Sets a Rewards Booster smart contract as an observer, i.e. as a contract that is notified when accounts deposit or
    /// withdraw collateral from markets. The name Booster Observer is used to reference the Rewards Booster smart contract.
    ///
    /// # Arguments:
    ///
    /// - `new_booster_observer` - the rewards booster smart contract address.
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - `new_booster_observer` must be a rewards booster smart contract
    /// - `new_booster_observer` must not have been already used as a rewards booster
    ///
    #[endpoint(setBoosterObserver)]
    fn set_booster_observer(&self, new_booster_observer: &ManagedAddress) {
        self.require_admin();
        require!(self.is_rewards_booster_sc(new_booster_observer), ERROR_INVALID_REWARDS_BOOSTER_SC);
        require!(self.booster_observer().is_empty(), ERROR_REWARDS_BOOSTER_ALREADY_SET);
        require!(!self.historical_observers(new_booster_observer).get(), ERROR_LEGACY_BOOSTER_OBSERVER);
        self.booster_observer().set(new_booster_observer);
        self.historical_observers(new_booster_observer).set(true);
        self.set_booster_observer_event(new_booster_observer);
    }

    /// Removes Rewards Booster smart contract from being an observer. From this point onwards, this smart contract will not
    /// be notified of any market change.
    ///
    #[endpoint(clearBoosterObserver)]
    fn clear_booster_observer(&self) {
        self.require_admin();

        let booster_observer_mapper = self.booster_observer();
        require!(!booster_observer_mapper.is_empty(), ERROR_REWARDS_BOOSTER_UNSET);

        let old_booster_observer = booster_observer_mapper.take();
        require!(self.is_finalized(&old_booster_observer), ERROR_REWARDS_BOOSTER_NOT_FINALIZED);

        self.clear_booster_observer_event(&old_booster_observer);
    }

    /// Sets a USH Money Market smart contract as an observer, i.e. as a contract that is notified when accounts deposit or
    /// withdraw collateral from markets. The name USH Market Observer is used to reference the USH Money Market smart
    /// contract.
    ///
    /// # Arguments:
    ///
    /// - `new_ush_market_observer` - The USH Money Market smart contract address.
    ///
    /// # Notes
    ///
    /// - can only be called by the admin
    /// - `new_ush_market_observer` must have been already whitelisted as a money market
    /// - `new_ush_market_observer` must not have been already used as a USH market observer
    ///
    #[endpoint(setUshMarketObserver)]
    fn set_ush_market_observer(&self, new_ush_market_observer: &ManagedAddress) {
        self.require_admin();
        self.require_whitelisted_money_market(new_ush_market_observer);
        require!(self.is_ush_market_sc(new_ush_market_observer), ERROR_INVALID_USH_MARKET_SC);
        require!(self.ush_market_observer().is_empty(), ERROR_USH_MARKET_OBSERVER_ALREADY_SET);
        require!(!self.historical_observers(new_ush_market_observer).get(), ERROR_LEGACY_USH_MARKET_OBSERVER);
        self.ush_market_observer().set(new_ush_market_observer);
        self.historical_observers(new_ush_market_observer).set(true);
        self.set_ush_market_observer_event(new_ush_market_observer);
    }

    /// Clears the USH Market smart contract from being an observer. From this point onwards, this smart contract will not be
    /// notified of any market change.
    ///
    #[endpoint(clearUshMarketObserver)]
    fn clear_ush_market_observer(&self) {
        self.require_admin();

        let ush_market_observer_mapper = self.ush_market_observer();
        require!(!ush_market_observer_mapper.is_empty(), ERROR_USH_MARKET_OBSERVER_UNSET);

        let old_ush_market_observer = ush_market_observer_mapper.take();
        require!(self.is_finalized(&old_ush_market_observer), ERROR_USH_MARKET_NOT_FINALIZED);

        self.clear_ush_market_observer_event(&old_ush_market_observer);
    }
}
