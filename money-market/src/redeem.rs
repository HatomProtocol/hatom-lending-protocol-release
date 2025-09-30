multiversx_sc::imports!();

use super::{common, errors::*, events, proxies, storage};

pub type RedeemResultType<BigUint> = MultiValue2<EgldOrEsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[multiversx_sc::module]
pub trait RedeemModule: common::CommonModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Exchanges caller paid Hatom's tokens back for her underlying asset.
    ///
    /// # Arguments:
    ///
    /// - `opt_underlying_amount` - An optional amount of underlying asset to receive back in exchange for the paid Hatom's
    ///   tokens.
    ///
    #[payable("*")]
    #[endpoint(redeem)]
    fn redeem(&self, opt_underlying_amount: OptionalValue<BigUint>) -> RedeemResultType<Self::Api> {
        self.accrue_interest();

        let redeemer = self.blockchain().get_caller();
        let (token_id, tokens) = self.call_value().single_fungible_esdt();

        require!(token_id == self.token_id().get(), ERROR_INVALID_TOKEN_PAYMENT);
        require!(tokens > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);

        match opt_underlying_amount {
            OptionalValue::Some(underlying_amount) => {
                require!(underlying_amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);
                self.redeem_underlying_amount(redeemer, tokens, underlying_amount)
            },
            OptionalValue::None => self.redeem_tokens(redeemer, tokens),
        }
    }

    /// The caller redeems Hatom's tokens in exchange for the underlying asset.
    ///
    /// # Arguments:
    ///
    /// - `redeemer` - The address of the account which is redeeming the tokens.
    /// - `tokens` - The amount of Hatom's tokens to redeem into underlying.
    ///
    fn redeem_tokens(&self, redeemer: ManagedAddress, tokens: BigUint) -> RedeemResultType<Self::Api> {
        // no need to check if redeeming is allowed, the redeemer has already exit market and received its Hatom's tokens
        // back if allowed.

        // check if accrual has been updated
        self.require_market_fresh();

        // compute the underlying amount to be redeemed
        let underlying_amount = self.tokens_to_underlying_amount(&tokens);

        self.redeem_internal(&redeemer, &tokens, &underlying_amount);

        self.emit_updated_rates();
        self.redeem_event(&redeemer, &underlying_amount, &tokens);

        let underlying_payment = EgldOrEsdtTokenPayment::new(self.underlying_id().get(), 0, underlying_amount);
        let token_payment = EsdtTokenPayment::new(self.token_id().get(), 0, tokens);

        (underlying_payment, token_payment).into()
    }

    /// The caller redeems Hatom's tokens in exchange for the underlying asset.
    ///
    /// # Arguments:
    ///
    /// - `redeemer` - The address of the account which is redeeming the tokens.
    /// - `paid_tokens` - The amount of Hatom's tokens to redeem into underlying.
    /// - `underlying_amount` - The amount of underlying to receive back in exchange from the paid Hatom's tokens.
    ///
    fn redeem_underlying_amount(&self, redeemer: ManagedAddress, paid_tokens: BigUint, underlying_amount: BigUint) -> RedeemResultType<Self::Api> {
        // no need to check if redeeming is allowed, the redeemer has already exited market and received its Hatom's tokens
        // back if allowed.

        // check if accrual has been updated
        self.require_market_fresh();

        // compute the amount of Hatom's tokens intended to be redeemed
        let tokens = self.underlying_amount_to_tokens(&underlying_amount) + 1u64;
        require!(tokens > BigUint::zero(), ERROR_NOT_ENOUGH_UNDERLYING);
        require!(paid_tokens >= tokens, ERROR_NOT_ENOUGH_TOKENS_TO_REDEEM);

        self.redeem_internal(&redeemer, &tokens, &underlying_amount);

        // send back remainder Hatom's tokens only if necessary
        if paid_tokens > tokens {
            let token_id = self.token_id().get();
            let tokens_left = paid_tokens - &tokens;
            self.send().direct_esdt(&redeemer, &token_id, 0, &tokens_left);
        }

        self.emit_updated_rates();
        self.redeem_event(&redeemer, &underlying_amount, &tokens);

        let underlying_payment = EgldOrEsdtTokenPayment::new(self.underlying_id().get(), 0, underlying_amount);
        let token_payment = EsdtTokenPayment::new(self.token_id().get(), 0, tokens);

        (underlying_payment, token_payment).into()
    }

    fn redeem_internal(&self, redeemer: &ManagedAddress, tokens: &BigUint, underlying_amount: &BigUint) {
        self.try_ensure_staking_rewards(underlying_amount);

        // update cash
        self.cash().update(|amount| *amount -= underlying_amount);

        // update total supply
        self.total_supply().update(|_tokens| *_tokens -= tokens);

        let token_id = self.token_id().get();
        let underlying_id = self.underlying_id().get();

        // burn Hatom's tokens to redeem
        self.send().esdt_local_burn(&token_id, 0, tokens);

        // send underlying to redeemer
        self.send().direct(redeemer, &underlying_id, 0, underlying_amount);
    }
}
