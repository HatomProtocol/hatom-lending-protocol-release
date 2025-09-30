#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod oracle_proxy;

pub use admin;

pub mod common;
pub mod constants;
pub mod errors;
pub mod events;
pub mod governance;
pub mod model;
pub mod prices;
pub mod proxies;
pub mod storage;

use crate::model::ExchangePricingMethod;

#[multiversx_sc::contract]
pub trait Oracle: admin::AdminModule + common::CommonModule + events::EventsModule + governance::GovernanceModule + prices::PriceModule + proxies::ProxyModule + storage::StorageModule {
    /// Initializes the Oracle.
    ///
    /// # Arguments:
    ///
    /// - `egld_wrapper` - The wrapped EGLD smart contract for the pertinent shard.
    /// - `xexchange_pricing_method` - The xExchange pricing methods allowed.
    /// - `opt_admin` - An optional admin address for the contract.
    ///
    /// Notes:
    ///
    /// - If the contract is being deployed for the first time, the admin address will be set.
    /// - If the admin address is not provided, the admin will be set as the deployer.
    /// - If the contract is being upgraded, the admin address will not be overwritten.
    /// - A new implementation could be deployed instead of performing an upgrade.
    ///
    #[init]
    fn init(&self, egld_wrapper: ManagedAddress, xexchange_pricing_method: ExchangePricingMethod, opt_admin: OptionalValue<ManagedAddress>) {
        self.try_set_egld_wrapper(&egld_wrapper);
        self.try_set_xexchange_pricing_method(xexchange_pricing_method);
        self.try_set_admin(opt_admin);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
