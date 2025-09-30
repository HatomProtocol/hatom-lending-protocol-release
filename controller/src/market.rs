multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::{errors::*, events, guardian, policies, proxies, rewards, risk_profile, shared, storage};

pub type ExitMarketAndRedeemResultType<BigUint> = MultiValue3<EgldOrEsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[multiversx_sc::module]
pub trait MarketModule: admin::AdminModule + events::EventModule + guardian::GuardianModule + policies::PolicyModule + proxies::ProxyModule + rewards::RewardsModule + risk_profile::RiskProfileModule + shared::SharedModule + storage::StorageModule {
    /// Payable endpoint used to enter to a one or many markets, i.e. provide collateral for sender liquidity calculations.
    /// The sender can perform multiple calls to keep adding more collateral.
    ///
    /// # Arguments:
    ///
    /// - `opt_account` - If given, the collateral will be deposited on the name of this account. Can only be performed by a
    ///   whitelisted money market.
    ///
    /// # Notes:
    ///
    /// - Must be paid with one or many valid ESDT Hatom tokens
    ///
    #[payable("*")]
    #[endpoint(enterMarkets)]
    fn enter_markets(&self, opt_account: OptionalValue<ManagedAddress>) {
        let account = match opt_account {
            OptionalValue::None => self.blockchain().get_caller(),
            OptionalValue::Some(account) => {
                let caller = self.blockchain().get_caller();
                require!(caller != account, ERROR_ADDRESSES_MUST_DIFFER);
                self.require_whitelisted_money_market(&caller);
                account
            },
        };
        let payments = self.call_value().all_esdt_transfers();
        for payment in payments.iter() {
            self.enter_market(&account, payment);
        }
    }

    fn enter_market(&self, account: &ManagedAddress, payment: EsdtTokenPayment) {
        let (token_id, _, amount) = payment.into_tuple();

        require!(self.is_whitelisted_token_id(&token_id), ERROR_NON_WHITELISTED_MARKET);
        require!(amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);

        let money_market = self.money_markets(&token_id).get();
        if let Some(ush_market) = self.get_ush_market_observer() {
            require!(money_market != ush_market, ERROR_INVALID_COLLATERAL);
        }

        self.update_supply_rewards_batches_state(&money_market);
        self.distribute_supplier_batches_rewards(&money_market, account);

        self.enter_market_internal(&money_market, account, &amount);
    }

