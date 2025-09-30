multiversx_sc::imports!();

use super::{constants::*, errors::*, events, proxies, shared, storage};

use crate::storage::{MarketType, RewardsBatch};

#[multiversx_sc::module]
pub trait RewardsModule: admin::AdminModule + events::EventModule + proxies::ProxyModule + shared::SharedModule + storage::StorageModule {
    /// Updates rewards batches states.
    ///
    /// # Arguments:
    ///
    /// - `supply` - Whether or not to update supply rewards.
    /// - `borrow` - Whether or not to update borrow rewards..
    /// - `money_markets` - The money market addresses to update rewards in. If empty, all whitelisted markets will be used.
    ///
    #[endpoint(updateRewardsBatchesState)]
    fn update_rewards_batches_state(&self, supply: bool, borrow: bool, money_markets: ManagedVec<ManagedAddress>) {
        let markets = self.validate_money_markets(money_markets);

        for money_market in markets.iter() {
            self.require_whitelisted_money_market(&money_market);

            if supply {
                self.update_supply_rewards_batches_state(&money_market);
            }

            if borrow {
                self.update_borrow_rewards_batches_state(&money_market);
            }
        }
    }

    /// Distributes caller or specified accounts rewards from supply and/or borrow markets, at specific money markets.
    ///
    /// # Arguments:
    ///
    /// - `supply` - Whether or not to distribute supply rewards.
    /// - `borrow` - Whether or not to distribute borrow rewards.
    /// - `money_markets` - The money market addresses to distribute rewards in. If empty, all whitelisted markets will be
    ///   used.
    /// - `accounts` - The addresses to distribute rewards for. If empty, the caller will be used.
    ///
    #[endpoint(distributeRewards)]
    fn distribute_rewards(&self, supply: bool, borrow: bool, money_markets: ManagedVec<ManagedAddress>, accounts: ManagedVec<ManagedAddress>) {
        let markets = self.validate_money_markets(money_markets);

        let accounts = if accounts.is_empty() {
            let caller = self.blockchain().get_caller();
            ManagedVec::from_single_item(caller)
        } else {
            accounts
        };

        self.distribute_rewards_internal(supply, borrow, &markets, &accounts);
    }

    fn distribute_rewards_internal(&self, supply: bool, borrow: bool, money_markets: &ManagedVec<ManagedAddress>, accounts: &ManagedVec<ManagedAddress>) {
        for money_market in money_markets.iter() {
            // updates borrow rewards batches states and distributes rewards for all accounts
            if borrow {
                self.update_borrow_rewards_batches_state(&money_market);
                for account in accounts.iter() {
                    self.distribute_borrower_batches_rewards(&money_market, &account);
                }
            }

            // updates supply rewards batches states and distributes rewards for all accounts
            if supply {
                self.update_supply_rewards_batches_state(&money_market);
                for account in accounts.iter() {
                    self.distribute_supplier_batches_rewards(&money_market, &account);
                }
            }
        }
    }

    /// Claims caller or specified accounts rewards from supply and/or borrow markets, at specific money markets.
    ///
    /// # Arguments:
    ///
    /// - `boost` - Whether or not to boost rewards whenever possible.
    /// - `supply` - Whether or not to claim supply rewards.
    /// - `borrow` - Whether or not to claim borrow rewards.
    /// - `money_markets` - The money market addresses to claim rewards in. If empty, all whitelisted markets will be used.
    /// - `accounts` - The addresses to claim rewards for. If empty, the caller will be used.
    /// - `opt_min_boosted_rewards_out`: An optional minimum amount of boosted rewards out.
    ///
    #[endpoint(claimRewards)]
    fn claim_rewards(&self, boost: bool, supply: bool, borrow: bool, money_markets: ManagedVec<ManagedAddress>, accounts: ManagedVec<ManagedAddress>, opt_min_boosted_rewards_out: OptionalValue<BigUint>) -> MultiValueEncoded<MultiValue2<ManagedAddress, EgldOrEsdtTokenPayment>> {
        let markets = self.validate_money_markets(money_markets);

        let accounts = if accounts.is_empty() {
            let caller = self.blockchain().get_caller();
            ManagedVec::from_single_item(caller)
        } else {
            self.require_admin_or_rewards_manager();
            require!(!boost, ERROR_BOOST_NOT_ALLOWED);
            accounts
        };

        self.claim_rewards_internal(boost, supply, borrow, &markets, &accounts, &opt_min_boosted_rewards_out)
    }

