#![no_std]

multiversx_sc::imports!();

pub mod ush_money_market_proxy;

pub use admin;

pub mod borrow;
pub mod commons;
pub mod constants;
pub mod errors;
pub mod events;
pub mod governance;
pub mod liquidate;
pub mod mint;
pub mod observer;
pub mod proxies;
pub mod redeem;
pub mod repay_borrow;
pub mod seize;
pub mod staking;
pub mod storage;

use crate::{constants::*, errors::*, storage::State};

#[multiversx_sc::contract]
pub trait UshMoneyMarket: admin::AdminModule + borrow::BorrowModule + commons::CommonsModule + events::EventsModule + governance::GovernanceModule + liquidate::LiquidateModule + mint::MintModule + observer::ObserverModule + proxies::ProxyModule + redeem::RedeemModule + repay_borrow::RepayBorrowModule + seize::SeizeModule + staking::StakingModule + storage::StorageModule {
    /// Initializes the USH Money Market.
    ///
    /// # Arguments:
    ///
    /// - `controller` - The Controller smart contract address.
    /// - `ush_minter` - The USH Minter smart contract address.
    /// - `opt_admin` - An optional admin address for the contract.
    ///
    /// Notes:
    ///
    /// - If the admin address is not provided, the admin will be set as the deployer.
    ///
    #[init]
    fn init(&self, controller: ManagedAddress, ush_minter: ManagedAddress, opt_admin: OptionalValue<ManagedAddress>) {
        // set controller
        self.set_controller(&controller);

        // set USH minter
        self.set_ush_minter(&ush_minter);

        // initialize timestamp
        self.set_accrual_timestamp();

        // set admin
        self.try_set_admin(opt_admin);

        // set state
        self.set_ush_market_state_internal(State::Inactive);
    }

    #[upgrade]
    fn upgrade(&self) {}

    /// Issues Hatom USH.
    ///
    #[payable("EGLD")]
    #[endpoint(issueHatomUSH)]
    fn issue_hatom_ush(&self) {
        self.require_admin();
        self.require_inactive();

        self.require_hush_not_issued();

        require!(!self.ongoing_issuance().get(), ERROR_HATOM_USH_ONGOING_ISSUANCE);
        self.ongoing_issuance().set(true);

        let issue_cost = self.call_value().egld_value();
        let caller = self.blockchain().get_caller();
        let initial_supply = BigUint::zero();

        let name = ManagedBuffer::from(HUSH_NAME);
        let ticker = ManagedBuffer::from(HUSH_TICKER);

        self.issue_started_event(&caller, &ticker, &initial_supply);

        self.send()
            .esdt_system_sc_proxy()
            .issue_fungible(
                issue_cost.clone_value(),
                &name,
                &ticker,
                &initial_supply,
                FungibleTokenProperties {
                    num_decimals: HUSH_DECIMALS,
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
                self.hush_id().set(&token_id);
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

    /// Sets minting and burning roles for the Money Market smart contract with respect to Hatom USH.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin.
    ///
    #[endpoint(setMarketRoles)]
    fn set_market_roles(&self) {
        self.require_admin();
        self.require_inactive();
        self.require_hush_issued();
        let hush_id = self.hush_id().get();
        let money_market = self.blockchain().get_sc_address();
        self.send().esdt_system_sc_proxy().set_special_roles(&money_market, &hush_id, [EsdtLocalRole::Mint][..].iter().cloned()).async_call_and_exit();
    }
}
