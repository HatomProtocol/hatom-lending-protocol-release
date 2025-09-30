multiversx_sc::imports!();

use super::{commons, errors::*, events, proxies, storage};

pub type RedeemResultType<BigUint> = MultiValue2<EgldOrEsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[multiversx_sc::module]
pub trait RedeemModule: commons::CommonsModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Exchanges paid Hatom USH tokens for USH.
    ///
    /// # Arguments:
    ///
    /// - `opt_ush_amount` - An optional amount of USH to receive back in exchange for the paid Hatom USH.
    ///
    #[payable("*")]
    #[endpoint(redeem)]
    fn redeem(&self, opt_ush_amount: OptionalValue<BigUint>) -> RedeemResultType<Self::Api> {
        self.accrue_interest();

        let redeemer = self.blockchain().get_caller();
        let (hush_id, tokens) = self.call_value().single_fungible_esdt();

        require!(hush_id == self.hush_id().get(), ERROR_INVALID_HUSH_PAYMENT);
        require!(tokens > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);

        match opt_ush_amount {
            OptionalValue::Some(ush_amount) => {
                require!(ush_amount > BigUint::zero(), ERROR_AMOUNT_MUST_BE_GREATER_THAN_ZERO);
                self.redeem_ush_amount(redeemer, tokens, ush_amount)
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

        // compute the USH amount to be redeemed
        let ush_amount = self.hush_to_ush(&tokens);

        self.redeem_internal(&redeemer, &tokens, &ush_amount);

        self.redeem_event(&redeemer, &ush_amount, &tokens);

        let ush_id = EgldOrEsdtTokenIdentifier::esdt(self.ush_id().get());
        let ush_payment = EgldOrEsdtTokenPayment::new(ush_id, 0, ush_amount);

        let hush_id = self.hush_id().get();
        let hush_payment = EsdtTokenPayment::new(hush_id, 0, tokens);

        (ush_payment, hush_payment).into()
    }

    /// The caller redeems Hatom's tokens in exchange for the underlying asset.
    ///
    /// # Arguments:
    ///
    /// - `redeemer` - The address of the account which is redeeming the tokens.
    /// - `paid_tokens` - The amount of Hatom's tokens to redeem into underlying.
    /// - `ush_amount` - The amount of underlying to receive back in exchange from the paid Hatom's tokens.
    ///
    fn redeem_ush_amount(&self, redeemer: ManagedAddress, paid_tokens: BigUint, ush_amount: BigUint) -> RedeemResultType<Self::Api> {
        // no need to check if redeeming is allowed, the redeemer has already exited market and received its Hatom's tokens
        // back if allowed.

        // check if accrual has been updated
        self.require_market_fresh();

        // compute the amount of Hatom's tokens intended to be redeemed
        let tokens = self.ush_to_hush(&ush_amount) + 1u64;
        require!(tokens > BigUint::zero(), ERROR_NOT_ENOUGH_USH);
        require!(paid_tokens >= tokens, ERROR_NOT_ENOUGH_HUSH_TO_REDEEM);

        self.redeem_internal(&redeemer, &tokens, &ush_amount);

        // send back remainder Hatom's tokens only if necessary
        if paid_tokens > tokens {
            let hush_id = self.hush_id().get();
            let tokens_left = paid_tokens - &tokens;
            self.send().direct_esdt(&redeemer, &hush_id, 0, &tokens_left);
        }

        self.redeem_event(&redeemer, &ush_amount, &tokens);

        let ush_id = EgldOrEsdtTokenIdentifier::esdt(self.ush_id().get());
        let ush_payment = EgldOrEsdtTokenPayment::new(ush_id, 0, ush_amount);

        let hush_id = self.hush_id().get();
        let hush_payment = EsdtTokenPayment::new(hush_id, 0, tokens);

        (ush_payment, hush_payment).into()
    }

    fn redeem_internal(&self, redeemer: &ManagedAddress, tokens: &BigUint, ush_amount: &BigUint) {
        // update total supply
        self.total_supply().update(|_tokens| *_tokens -= tokens);

        let ush_id = self.ush_id().get();
        let hush_id = self.hush_id().get();

        // burn Hatom's tokens to redeem
        self.send().esdt_local_burn(&hush_id, 0, tokens);

        // send underlying to redeemer
        self.send().direct_esdt(redeemer, &ush_id, 0, ush_amount);
    }
}