    /// Claim accrued rewards for several holders coming from specified markets, whether they have been earned by supplying
    /// and/or borrowing
    ///
    /// # Arguments:
    ///
    /// - `boost` - Whether or not to boost rewards whenever possible.
    /// - `supply` - Whether or not to claim rewards earned by supplying.
    /// - `borrow` - Whether or not to claim rewards earned by borrowing.
    /// - `money_markets` - The money market addresses to claim rewards in.
    /// - `accounts` - The addresses to claim rewards for.
    /// - `opt_min_boosted_rewards_out`: An optional minimum amount of boosted rewards out.
    ///
    fn claim_rewards_internal(&self, boost: bool, supply: bool, borrow: bool, money_markets: &ManagedVec<ManagedAddress>, accounts: &ManagedVec<ManagedAddress>, opt_min_boosted_rewards_out: &OptionalValue<BigUint>) -> MultiValueEncoded<MultiValue2<ManagedAddress, EgldOrEsdtTokenPayment>> {
        // first, distribute rewards to all accounts
        self.distribute_rewards_internal(supply, borrow, money_markets, accounts);

        // then, claim rewards to all accounts
        let mut payments_out = MultiValueEncoded::new();
        let mut boosted_rewards_eff = BigUint::zero();
        for money_market in money_markets.iter() {
            // send all rewards tokens to all accounts
            let rewards_batches = self.rewards_batches(&money_market);

            for account in accounts.iter() {
                for rewards_batch in rewards_batches.iter() {
                    let rewards_token_id = &rewards_batch.token_id;
                    let sc_balance = self.blockchain().get_sc_balance(rewards_token_id, 0);
                    let rewards = self.get_account_accrued_rewards(&account, rewards_token_id);

                    // don't do anything if rewards are zero
                    if rewards == BigUint::zero() {
                        continue;
                    }

                    // should be enough balance left in the contract, otherwise fail (should not happen)
                    require!(rewards <= sc_balance, ERROR_INSUFFICIENT_REWARDS_BALANCE);

                    // if boost is enabled and there is a booster, then boost the rewards
                    let booster_mapper = self.rewards_booster(rewards_token_id);
                    if boost && !booster_mapper.is_empty() {
                        let mut booster = booster_mapper.get();

                        let wad = BigUint::from(WAD);
                        let delta_rewards = &rewards * &booster.premium / &wad;

                        // if there is no sufficient amount, don't boost, don't fail and send non boosted rewards
                        if delta_rewards > booster.amount_left {
                            // tracks rewards batch only
                            self.send().direct(&account, rewards_token_id, 0, &rewards);
                            self.account_accrued_rewards(&account, rewards_token_id).set(&BigUint::zero());
                            self.rewards_claimed_event(&account, &rewards_batch, &rewards);

                            payments_out.push((account.clone_value(), EgldOrEsdtTokenPayment::new(rewards_token_id.clone(), 0, rewards)).into());

                            continue;
                        }

                        // should be enough balance left in the contract, otherwise fail (should not happen)
                        let boosted_rewards = &rewards + &delta_rewards;
                        require!(boosted_rewards <= sc_balance, ERROR_INSUFFICIENT_BOOSTED_REWARDS_BALANCE);

                        booster.distributed_amount += &delta_rewards;
                        booster.amount_left -= &delta_rewards;
                        booster_mapper.set(&booster);

                        self.boosted_rewards_claimed_event(&account, &booster, &delta_rewards);

                        // if rewards token is EGLD then add a EGLD => WEGLD step first
                        let swap_token_id = if rewards_token_id.is_egld() {
                            self.wrap_egld(&boosted_rewards);
                            self.wegld_id().get()
                        } else {
                            rewards_token_id.clone().unwrap_esdt()
                        };

                        // swap rewards batch tokens into governance token
                        let governance_token_id = self.governance_token_id().get();
                        let rewards_eff = self.custom_swap(&booster.swap_path, true, &swap_token_id, &boosted_rewards, &governance_token_id);

                        boosted_rewards_eff += &rewards_eff;

                        self.send().direct_esdt(&account, &governance_token_id, 0, &rewards_eff);

                        payments_out.push((account.clone_value(), EgldOrEsdtTokenPayment::new(EgldOrEsdtTokenIdentifier::esdt(governance_token_id), 0, rewards_eff)).into());
                    } else {
                        self.send().direct(&account, rewards_token_id, 0, &rewards);

                        payments_out.push((account.clone_value(), EgldOrEsdtTokenPayment::new(rewards_token_id.clone(), 0, rewards.clone())).into());
                    }

                    // tracks rewards coming from batches only, not from boosters
                    self.account_accrued_rewards(&account, rewards_token_id).set(&BigUint::zero());
                    self.rewards_claimed_event(&account, &rewards_batch, &rewards);
                }
            }
        }

        if let Some(min_boosted_rewards_out) = opt_min_boosted_rewards_out.clone().into_option() {
            require!(boost, ERROR_UNEXPECTED_MIN_AMOUNT_OUT);
            require!(boosted_rewards_eff >= min_boosted_rewards_out, ERROR_MIN_AMOUNT_OUT_NOT_REACHED);
        }

        payments_out
    }

