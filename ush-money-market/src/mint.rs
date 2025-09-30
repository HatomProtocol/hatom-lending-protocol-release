multiversx_sc::imports!();

use super::{commons, errors::*, events, proxies, storage};

#[multiversx_sc::module]
pub trait MintModule: commons::CommonsModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Mints Hatom USH and enters the market in a single transaction.
    ///
    /// # Notes:
    ///
    /// - Must be paid with USH.
    ///
    #[payable("*")]
    #[endpoint(mintAndEnterMarket)]
    fn mint_and_enter_market(&self, opt_account: OptionalValue<ManagedAddress>) -> EsdtTokenPayment {
        self.require_active();
        self.require_eligible_as_collateral();

        // not really needed
        self.accrue_interest();

        let account = match opt_account {
            OptionalValue::None => self.blockchain().get_caller(),
            OptionalValue::Some(account) => {
                let caller = self.blockchain().get_caller();
                require!(caller != account, ERROR_ADDRESSES_MUST_DIFFER);
                self.require_trusted_minter(&caller);
                account
            },
        };

        let (ush_id, ush_payment_amount) = self.call_value().single_fungible_esdt();
        self.require_valid_ush_payment(&ush_id, &ush_payment_amount);

        let token_payment = self.mint_internal(&account, &ush_payment_amount);
        self.enter_market(OptionalValue::Some(account), &token_payment);

        token_payment
    }

    fn mint_internal(&self, minter: &ManagedAddress, ush_amount: &BigUint) -> EsdtTokenPayment {
        // compute the amount of HUSH to be minted
        let tokens = self.ush_to_hush(ush_amount);
        require!(tokens > BigUint::zero(), ERROR_NOT_ENOUGH_USH);

        // check if minting is allowed
        let money_market = self.blockchain().get_sc_address();
        let mint_allowed = self.mint_allowed(&money_market, ush_amount);
        require!(mint_allowed, ERROR_CONTROLLER_REJECTED_MINT);

        // check if accrual has been updated
        self.require_market_fresh();

        // update total supply
        self.total_supply().update(|_tokens| *_tokens += &tokens);

        // mint Hatom's tokens
        let hush_id = self.hush_id().get();
        self.send().esdt_local_mint(&hush_id, 0, &tokens);

        self.mint_event(minter, ush_amount, &tokens);

        EsdtTokenPayment::new(hush_id, 0, tokens)
    }
}