    /// Exits a given amount of tokens from a given money market, i.e. removes the caller's deposited collateral for
    /// liquidity computations. If the amount of tokens is not specified, all the position is removed.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `opt_tokens` - If given, the amount of collateral tokens to remove.
    ///
    /// # Notes:
    ///
    /// - The provided address must be a whitelisted money market.
    /// - The caller must have collateral in the corresponding money market.
    /// - The amount of tokens to withdraw should not exceed the current deposited amount.
    /// - The caller must be providing the necessary collateral for any outstanding borrows.
    ///
    #[endpoint(exitMarket)]
    fn exit_market(&self, money_market: ManagedAddress, opt_tokens: OptionalValue<BigUint>) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        self.exit_market_internal(&money_market, &caller, opt_tokens, true)
    }

    /// Exits a given amount of tokens from a given money market, i.e. removes the caller's deposited collateral for liquidity
    /// computations and redeems the corresponding amount of tokens.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `opt_tokens` - If given, the amount of collateral tokens to remove.
    /// - `opt_underlying_amount` - An optional amount of underlying asset to receive back in exchange for the paid Hatom's
    ///   tokens.
    ///
    /// # Notes:
    ///
    /// - The provided address must be a whitelisted money market.
    /// - The caller must have collateral in the corresponding money market.
    /// - The amount of tokens to withdraw should not exceed the current deposited amount.
    ///
    #[endpoint(exitMarketAndRedeem)]
    fn exit_market_and_redeem(&self, money_market: &ManagedAddress, opt_tokens: Option<BigUint>, opt_underlying_amount: Option<BigUint>) -> ExitMarketAndRedeemResultType<Self::Api> {
        let redeemer = self.blockchain().get_caller();
        let token_payment_in = self.exit_market_internal(money_market, &redeemer, OptionalValue::from(opt_tokens), false);

        // redeem tokens
        let (underlying_payment, token_payment_burn) = self.redeem(money_market, &token_payment_in, opt_underlying_amount).into_tuple();

        // return the remaining token payment to the caller
        if token_payment_in.amount > token_payment_burn.amount {
            let (token_id, _, amount_eff) = token_payment_burn.clone().into_tuple();
            let amount_left = &token_payment_in.amount - &amount_eff;
            let token_payment_out = EsdtTokenPayment::new(token_id, 0, amount_left);
            self.send().direct_esdt(&redeemer, &token_payment_out.token_identifier, 0, &token_payment_out.amount);
        }

        // send underlying to redeemer
        let (underlying_id, _, underlying_amount) = underlying_payment.clone().into_tuple();
        self.send().direct(&redeemer, &underlying_id, 0, &underlying_amount);

        // this event is useful because the redeemer has been registered as the controller at the money market
        self.exit_market_and_redeem_event(money_market, &redeemer, &underlying_payment, &token_payment_burn);

        (underlying_payment, token_payment_in, token_payment_burn).into()
    }

    fn exit_market_internal(&self, money_market: &ManagedAddress, caller: &ManagedAddress, opt_tokens: OptionalValue<BigUint>, send: bool) -> EsdtTokenPayment {
        self.require_whitelisted_money_market(&money_market);

        // `market_members` storage could be also used for this type of check, but this seems much safer
        let account_collateral_tokens_mapper = self.account_collateral_tokens(&money_market, &caller);
        let old_tokens = account_collateral_tokens_mapper.get();
        require!(old_tokens > BigUint::zero(), ERROR_NO_COLLATERAL);

        let exit_tokens = match opt_tokens {
            OptionalValue::None => old_tokens.clone(),
            OptionalValue::Some(tokens) => {
                require!(tokens > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);
                require!(tokens <= old_tokens, ERROR_INSUFFICIENT_COLLATERAL);
                tokens
            },
        };

        // check protocol balance
        let (_, token_id) = self.identifiers(&money_market).get();
        let sc_address = self.blockchain().get_sc_address();
        require!(self.blockchain().get_esdt_balance(&sc_address, &token_id, 0) >= exit_tokens, ERROR_INSUFFICIENT_BALANCE);

        // check risk profile
        require!(self.redeem_allowed(&money_market, &caller, &exit_tokens), ERROR_REQUESTER_RISKY_OR_INSOLVENT);

        // update account collateral tokens
        account_collateral_tokens_mapper.update(|tokens| *tokens -= &exit_tokens);

        // update total collateral tokens
        self.total_collateral_tokens(&money_market).update(|tokens| *tokens -= &exit_tokens);

        // remove account from market if user does not hold collateral and does not hold an outstanding borrow
        self.remove_account_market_internal(&money_market, &caller);

        // send tokens to caller
        if send {
            self.send().direct_esdt(&caller, &token_id, 0, &exit_tokens);
        }

        // notify observers there has been a change in this market
        self.notify_market_observers(&money_market, &caller, &old_tokens);

        self.exit_market_event(&money_market, &caller, &exit_tokens);

        EsdtTokenPayment::new(token_id, 0, exit_tokens)
    }

    /// Removes an account from the given money market when the account has no collateral and no outstanding borrow in the
    /// given money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `opt_account` - If given, the address of the account to remove. If not given, the caller's address is used.
    ///
    #[endpoint(removeAccountMarket)]
    fn remove_account_market(&self, money_market: &ManagedAddress, opt_account: &OptionalValue<ManagedAddress>) {
        let account = match opt_account {
            OptionalValue::None => self.blockchain().get_caller(),
            OptionalValue::Some(account) => account.clone(),
        };
        self.remove_account_market_internal(money_market, &account);
    }

    fn remove_account_market_internal(&self, money_market: &ManagedAddress, account: &ManagedAddress) {
        let (underlying_owed, _) = self.get_account_snapshot(money_market, account);
        let tokens = self.get_account_collateral_tokens(money_market, account);
        if tokens == BigUint::zero() && underlying_owed == BigUint::zero() {
            self.account_markets(account).swap_remove(money_market);
            self.market_members(money_market).swap_remove(account);
        }
    }
}