    /// Sends all rewards from all rewards batches for the given money markets to the given account.
    ///
    /// # Arguments:
    ///
    /// - `boost`: Whether to boost the rewards or not.
    /// - `supply` - Whether or not to claim supply rewards.
    /// - `borrow` - Whether or not to claim borrow rewards.
    /// - `tokens`: An array of rewards tokens.
    /// - `money_markets`: An array of money market addresses in which the rewards distribution will be done.
    /// - `accounts`: An array of account addresses.
    /// - `opt_min_boosted_rewards_out`: An optional minimum amount of boosted rewards out.
    ///
    /// # Notes:
    ///
    /// - If `boost` is enabled, then the rewards will be boosted using the rewards booster.
    /// - If no money markets are specified, then all whitelisted money markets will be used.
    /// - If a provided money market does not have any batch for the rewards tokens, then it will be ignored.
    /// - If no accounts are provided, then only the caller will claim his rewards.
    ///
    #[endpoint(claimRewardsTokens)]
    fn claim_rewards_tokens(&self, boost: bool, supply: bool, borrow: bool, tokens: ManagedVec<EgldOrEsdtTokenIdentifier>, money_markets: ManagedVec<ManagedAddress>, accounts: ManagedVec<ManagedAddress>, opt_min_boosted_rewards_out: OptionalValue<BigUint>) -> MultiValueEncoded<MultiValue2<ManagedAddress, EgldOrEsdtTokenPayment>> {
        let markets = self.validate_money_markets(money_markets);

        let accounts = if accounts.is_empty() {
            let caller = self.blockchain().get_caller();
            ManagedVec::from_single_item(caller)
        } else {
            self.require_admin_or_rewards_manager();
            require!(!boost, ERROR_BOOST_NOT_ALLOWED);
            accounts
        };

        self.claim_rewards_tokens_internal(boost, supply, borrow, &tokens, &markets, &accounts, &opt_min_boosted_rewards_out)
    }

