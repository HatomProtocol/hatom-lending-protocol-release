#![no_std]

multiversx_sc::imports!();

pub mod controller_proxy;

pub use admin;

pub mod constants;
pub mod errors;
pub mod events;
pub mod governance;
pub mod guardian;
pub mod market;
pub mod policies;
pub mod proxies;
pub mod rewards;
pub mod risk_profile;
pub mod shared;
pub mod storage;

/// Controller Smart Contract
///
/// Handles the control (i.e. checks) for virtually all interactions with the protocol.
///
#[multiversx_sc::contract]
pub trait Controller: admin::AdminModule + events::EventModule + governance::GovernanceModule + guardian::GuardianModule + market::MarketModule + policies::PolicyModule + proxies::ProxyModule + rewards::RewardsModule + risk_profile::RiskProfileModule + shared::SharedModule + storage::StorageModule {
    /// Initializes the contract with an optional admin address.
    ///
    /// # Arguments:
    ///
    /// - `opt_admin` - An optional admin address for the contract.
    ///
    /// Notes:
    ///
    /// - If the contract is being deployed for the first time, the admin address will be set.
    /// - If the admin address is not provided, the admin will be set as the deployer.
    /// - If the contract is being upgraded, the admin address will not be overwritten.
    ///
    #[init]
    fn init(&self, opt_admin: OptionalValue<ManagedAddress>) {
        self.try_set_admin(opt_admin);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
