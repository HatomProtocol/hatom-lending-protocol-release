multiversx_sc::imports!();
multiversx_sc::derive_imports!();

/// The money market state.
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq)]
pub enum State {
    Empty,
    Active,
    Inactive,
    Finalized,
}

pub enum InteractionType {
    Borrow,
    RepayBorrow,
    EnterOrExitMarket,
}

pub enum DiscountStrategy {
    PreviousDiscount,
    CachedExchangeRate,
    UpdatedExchangeRate,
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
    pub discount: BigUint<M>,
}

impl<M> AccountSnapshot<M>
where
    M: ManagedTypeApi,
{
    pub fn new(borrow_amount: &BigUint<M>, borrow_index: &BigUint<M>, discount: &BigUint<M>) -> Self {
        Self {
            borrow_amount: borrow_amount.clone(),
            borrow_index: borrow_index.clone(),
            discount: discount.clone(),
        }
    }
}

#[multiversx_sc::module]
pub trait StorageModule {
    /// Stores the smart contract state.
    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;

    /// Stores the Controller address.
    #[storage_mapper("controller")]
    fn controller(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the USH Minter address.
    #[view(getUshMinter)]
    #[storage_mapper("ush_minter")]
    fn ush_minter(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the USH token identifier.
    #[view(getUshId)]
    #[storage_mapper("ush_id")]
    fn ush_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores the HUSH token identifier.
    #[view(getHushId)]
    #[storage_mapper("hush_id")]
    fn hush_id(&self) -> SingleValueMapper<TokenIdentifier>;

    /// Stores whether HUSH is being issued or not.
    #[view(getOngoingIssuance)]
    #[storage_mapper("ongoing_issuance")]
    fn ongoing_issuance(&self) -> SingleValueMapper<bool>;

    /// Stores the borrow snapshot for a given borrower account.
    #[view(getAccountBorrowSnapshot)]
    #[storage_mapper("account_borrow_snapshot")]
    fn account_borrow_snapshot(&self, borrower: &ManagedAddress) -> SingleValueMapper<AccountSnapshot<Self::Api>>;

    /// Stores the account's principal.
    #[view(getAccountPrincipal)]
    #[storage_mapper("account_principal")]
    fn account_principal(&self, borrower: &ManagedAddress) -> SingleValueMapper<BigUint>;

    /// Stores the total amount of outstanding borrows up to the last accrue of interest.
    #[view(getTotalBorrows)]
    #[storage_mapper("total_borrows")]
    fn total_borrows(&self) -> SingleValueMapper<BigUint>;

    // Stores the total amount of outstanding borrows with discount up to the last accrue of interest.
    #[view(getEffectiveBorrows)]
    #[storage_mapper("effective_borrows")]
    fn effective_borrows(&self) -> SingleValueMapper<BigUint>;

    // Stores the total principal.
    #[view(getTotalPrincipal)]
    #[storage_mapper("total_principal")]
    fn total_principal(&self) -> SingleValueMapper<BigUint>;

    /// Stores the current amount of reserves.
    #[view(getTotalReserves)]
    #[storage_mapper("total_reserves")]
    fn total_reserves(&self) -> SingleValueMapper<BigUint>;

    /// Stores the total amount of staking rewards.
    #[view(getStakingRewards)]
    #[storage_mapper("staking_rewards")]
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

    /// Stores whether USH can be used as collateral or not.
    #[view(eligibleAsCollateral)]
    #[storage_mapper("eligible_as_collateral")]
    fn eligible_as_collateral(&self) -> SingleValueMapper<bool>;

    /// Stores the borrow rate per second.
    #[view(getBorrowRate)]
    #[storage_mapper("borrow_rate")]
    fn borrow_rate(&self) -> SingleValueMapper<BigUint>;

    /// Stores the last time the borrow rate was updated.
    #[view(getLastBorrowRateUpdate)]
    #[storage_mapper("last_borrow_rate_update")]
    fn last_borrow_rate_update(&self) -> SingleValueMapper<u64>;

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

    /// Stores the Staking Module smart contract address.
    #[view(getStakingSc)]
    #[storage_mapper("staking_sc")]
    fn staking_sc(&self) -> SingleValueMapper<ManagedAddress>;

    /// Stores the address of the Discount Rate Model.
    #[view(getDiscountRateModel)]
    #[storage_mapper("discount_rate_model")]
    fn discount_rate_model(&self) -> SingleValueMapper<ManagedAddress>;

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

    /// Stores the set of addresses with borrow.
    #[view(getMarketBorrowers)]
    #[storage_mapper("market_borrowers")]
    fn market_borrowers(&self) -> UnorderedSetMapper<ManagedAddress>;

    /// Stores a whitelist of trusted smart contracts that can mint and enter market on behalf of users.
    #[storage_mapper("trusted_minters_list")]
    fn trusted_minters_list(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;
}