    fn claim_rewards_tokens_internal(&self, boost: bool, supply: bool, borrow: bool, tokens: &ManagedVec<EgldOrEsdtTokenIdentifier>, money_markets: &ManagedVec<ManagedAddress>, accounts: &ManagedVec<ManagedAddress>, opt_min_boosted_rewards_out: &OptionalValue<BigUint>) -> MultiValueEncoded<MultiValue2<ManagedAddress, EgldOrEsdtTokenPayment>> {
        // filter out money markets that don't have any of the tokens
        let mut filtered_markets: ManagedVec<ManagedAddress> = ManagedVec::new();
        for market in money_markets.into_iter() {
            if tokens.iter().any(|token| self.market_has_token_rewards_batch(&market, &token)) {
                filtered_markets.push(market);
            }
        }

        require!(!filtered_markets.is_empty(), ERROR_INVALID_REWARDS_TOKEN_IDS);

        // first, distribute rewards to all accounts
        self.distribute_rewards_internal(supply, borrow, &filtered_markets, accounts);

        let mut payments_out = MultiValueEncoded::new();
        let mut boosted_rewards_eff = BigUint::zero();
        for account in accounts.iter() {
            for rewards_token_id in tokens.iter() {
                let sc_balance = self.blockchain().get_sc_balance(&rewards_token_id, 0);
                let rewards = self.get_account_accrued_rewards(&account, &rewards_token_id);

                // don't do anything if rewards are zero
                if rewards == BigUint::zero() {
                    continue;
                }

                // should be enough balance left in the contract, otherwise fail (should not happen)
                require!(rewards <= sc_balance, ERROR_INSUFFICIENT_REWARDS_BALANCE);

                if boost {
                    let booster_mapper = self.rewards_booster(&rewards_token_id);

                    // should be a booster for this token, otherwise fail
                    require!(!booster_mapper.is_empty(), ERROR_TOKEN_NOT_BOOSTED);

                    let mut booster = booster_mapper.get();

                    // should be enough balance left in the booster, otherwise fail
                    let wad = BigUint::from(WAD);
                    let delta_rewards = &rewards * &booster.premium / &wad;
                    require!(booster.amount_left >= delta_rewards, ERROR_INSUFFICIENT_BOOSTED_REWARDS_BALANCE_LEFT);

                    // should be enough balance left in the contract, otherwise fail (should not happen)
                    let boosted_rewards = &rewards + &delta_rewards;
                    require!(boosted_rewards <= sc_balance, ERROR_INSUFFICIENT_BOOSTED_REWARDS_BALANCE);

                    booster.distributed_amount += &delta_rewards;
                    booster.amount_left -= &delta_rewards;
                    booster_mapper.set(&booster);

                    self.boosted_rewards_claimed_event(&account, &booster, &delta_rewards);

                    // if rewards token is EGLD then add a EGLD => WEGLD step first
                    let swap_token_id = if rewards_token_id.is_egld() {
                        self.wrap_egld(&boosted_rewards);
                        self.wegld_id().get()
                    } else {
                        rewards_token_id.clone().unwrap_esdt()
                    };

                    // swap rewards batch tokens into stake token
                    let governance_token_id = self.governance_token_id().get();
                    let rewards_eff = self.custom_swap(&booster.swap_path, true, &swap_token_id, &boosted_rewards, &governance_token_id);

                    boosted_rewards_eff += &rewards_eff;

                    self.send().direct_esdt(&account, &governance_token_id, 0, &rewards_eff);

                    payments_out.push((account.clone_value(), EgldOrEsdtTokenPayment::new(EgldOrEsdtTokenIdentifier::esdt(governance_token_id), 0, rewards_eff)).into());
                } else {
                    self.send().direct(&account, &rewards_token_id, 0, &rewards);

                    payments_out.push((account.clone_value(), EgldOrEsdtTokenPayment::new(rewards_token_id.clone(), 0, rewards.clone())).into());
                }

                // tracks rewards coming from batches only, not from boosters
                self.account_accrued_rewards(&account, &rewards_token_id).set(&BigUint::zero());

                self.rewards_token_claimed_event(&account, &rewards_token_id, &rewards);
            }
        }

        // should return at least the minimum amount, otherwise fail
        if let Some(min_boosted_rewards_out) = opt_min_boosted_rewards_out.clone().into_option() {
            require!(boost, ERROR_UNEXPECTED_MIN_AMOUNT_OUT);
            require!(boosted_rewards_eff >= min_boosted_rewards_out, ERROR_MIN_AMOUNT_OUT_NOT_REACHED);
        }

        payments_out
    }

