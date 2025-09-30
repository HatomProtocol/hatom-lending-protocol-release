multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::{constants::*, events, proxies, shared, storage};

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Clone, Debug)]
pub enum RiskProfile<M: ManagedTypeApi> {
    Solvent(BigUint<M>),
    RiskyOrInsolvent(BigUint<M>), // implies either risk of insolvency or insolvent
}

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Clone, Copy, Debug)]
pub enum Liquidation {
    Allowed,
    NotAllowed,
    AllowedButTooMuch,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct AccountSnapshot<M: ManagedTypeApi> {
    money_market: ManagedAddress<M>,
    underlying_owed_amount: BigUint<M>,
    fx: BigUint<M>,
}

impl<M: ManagedTypeApi> RiskProfile<M> {
    pub fn can_redeem(&self) -> bool {
        matches!(*self, RiskProfile::Solvent(_))
    }

    pub fn can_borrow(&self) -> bool {
        self.can_redeem()
    }

    /// Given an account with an outstanding borrow, checks if it is possible to liquidate its position by a repayment
    /// amount, considering the allowed closing factor
    pub fn can_be_liquidated(&self, repay_amount: &BigUint<M>, borrow_amount: &BigUint<M>, close_factor: &BigUint<M>) -> Liquidation {
        match *self {
            RiskProfile::Solvent(_) => Liquidation::NotAllowed,
            RiskProfile::RiskyOrInsolvent(_) => {
                let wad = BigUint::from(WAD);
                let max_close_amount = close_factor * borrow_amount / wad;
                if repay_amount > &max_close_amount {
                    return Liquidation::AllowedButTooMuch;
                }
                Liquidation::Allowed
            },
        }
    }
}

#[multiversx_sc::module]
pub trait RiskProfileModule: admin::AdminModule + events::EventModule + proxies::ProxyModule + shared::SharedModule + storage::StorageModule {
    /// Checks whether an account is risky or not by computing its current risk profile.
    ///
    /// # Arguments:
    ///
    /// - `account` - The account we wish to analyze.
    ///
    #[endpoint(isRisky)]
    fn is_risky(&self, account: &ManagedAddress) -> bool {
        let risk_profile = self.simulate_risk_profile(account, &ManagedAddress::zero(), &BigUint::zero(), &BigUint::zero(), true);
        match risk_profile {
            RiskProfile::Solvent(_) => false,
            RiskProfile::RiskyOrInsolvent(_) => true,
        }
    }

    /// Performs a risk profile simulation for a given account, considering its current opened positions and simulating
    /// either redeeming or borrowing (or both) in a given money market. The money market for the simulation must be already
    /// included as an account market. Otherwise, the simulation will not be performed.
    ///
    /// # Arguments:
    ///
    /// - `account` - The account we wish to analyze.
    /// - `this_money_market` - The money market address used for the borrow or redeem simulation (or both).
    /// - `redeem_tokens` - The amount of Hatom tokens to be redeemed for underlying at `this_money_market`.
    /// - `borrow_amount` - The amount of underlying to be borrowed at `this_money_market`.
    /// - `lazy` - If true, the simulation will return a solvent risk profile with a dummy liquidity if the account is not a
    ///   borrower. If false, the simulation will be fully performed, even if it is not a borrower (i.e. Solvent by
    ///   definition).
    ///
    #[endpoint(simulateRiskProfile)]
    fn simulate_risk_profile(&self, account: &ManagedAddress, this_money_market: &ManagedAddress, redeem_tokens: &BigUint, borrow_amount: &BigUint, lazy: bool) -> RiskProfile<Self::Api> {
        // * Important: `account_markets` might not include `this_money_market`. If that is the case, the simulation will not
        // * be performed and the result will not be accurate.
        let account_markets = self.account_markets(account);

        // assume the account does not have any outstanding borrow
        let mut borrower = false;

        // assume the accounts is not a USH borrower
        let mut ush_borrower = false;
        let opt_ush_market = self.get_ush_market_observer();
        let ush_market = opt_ush_market.unwrap_or_default();

        let mut snapshots: ManagedVec<AccountSnapshot<Self::Api>> = ManagedVec::new();
        for money_market in account_markets.iter() {
            let (underlying_owed_amount, fx) = self.get_account_snapshot(&money_market, account);

            if underlying_owed_amount > BigUint::zero() {
                if money_market == ush_market {
                    ush_borrower = true;
                }
                borrower = true;
            }

            snapshots.push(AccountSnapshot { money_market, underlying_owed_amount, fx });
        }

        if borrow_amount > &BigUint::zero() {
            if this_money_market == &ush_market {
                ush_borrower = true;
            }
            borrower = true;
        }

        // if it is a lazy computation and the account is not a borrower, return a solvent risk profile with a dummy liquidity
        if lazy && !borrower {
            return RiskProfile::Solvent(BigUint::zero());
        }

        // for exponential math
        let wad = BigUint::from(WAD);

        // represent the total borrow and collateral expressed in a numeraire of our choice (EGLD) in wad
        let mut total_borrow = BigUint::zero();
        let mut total_collateral = BigUint::zero();

        for snapshot in snapshots.iter() {
            let AccountSnapshot { money_market, underlying_owed_amount, fx } = snapshot;

            // get loan to value and collateral
            let (collateral_factor, ush_borrower_collateral_factor) = self.update_and_get_collateral_factors(&money_market);
            let ltv = if !ush_borrower { collateral_factor } else { ush_borrower_collateral_factor };
            let collateral_tokens = self.get_account_collateral_tokens(&money_market, account);

            // get both the underlying and token prices in a numeraire of our choice (EGLD) in wad
            let underlying_price = self.get_underlying_price(&money_market);
            let token_price = &fx * &underlying_price / &wad;
            let token_price_eff = &ltv * &token_price / &wad;

            // accumulate collateral and borrow
            total_collateral += &token_price_eff * &collateral_tokens / &wad;
            total_borrow += &underlying_price * &underlying_owed_amount / &wad;

            // if we are trying to redeem or borrow from `this_money_market`, these are the effects: redeeming reduces
            // collateral and borrowing increases borrow
            if money_market == *this_money_market {
                // redeem effect: notice that addition to `total_borrows` is equivalent to subtraction to `total_collateral`
                total_borrow += token_price_eff * redeem_tokens / &wad;

                // borrow effect
                total_borrow += underlying_price * borrow_amount / &wad;
            }
        }

        if total_collateral >= total_borrow {
            let liquidity = total_collateral - total_borrow;
            RiskProfile::Solvent(liquidity)
        } else {
            let shortfall = total_borrow - total_collateral;
            RiskProfile::RiskyOrInsolvent(shortfall)
        }
    }
}
