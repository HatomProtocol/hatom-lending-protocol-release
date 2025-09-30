#![no_std]

multiversx_sc::imports!();

pub mod money_market_proxy;

pub use admin;

pub mod borrow;
pub mod common;
pub mod constants;
pub mod errors;
pub mod events;
pub mod governance;
pub mod liquidate;
pub mod mint;
pub mod proxies;
pub mod redeem;
pub mod repay_borrow;
pub mod seize;
pub mod staking;
pub mod storage;

use crate::{constants::*, errors::*, storage::State};

#[multiversx_sc::contract]
pub trait MoneyMarket: admin::AdminModule + borrow::BorrowModule + common::CommonModule + events::EventsModule + governance::GovernanceModule + liquidate::LiquidateModule + mint::MintModule + proxies::ProxyModule + redeem::RedeemModule + repay_borrow::RepayBorrowModule + seize::SeizeModule + storage::StorageModule + staking::StakingModule {
    /// Initialize the Money Market.
    ///
    /// # Arguments:
    ///
    /// - `underlying_id` - The underlying token identifier.
    /// - `controller` - The address of the Controller.
    /// - `interest_rate_model` - The address of the Interest Rate Model.
    /// - `initial_exchange_rate` - The initial exchange rate in wad.
    /// - `opt_admin` - An optional admin address for the contract.
    ///
    /// Notes:
    ///
    /// - If the contract is being deployed for the first time, the admin address will be set.
    /// - If the admin address is not provided, the admin will be set as the deployer.
    /// - If the contract is being upgraded, the admin address will not be overwritten.
    /// - Upgrades won't change the underlying token identifier, initial exchange rate, accrual timestamp or the admin.
    ///
    #[init]
    fn init(&self, underlying_id: EgldOrEsdtTokenIdentifier, controller: ManagedAddress, interest_rate_model: ManagedAddress, initial_exchange_rate: BigUint, opt_admin: OptionalValue<ManagedAddress>) {
        // try set underlying
        self.try_set_underlying_id(&underlying_id);

        // try set initial exchange rate
        self.try_set_initial_exchange_rate(&initial_exchange_rate);

        // try set controller
        self.try_set_controller(&controller);

        // try set initialize timestamp
        self.try_set_accrual_timestamp();

        // try set interest rate model
        self.try_set_interest_rate_model(&interest_rate_model);

        // try set admin
        self.try_set_admin(opt_admin);

        // try set state
        self.try_set_market_state(&State::Inactive);
    }

    #[upgrade]
    fn upgrade(&self) {}

    /// Issue the ESDT Hatom Token.
    ///
    /// # Arguments:
    ///
    /// - `name` - The token display name for the Hatom token.
    /// - `ticker` - The token ticker for the Hatom token.
    /// - `decimals` - The decimal precision for the Hatom token.
    ///
    #[payable("EGLD")]
    #[endpoint(issueEsdtToken)]
    fn issue_esdt_token(&self, name: ManagedBuffer, ticker: ManagedBuffer, decimals: usize) {
        self.require_admin();
        self.require_inactive();

        require!(!self.is_token_issued(), ERROR_HATOM_TOKEN_ALREADY_ISSUED);

        require!(!self.ongoing_issuance().get(), ERROR_HATOM_TOKEN_ONGOING_ISSUANCE);
        self.ongoing_issuance().set(true);

        let issue_cost = self.call_value().egld_value();
        let caller = self.blockchain().get_caller();
        let initial_supply = BigUint::zero();

        self.issue_started_event(&caller, &ticker, &initial_supply);

        self.send()
            .esdt_system_sc_proxy()
            .issue_fungible(
                issue_cost.clone_value(),
                &name,
                &ticker,
                &initial_supply,
                FungibleTokenProperties {
                    num_decimals: decimals,
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_mint: true,
                    can_burn: true,
                    can_change_owner: true,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .callback(self.callbacks().esdt_issue_callback(&caller))
            .async_call_and_exit()
    }

    #[callback]
    fn esdt_issue_callback(&self, caller: &ManagedAddress, #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.token_id().set(&token_id);
                self.issue_success_event(caller, &token_id, &BigUint::zero());
            },
            ManagedAsyncCallResult::Err(message) => {
                let (token_id, returned_tokens) = self.call_value().egld_or_single_fungible_esdt();
                if token_id.is_egld() && returned_tokens > BigUint::zero() {
                    self.send().direct_egld(caller, &returned_tokens);
                }
                self.issue_failure_event(caller, &message.err_msg);
            },
        }
        self.ongoing_issuance().set(false);
    }

    /// Sets minting and burning roles for the Money Market smart contract with respect to the Hatom Token.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setMarketRoles)]
    fn set_market_roles(&self) {
        self.require_admin();
        self.require_inactive();
        self.require_token_issued();
        let token_id = self.token_id().get();
        let money_market = self.blockchain().get_sc_address();
        self.send().esdt_system_sc_proxy().set_special_roles(&money_market, &token_id, [EsdtLocalRole::Mint][..].iter().cloned()).async_call_and_exit();
    }

    /// Mint at least the initial supply of Hatom tokens. These tokens will be burned to make sure that the total supply
    /// never returns to zero. This is particularly useful because it enforces that the exchange rate will never returns to
    /// its initial condition.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    /// - Can only be called once.
    /// - The initial supply is equal to the minimum initial supply.
    /// - The remainder tokens are sent back to the caller.
    ///
    #[payable("*")]
    #[endpoint(mintInitialSupply)]
    fn mint_initial_supply(&self) {
        self.require_admin();
        self.require_inactive();
        self.require_token_issued();

        require!(!self.minted_initial_supply().get(), ERROR_INITIAL_SUPPLY_ALREADY_MINTED);

        self.accrue_interest();
        self.require_market_fresh();

        let (underlying_id, underlying_amount) = self.call_value().egld_or_single_fungible_esdt();
        self.require_valid_underlying_payment(&underlying_id, &underlying_amount);

        let minter = self.blockchain().get_caller();
        let (token_id, _, tokens) = self.mint_internal(&minter, &underlying_amount, false).into_tuple();
        self.minted_initial_supply().set(true);

        let initial_supply = BigUint::from(MIN_INITIAL_SUPPLY);
        require!(tokens >= initial_supply, ERROR_NOT_ENOUGH_UNDERLYING);

        self.send().esdt_local_burn(&token_id, 0, &initial_supply);

        let tokens_left = tokens - &initial_supply;
        if tokens_left > BigUint::zero() {
            self.send().direct_esdt(&minter, &token_id, 0, &tokens_left);
        };

        self.set_market_state_internal(&State::Active);

        self.mint_initial_supply_event(&minter, &initial_supply, &tokens_left);
    }
}
