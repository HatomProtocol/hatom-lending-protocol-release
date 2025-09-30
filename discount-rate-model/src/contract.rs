#![no_std]

multiversx_sc::imports!();

pub mod discount_rate_model_proxy;

pub mod commons;
pub mod constants;
pub mod discount;
pub mod errors;
pub mod events;
pub mod governance;
pub mod models;
pub mod proxies;
pub mod storage;

#[multiversx_sc::contract]
pub trait DiscountRateModel: admin::AdminModule + commons::CommonsModule + discount::DiscountModule + events::EventsModule + governance::GovernanceModule + proxies::ProxyModule + storage::StorageModule {
    /// Initializes the Discount Rate Model smart contract.
    ///
    /// # Arguments:
    ///
    /// - `ush_money_market` - The USH Money Market smart contract address.
    /// - `opt_admin` - An optional admin address for the contract.
    ///
    /// Notes:
    ///
    /// - If the admin address is not provided, the admin will be set as the deployer.
    ///
    #[init]
    fn init(&self, ush_money_market: ManagedAddress, opt_admin: OptionalValue<ManagedAddress>) {
        self.set_ush_money_market(&ush_money_market);
        self.try_set_admin(opt_admin);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