    /// Updates the supply rewards batches state for the specified money market. In other words, it advances the rewards
    /// batch index (its "share price") one time step.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market to update the supply rewards batches state for.
    ///
    fn update_supply_rewards_batches_state(&self, money_market: &ManagedAddress) {
        // for exponential math
        let wad = BigUint::from(WAD);

        // the amount of Hatom tokens deposited as collateral
        let total_collateral_tokens = self.get_total_collateral_tokens(money_market);

        // get current timestamp
        let t = self.blockchain().get_block_timestamp();

        // compute rewards from all rewards batches
        let mut rewards_batches = self.rewards_batches(money_market);

        for pos_id in 1..=rewards_batches.len() {
            let mut rewards_batch = rewards_batches.get(pos_id);

            if rewards_batch.market_type != MarketType::Supply {
                continue;
            }

            if rewards_batch.last_time == rewards_batch.end_time || t == rewards_batch.last_time {
                continue;
            }

            let dt = if t > rewards_batch.end_time {
                let dt = rewards_batch.end_time - rewards_batch.last_time;
                rewards_batch.last_time = rewards_batch.end_time;
                dt
            } else {
                let dt = t - rewards_batch.last_time;
                rewards_batch.last_time = t;
                dt
            };

            if rewards_batch.speed > BigUint::zero() {
                let rewards_accrued = &rewards_batch.speed * dt; // [wad]
                if total_collateral_tokens == BigUint::zero() {
                    let delta_rewards = rewards_accrued / &wad;
                    rewards_batch.distributed_amount += &delta_rewards;
                    self.undistributed_rewards(&rewards_batch.token_id).update(|rewards| *rewards += &delta_rewards);
                } else {
                    let delta_index = &rewards_accrued * &wad / &total_collateral_tokens; // [wad * wad]

                    if delta_index != BigUint::zero() {
                        rewards_batch.index += delta_index;
                    } else {
                        let delta_rewards = rewards_accrued / &wad;
                        self.undistributed_rewards(&rewards_batch.token_id).update(|rewards| *rewards += &delta_rewards);
                    }
                }
            }

            rewards_batches.set(pos_id, &rewards_batch);

            self.supply_rewards_batches_updated_event(&rewards_batch);
        }
    }

    /// Updates the borrow rewards batches state for the specified money market. In other words, it advances the rewards
    /// batch index (its "share price") one time step.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market to update the borrow rewards batches state for.
    ///
    fn update_borrow_rewards_batches_state(&self, money_market: &ManagedAddress) {
        // for exponential math
        let wad = BigUint::from(WAD);

        // in most cases, this is the total borrows discounted to the money market inception
        let base_total_borrows = self.get_base_total_borrows(money_market);

        // get current timestamp
        let t = self.blockchain().get_block_timestamp();

        // compute rewards from all rewards batches
        let mut rewards_batches = self.rewards_batches(money_market);

        for pos_id in 1..=rewards_batches.len() {
            let mut rewards_batch = rewards_batches.get(pos_id);

            if rewards_batch.market_type != MarketType::Borrow {
                continue;
            }

            if rewards_batch.last_time == rewards_batch.end_time || t == rewards_batch.last_time {
                continue;
            }

            let dt = if t > rewards_batch.end_time {
                let dt = rewards_batch.end_time - rewards_batch.last_time;
                rewards_batch.last_time = rewards_batch.end_time;
                dt
            } else {
                let dt = t - rewards_batch.last_time;
                rewards_batch.last_time = t;
                dt
            };

            if rewards_batch.speed > BigUint::zero() {
                let rewards_accrued = &rewards_batch.speed * dt; // [wad]
                if base_total_borrows == BigUint::zero() {
                    let delta_rewards = rewards_accrued / &wad;
                    rewards_batch.distributed_amount += &delta_rewards;
                    self.undistributed_rewards(&rewards_batch.token_id).update(|rewards| *rewards += &delta_rewards);
                } else {
                    let delta_index = &rewards_accrued * &wad / (&base_total_borrows + 1u64); // [wad * wad]

                    if delta_index != BigUint::zero() {
                        rewards_batch.index += delta_index;
                    } else {
                        let delta_rewards = rewards_accrued / &wad;
                        self.undistributed_rewards(&rewards_batch.token_id).update(|rewards| *rewards += &delta_rewards);
                    }
                }
            }

            rewards_batches.set(pos_id, &rewards_batch);

            self.borrow_rewards_batches_updated_event(&rewards_batch);
        }
    }

