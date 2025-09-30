multiversx_sc::imports!();

use super::{common, errors::*, events, proxies, storage};

#[multiversx_sc::module]
pub trait MintModule: common::CommonModule + events::EventsModule + proxies::ProxyModule + storage::StorageModule {
    /// Supply underlying to the money market, providing liquidity and accruing interest in exchange. In exchange, minted
    /// Hatom tokens are directed to the caller, which can be redeemed for underlying at a given point in the future.
    ///
    #[payable("*")]
    #[endpoint(mint)]
    fn mint(&self) -> EsdtTokenPayment {
        self.require_active();
        self.accrue_interest();

        let (underlying_id, underlying_amount) = self.call_value().egld_or_single_fungible_esdt();
        self.require_valid_underlying_payment(&underlying_id, &underlying_amount);

        let minter = self.blockchain().get_caller();
        self.mint_internal(&minter, &underlying_amount, true)
    }

    /// Mints Hatom's tokens and enters the market in a single transaction.
    ///
    /// # Arguments:
    ///
    /// - `opt_account` - If given, the collateral will be deposited on the name of this account. Can only be performed by a
    ///   trusted minter.
    ///
    /// # Notes:
    ///
    /// - Must be paid with the underlying asset.
    ///
    #[payable("*")]
    #[endpoint(mintAndEnterMarket)]
    fn mint_and_enter_market(&self, opt_account: OptionalValue<ManagedAddress>) -> EsdtTokenPayment {
        self.require_active();
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

        let (underlying_id, underlying_amount) = self.call_value().egld_or_single_fungible_esdt();
        self.require_valid_underlying_payment(&underlying_id, &underlying_amount);

        let token_payment = self.mint_internal(&account, &underlying_amount, false);
        self.enter_market(OptionalValue::Some(account), &token_payment);

        token_payment
    }

    fn mint_internal(&self, minter: &ManagedAddress, underlying_amount: &BigUint, send: bool) -> EsdtTokenPayment {
        // compute the amount of Hatom's tokens to be minted
        let tokens = self.underlying_amount_to_tokens(underlying_amount);
        require!(tokens > BigUint::zero(), ERROR_NOT_ENOUGH_UNDERLYING);

        // check if minting is allowed
        let money_market = self.blockchain().get_sc_address();
        let mint_allowed = self.mint_allowed(&money_market, underlying_amount);
        require!(mint_allowed, ERROR_CONTROLLER_REJECTED_MINT);

        // check if accrual has been updated
        self.require_market_fresh();

        // update cash
        self.cash().update(|amount| *amount += underlying_amount);

        // update total supply
        self.total_supply().update(|_tokens| *_tokens += &tokens);

        // mint Hatom's tokens
        let token_id = self.token_id().get();
        self.send().esdt_local_mint(&token_id, 0, &tokens);

        // send tokens to minter
        if send {
            self.send().direct_esdt(minter, &token_id, 0, &tokens);
        }

        self.emit_updated_rates();
        self.mint_event(minter, underlying_amount, &tokens);

        EsdtTokenPayment::new(token_id, 0, tokens)
    }
}
