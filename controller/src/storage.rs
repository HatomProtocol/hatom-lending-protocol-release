multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const SWAP_TOKENS_FIXED_INPUT_FUNC_NAME: &[u8] = b"swapTokensFixedInput";

pub type SwapOperationType<M> = MultiValue4<ManagedAddress<M>, ManagedBuffer<M>, TokenIdentifier<M>, BigUint<M>>;

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Clone, Copy, Debug)]
pub enum Status {
    Active,
    Paused,
}

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Clone, Copy, Debug)]
pub enum State {
    Inactive,
    Active,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct MarketState<M>
where
    M: ManagedTypeApi,
{
    pub timestamp: u64,
    pub index: BigUint<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq, ManagedVecItem)]
pub enum MarketType {
    Supply,
    Borrow,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct RewardsBatch<M>
where
    M: ManagedTypeApi,
{
    pub id: usize,
    pub money_market: ManagedAddress<M>,
    pub market_type: MarketType,
    pub token_id: EgldOrEsdtTokenIdentifier<M>,
    pub amount: BigUint<M>,
    pub distributed_amount: BigUint<M>,
    pub speed: BigUint<M>,
    pub index: BigUint<M>,
    pub last_time: u64,
    pub end_time: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct RewardsBooster<M>
where
    M: ManagedTypeApi,
{
    pub token_id: EgldOrEsdtTokenIdentifier<M>,
    pub premium: BigUint<M>,
    pub amount_left: BigUint<M>,
    pub distributed_amount: BigUint<M>,
    pub swap_path: ManagedVec<M, SwapStep<M>>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct SwapStep<M>
where
    M: ManagedTypeApi,
{
    pub pair_address: ManagedAddress<M>,
    pub input_token_id: TokenIdentifier<M>,
    pub output_token_id: TokenIdentifier<M>,
}

#[multiversx_sc::module]
pub trait StorageModule {
    /// Stores the guardian address.
    #[view(getPauseGuardian)]
    #[storage_mapper("pause_guardian")]
    fn pause_guardian(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the rewards manager address.
    #[view(getRewardsManager)]
    #[storage_mapper("rewards_manager")]
    fn rewards_manager(&self) -> SingleValueMapper<ManagedAddress>;

    /// Whitelisted markets, i.e. supported markets.
    #[storage_mapper("whitelisted_markets")]
    fn whitelisted_markets(&self) -> UnorderedSetMapper<Self::Api, ManagedAddress>;

    /// Stores a whitelisted market address given a token identifier.
    #[view(getMoneyMarketByTokenId)]
    #[storage_mapper("money_markets")]
    fn money_markets(&self, token_id: &TokenIdentifier) -> SingleValueMapper<ManagedAddress>;

    /// Stores both the underlying identifier and the token identifier associated to a whitelisted money market.
    #[view(getIdentifiersByMoneyMarket)]
    #[storage_mapper("identifiers")]
    fn identifiers(&self, money_market: &ManagedAddress) -> SingleValueMapper<(EgldOrEsdtTokenIdentifier, TokenIdentifier)>;

    /// Stores the set of money markets addresses in which an account has entered, i.e. deposited collateral or took a
    /// borrow.
    #[storage_mapper("account_markets")]
    fn account_markets(&self, account: &ManagedAddress) -> UnorderedSetMapper<ManagedAddress>;

    /// Stores the set of addresses that belong to a given money market.
    #[view(getMarketMembers)]
    #[storage_mapper("market_members")]
    fn market_members(&self, money_market: &ManagedAddress) -> UnorderedSetMapper<ManagedAddress>;

    /// Stores the maximum amount of markets an account can enter at any given point in time.
    #[view(getMaxMarketsPerAccount)]
    #[storage_mapper("max_markets_per_account")]
    fn max_markets_per_account(&self) -> SingleValueMapper<usize>;

    /// Stores the price oracle smart contract address.
    #[view(getPriceOracle)]
    #[storage_mapper("price_oracle")]
    fn price_oracle(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the collateral factor for each money market.
    #[view(getCollateralFactor)]
    #[storage_mapper("collateral_factor")]
    fn collateral_factor(&self, money_market: &ManagedAddress) -> SingleValueMapper<BigUint>;

    /// Stores the collateral factor for each money market taken into consideration if the account has borrowed USH.
    #[view(getUshBorrowerCollateralFactor)]
    #[storage_mapper("ush_borrower_collateral_factor")]
    fn ush_borrower_collateral_factor(&self, money_market: &ManagedAddress) -> SingleValueMapper<BigUint>;

    /// Stores the next collateral factors for each money market.
    #[view(getNextCollateralFactor)]
    #[storage_mapper("next_collateral_factors")]
    fn next_collateral_factors(&self, money_market: &ManagedAddress) -> SingleValueMapper<(u64, BigUint, BigUint)>;

    /// Stores the total collateral amount that a given account has deposited into a given money market.
    #[storage_mapper("account_collateral_tokens")]
    fn account_collateral_tokens(&self, money_market: &ManagedAddress, account: &ManagedAddress) -> SingleValueMapper<BigUint>;

    /// Stores the total collateral amount deposited into a given money market.
    #[storage_mapper("total_collateral_tokens")]
    fn total_collateral_tokens(&self, money_market: &ManagedAddress) -> SingleValueMapper<BigUint>;

    /// A supported money market might have a liquidity cap, which is stored here.
    #[view(getLiquidityCap)]
    #[storage_mapper("liquidity_cap")]
    fn liquidity_cap(&self, money_market: &ManagedAddress) -> SingleValueMapper<BigUint>;

    /// A supported money market might have a borrowing cap, which is stored here.
    #[view(getBorrowCap)]
    #[storage_mapper("borrow_cap")]
    fn borrow_cap(&self, money_market: &ManagedAddress) -> SingleValueMapper<BigUint>;

    /// Stores the mint status.
    #[storage_mapper("mint_status")]
    fn mint_status(&self, money_market: &ManagedAddress) -> SingleValueMapper<Status>;

    /// Stores the borrow status.
    #[storage_mapper("borrow_status")]
    fn borrow_status(&self, money_market: &ManagedAddress) -> SingleValueMapper<Status>;

    /// Stores the seize status.
    #[storage_mapper("seize_status")]
    fn seize_status(&self, money_market: &ManagedAddress) -> SingleValueMapper<Status>;

    /// Stores the global seize status.
    #[storage_mapper("global_seize_status")]
    fn global_seize_status(&self) -> SingleValueMapper<Status>;

    /// Stores the amount of rewards accrued by a given account for a given rewards token.
    #[storage_mapper("account_accrued_rewards")]
    fn account_accrued_rewards(&self, account: &ManagedAddress, rewards_token_id: &EgldOrEsdtTokenIdentifier) -> SingleValueMapper<BigUint>;

    /// Stores the rewards index for a given account and rewards token in the specified money market.
    #[view(getAccountRewardsIndex)]
    #[storage_mapper("account_rewards_index")]
    fn account_batch_rewards_index(&self, money_market: &ManagedAddress, batch_id: &usize, account: &ManagedAddress) -> SingleValueMapper<BigUint>;

    /// Stores the ID of the next rewards batch in the specified money market.
    #[view(getNextRewardsBatchId)]
    #[storage_mapper("next_rewards_batch_id")]
    fn next_rewards_batch_id(&self, money_market: &ManagedAddress) -> SingleValueMapper<usize>;

    /// Stores the maximum amount of batches allowed per market.
    #[view(getMaxRewardsBatchesPerMarket)]
    #[storage_mapper("max_rewards_batches_per_market")]
    fn max_rewards_batches(&self, money_market: &ManagedAddress) -> SingleValueMapper<usize>;

    /// Stores the maximum allowed slippage.
    #[view(getMaxSlippage)]
    #[storage_mapper("max_slippage")]
    fn max_slippage(&self) -> SingleValueMapper<BigUint>;

    /// Stores the list of rewards batches in the specified money market.
    #[view(getRewardsBatches)]
    #[storage_mapper("rewards_batches")]
    fn rewards_batches(&self, money_market: &ManagedAddress) -> VecMapper<RewardsBatch<Self::Api>>;

    /// Stores the undistributed rewards for a given rewards token identifier.
    #[view(getUndistributedRewards)]
    #[storage_mapper("undistributed_rewards")]
    fn undistributed_rewards(&self, token_id: &EgldOrEsdtTokenIdentifier) -> SingleValueMapper<BigUint>;

    /// Stores the current position of a rewards batch in the specified money market at the corresponding VecMapper.
    #[view(getRewardsBatchPosition)]
    #[storage_mapper("rewards_batch_position")]
    fn rewards_batch_position(&self, money_market: &ManagedAddress, batch_id: &usize) -> SingleValueMapper<usize>;

    /// Stores the rewards batch booster for a given rewards token identifier.
    #[view(getRewardsBooster)]
    #[storage_mapper("rewards_booster")]
    fn rewards_booster(&self, token_id: &EgldOrEsdtTokenIdentifier) -> SingleValueMapper<RewardsBooster<Self::Api>>;

    /// Stores wrapped EGLD smart contract address.
    #[view(getEgldWrapper)]
    #[storage_mapper("egld_wrapper")]
    fn egld_wrapper(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the token identifier of the wrapped EGLD token.
    #[view(getWegldId)]
    #[storage_mapper("wegld_id")]
    fn wegld_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores the governance token identifier.
    #[view(getGovernanceTokenId)]
    #[storage_mapper("governance_token_id")]
    fn governance_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores the xExchange Router address.
    #[view(getRouter)]
    #[storage_mapper("router")]
    fn router(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the boosting state.
    #[view(getBoostingState)]
    #[storage_mapper("boosting_state")]
    fn boosting_state(&self) -> SingleValueMapper<State>;

    /// Stores whether boosting is or not supported.
    #[view(isRewardsBatchBoostingSupported)]
    #[storage_mapper("rewards_batch_boosting_supported")]
    fn rewards_batch_boosting_supported(&self) -> SingleValueMapper<bool>;

    /// Stores the Rewards Booster smart contract address.
    #[view(getBoosterObserver)]
    #[storage_mapper("booster_observer")]
    fn booster_observer(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the USH Money Market observer.
    #[view(getUshMarketObserver)]
    #[storage_mapper("ush_market_observer")]
    fn ush_market_observer(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores historical observers smart contract addresses.
    #[storage_mapper("historical_observers")]
    fn historical_observers(&self, observer: &ManagedAddress) -> SingleValueMapper<bool>;
}
