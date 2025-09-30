multiversx_sc::imports!();
multiversx_sc::derive_imports!();

/// The money market state.
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq)]
pub enum State {
    Empty,
    Active,
    Inactive,
}

/// Represents a snapshot of an account's borrow balance.
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct AccountSnapshot<M>
where
    M: ManagedTypeApi,
{
    pub borrow_amount: BigUint<M>,
    pub borrow_index: BigUint<M>,
}

#[multiversx_sc::module]
pub trait StorageModule {
    /// Stores the money market state.
    #[view(getState)]
    #[storage_mapper("market_state")]
    fn market_state(&self) -> SingleValueMapper<State>;

    /// Stores the underlying identifier.
    #[view(getUnderlyingId)]
    #[storage_mapper("underlying_id")]
    fn underlying_id(&self) -> SingleValueMapper<EgldOrEsdtTokenIdentifier>;

    /// Stores the token identifier.
    #[view(getTokenId)]
    #[storage_mapper("token_id")]
    fn token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores whether the token is being issued or not.
    #[view(getOngoingIssuance)]
    #[storage_mapper("ongoing_issuance")]
    fn ongoing_issuance(&self) -> SingleValueMapper<bool>;

    /// Stores whether the token initial supply has been minted or not.
    #[view(getMintedInitialSupply)]
    #[storage_mapper("minted_initial_supply")]
    fn minted_initial_supply(&self) -> SingleValueMapper<bool>;

    /// Stores the borrow snapshot for a given borrower account.
    #[view(getAccountBorrowSnapshot)]
    #[storage_mapper("account_borrow_snapshot")]
    fn account_borrow_snapshot(&self, borrower: &ManagedAddress) -> SingleValueMapper<AccountSnapshot<Self::Api>>;

    /// Stores the current balance of the underlying asset.
    #[view(getCash)]
    #[storage_mapper("cash")]
    fn cash(&self) -> SingleValueMapper<BigUint>;

    /// Stores the total amount of outstanding borrows up to the last accrue of interest.
    #[view(getTotalBorrows)]
    #[storage_mapper("total_borrows")]
    fn total_borrows(&self) -> SingleValueMapper<BigUint>;

    /// Stores the current amount of reserves.
    #[view(getTotalReserves)]
    #[storage_mapper("total_reserves")]
    fn total_reserves(&self) -> SingleValueMapper<BigUint>;

    /// Stores the total amount of staking rewards.
    #[view(getStakingRewards)]
    #[storage_mapper("total_staking_rewards")]
    fn staking_rewards(&self) -> SingleValueMapper<BigUint>;

    /// Stores the amount of historical staking rewards.
    #[view(getHistoricalStakingRewards)]
    #[storage_mapper("historical_staking_rewards")]
    fn historical_staking_rewards(&self) -> SingleValueMapper<BigUint>;

    /// Stores the amount of protocol revenue.
    #[view(getRevenue)]
    #[storage_mapper("revenue")]
    fn revenue(&self) -> SingleValueMapper<BigUint>;

    /// Stores the total supply of the token.
    #[view(getTotalSupply)]
    #[storage_mapper("total_supply")]
    fn total_supply(&self) -> SingleValueMapper<BigUint>;

    /// Stores the reserve factor used to calculate the protocol's earnings.
    #[storage_mapper("reserve_factor")]
    fn reserve_factor(&self) -> SingleValueMapper<BigUint>;

    /// Stores the staking factor used to calculate staking rewards.
    #[view(getStakeFactor)]
    #[storage_mapper("stake_factor")]
    fn stake_factor(&self) -> SingleValueMapper<BigUint>;

    /// Stores the timestamp of the last accrual.
    #[view(getAccrualTimestamp)]
    #[storage_mapper("accrual_timestamp")]
    fn accrual_timestamp(&self) -> SingleValueMapper<u64>;

    /// Stores the borrow index up to the last accrual of interest.
    #[storage_mapper("borrow_index")]
    fn borrow_index(&self) -> SingleValueMapper<BigUint>;

    /// Stores the address of the Controller.
    #[storage_mapper("controller")]
    fn controller(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the address of Staking Module.
    #[storage_mapper("staking_contract")]
    fn staking_contract(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the address of the Interest Rate Model.
    #[storage_mapper("interest_rate_model")]
    fn interest_rate_model(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the initial exchange rate between underlying and token, initialized at deployment.
    #[view(getInitialExchangeRate)]
    #[storage_mapper("initial_exchange_rate")]
    fn initial_exchange_rate(&self) -> SingleValueMapper<BigUint>;

    /// Stores the current close factor.
    #[storage_mapper("close_factor")]
    fn close_factor(&self) -> SingleValueMapper<BigUint>;

    /// Stores the current liquidation incentive.
    #[storage_mapper("liquidation_incentive")]
    fn liquidation_incentive(&self) -> SingleValueMapper<BigUint>;

    /// Stores the current protocol seize share.
    #[view(getProtocolSeizeShare)]
    #[storage_mapper("protocol_seize_share")]
    fn protocol_seize_share(&self) -> SingleValueMapper<BigUint>;

    /// Stores the accrual time threshold.
    #[view(getAccrualTimeThreshold)]
    #[storage_mapper("accrual_time_threshold")]
    fn accrual_time_threshold(&self) -> SingleValueMapper<u64>;

    /// Stores a whitelist of trusted smart contracts that can mint and enter market on behalf of users.
    #[storage_mapper("trusted_minters_list")]
    fn trusted_minters_list(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;
}
