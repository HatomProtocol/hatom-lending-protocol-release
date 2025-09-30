multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::{events, proxies, shared, storage};

#[multiversx_sc::module]
pub trait GuardianModule: admin::AdminModule + events::EventModule + proxies::ProxyModule + shared::SharedModule + storage::StorageModule {
    /// Changes the minting status for a specific money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `pause` - A boolean that indicates whether the protocol must be or not paused.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or the Guardian.
    ///
    #[endpoint(pauseMint)]
    fn pause_mint(&self, money_market: &ManagedAddress, pause: bool) {
        self.require_admin_or_guardian();
        self.require_whitelisted_money_market(money_market);

        if pause {
            self.mint_status(money_market).set(storage::Status::Paused);
        } else {
            self.mint_status(money_market).set(storage::Status::Active);
        }

        self.mint_paused_event(money_market, pause);
    }

    /// Changes the borrowing status for a specific money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `pause` - A boolean that indicates whether the protocol must be or not paused.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or the Guardian.
    ///
    #[endpoint(pauseBorrow)]
    fn pause_borrow(&self, money_market: &ManagedAddress, pause: bool) {
        self.require_admin_or_guardian();
        self.require_whitelisted_money_market(money_market);

        if pause {
            self.borrow_status(money_market).set(storage::Status::Paused);
        } else {
            self.borrow_status(money_market).set(storage::Status::Active);
        }

        self.borrow_paused_event(money_market, pause);
    }

    /// Changes the seizing status for a specific money market.
    ///
    /// # Arguments:
    ///
    /// - `money_market` - The address of the money market smart contract.
    /// - `pause` - A boolean that indicates whether the protocol must be or not paused.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or the Guardian.
    ///
    #[endpoint(pauseSeize)]
    fn pause_seize(&self, money_market: &ManagedAddress, pause: bool) {
        self.require_admin_or_guardian();
        self.require_whitelisted_money_market(money_market);

        if pause {
            self.seize_status(money_market).set(storage::Status::Paused);
        } else {
            self.seize_status(money_market).set(storage::Status::Active);
        }

        self.seize_paused_event(money_market, pause);
    }

    /// Changes the seizing status (required for liquidations) for all money markets.
    ///
    /// # Arguments:
    ///
    /// - `pause` - A boolean that indicates whether the protocol must be or not paused.
    ///
    /// # Notes:
    ///
    /// - Can only be called by the admin or the Guardian.
    ///
    #[endpoint(pauseGlobalSeize)]
    fn pause_global_seize(&self, pause: bool) {
        self.require_admin_or_guardian();

        if pause {
            self.global_seize_status().set(storage::Status::Paused);
        } else {
            self.global_seize_status().set(storage::Status::Active);
        }

        self.global_seize_paused_event(pause);
    }
}