    /// Distributes rewards to a supplier for all applicable rewards batches.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market to distribute rewards for.
    /// - `supplier` - The address of the supplier to distribute rewards to.
    ///
    fn distribute_supplier_batches_rewards(&self, money_market: &ManagedAddress, supplier: &ManagedAddress) {
        // for exponential math
        let wad = BigUint::from(WAD);
        let wad_wad = &wad * &wad;

        // rewards are computed only based on the amount of hatom tokens that are deposited as collateral
        let account_collateral_tokens = self.get_account_collateral_tokens(money_market, supplier);

        let mut rewards_batches = self.rewards_batches(money_market);

        for pos_id in 1..=rewards_batches.len() {
            let mut rewards_batch = rewards_batches.get(pos_id);

            if rewards_batch.market_type != MarketType::Supply {
                continue;
            }

            let RewardsBatch { id: batch_id, token_id: rewards_token_id, index: rewards_index, .. } = &rewards_batch;

            let supplier_index = match self.get_account_batch_rewards_index(money_market, batch_id, supplier) {
                None => &wad * &wad,
                Some(index) => index,
            };

            self.account_batch_rewards_index(money_market, batch_id, supplier).set(rewards_index);

            let delta_index = rewards_index - &supplier_index;
            let delta_rewards = &account_collateral_tokens * &delta_index / &wad_wad;

            self.account_accrued_rewards(supplier, rewards_token_id).update(|rewards| *rewards += &delta_rewards);

            // update batch state
            rewards_batch.distributed_amount += &delta_rewards;
            rewards_batches.set(pos_id, &rewards_batch);

            self.supplier_rewards_distributed_event(supplier, &rewards_batch, &delta_rewards);
        }
    }

    /// Distributes rewards to a borrower for all applicable rewards batches.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market to distribute rewards for.
    /// - `borrower` - The address of the borrower to distribute rewards to.
    /// - `market_borrow_index` - The current borrow index for the money market.
    ///
    fn distribute_borrower_batches_rewards(&self, money_market: &ManagedAddress, borrower: &ManagedAddress) {
        // for exponential math
        let wad = BigUint::from(WAD);
        let wad_wad = &wad * &wad;

        // in most cases, this is the account borrows discounted to the money market inception
        let base_account_borrow_amount = self.get_base_account_borrow_amount(money_market, borrower);

        let mut rewards_batches = self.rewards_batches(money_market);

        for pos_id in 1..=rewards_batches.len() {
            let mut rewards_batch = rewards_batches.get(pos_id);

            // only borrow rewards batches
            if rewards_batch.market_type != MarketType::Borrow {
                continue;
            }

            let RewardsBatch { id: batch_id, token_id: rewards_token_id, index: rewards_index, .. } = &rewards_batch;

            let borrower_index = match self.get_account_batch_rewards_index(money_market, batch_id, borrower) {
                None => &wad * &wad,
                Some(index) => index,
            };

            self.account_batch_rewards_index(money_market, batch_id, borrower).set(rewards_index);

            let delta_index = rewards_index - &borrower_index;
            let delta_rewards = &base_account_borrow_amount * &delta_index / &wad_wad;

            self.account_accrued_rewards(borrower, rewards_token_id).update(|rewards| *rewards += &delta_rewards);

            // update batch state
            rewards_batch.distributed_amount += &delta_rewards;
            rewards_batches.set(pos_id, &rewards_batch);

            self.borrower_rewards_distributed_event(borrower, &rewards_batch, &delta_rewards);
        }
    }
}
