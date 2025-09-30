#![no_std]

multiversx_sc::imports!();

pub mod interest_rate_model_proxy;

const WAD: u64 = 1_000_000_000_000_000_000;
const SECONDS_PER_YEAR: u32 = 31_556_926;

// Interest Rate Model
//
// borrow_rate(u) ^
//                │
//          r_max +──────────────────────────+ . . . . . . . . .
//                │                         .
//                │                        .
//                │                       .
//   m1 + m2 + r0 +──────────────────────+
//                │                     .│
//                │                    . │
//                │                   .  │
//                │                  .   │
//                │                 .    │
//                │                .     │
//        m1 + r0 +───────────────+      │
//                │            .  │      │
//                │         .     │      │
//                │      .        │      │
//                │   .           │      │
//             r0 _.              │      │
//                │               │      │
//               0└───────────────+──────+─────────────────────>
//                0               uo     1                      u
//

#[multiversx_sc::contract]
pub trait InterestRateModel {
    /// Initializes the Interest Rate Model smart contract with the given parameters.
    ///
    /// The `r0_y`, `m1_y`, `m2_y`, `uo`, and `r_max` parameters describe the piecewise linear function that determines the
    /// borrow rate. They are provided in per year term basis and are translated to a per second term basis using constants
    /// `WAD` and `SECONDS_PER_YEAR`.
    ///
    /// # Arguments:
    ///
    /// - `r0_y` - The base borrow rate per year.
    /// - `m1_y` - The first slope of the borrow rate function per year.
    /// - `m2_y` - The last slope of the borrow rate function per year.
    /// - `uo` - The optimal utilization.
    /// - `r_max` - The maximum borrow rate per second.
    ///
    #[init]
    fn init(&self, r0_y: BigUint, m1_y: BigUint, m2_y: BigUint, uo: BigUint, r_max: BigUint) {
        let wad = BigUint::from(WAD);
        let spy = BigUint::from(SECONDS_PER_YEAR);

        require!(uo < wad, "optimal utilization should be less than one");

        let r0 = r0_y / &spy;
        let m1 = m1_y * &wad / &spy / &uo;
        let m2 = m2_y * &wad / &spy / (&wad - &uo);
        require!(m2 > BigUint::zero(), "last slope must be greater than zero");
        require!(m2 >= m1, "last slope must be greater or equal than first slope");

        // make sure r_max is higher than the borrow rate for u = 1
        let r1 = &m1 * &uo / &wad + &r0;
        let r = r1 + &m2 * &(&wad - &uo) / &wad;
        require!(r_max > r0 && r_max >= r, "max borrow rate too low");

        self.base_rate().set_if_empty(&r0);
        self.first_slope().set_if_empty(&m1);
        self.last_slope().set_if_empty(&m2);
        self.optimal_utilization().set_if_empty(&uo);
        self.max_borrow_rate().set_if_empty(&r_max);
    }

    #[upgrade]
    fn upgrade(&self) {}

    /// Returns true to indicate that this contract is an interest rate model.
    #[view(isInterestRateModel)]
    fn is_interest_rate_model(&self) -> bool {
        true
    }

    /// Returns the utilization rate for the given amount of borrows and liquidity. Utilization rate is calculated as borrows
    /// divided by liquidity. If borrows are zero, returns zero. If liquidity is zero, returns the utilization rate that
    /// yields the maximum borrow rate.
    ///
    /// # Arguments:
    ///
    /// - `borrows` - The total amount borrowed.
    /// - `liquidity` - The total amount of funds available for borrowing.
    ///
    #[view(getUtilization)]
    fn get_utilization(&self, borrows: &BigUint, liquidity: &BigUint) -> BigUint {
        let zero = BigUint::zero();
        if *borrows == zero {
            return zero;
        }

        let wad = BigUint::from(WAD);

        // handle edge case, when liquidity is zero return the utilization that yields the max borrow rate (sum `+ 2` to
        // take into consideration the truncation error yielded by the previous two divisions)
        if *liquidity == zero {
            let r0 = self.base_rate().get();
            let m1 = self.first_slope().get();
            let m2 = self.last_slope().get();
            let uo = self.optimal_utilization().get();
            let r_max = self.max_borrow_rate().get();
            return (&m2 - &m1) * uo / &m2 + (r_max - r0) * wad / &m2 + 2u64;
        }

        borrows * &wad / liquidity
    }

    /// Returns the current model parameters used to calculate the borrow rate, as a tuple of:
    ///
    /// - Base rate (r0)
    /// - First slope (m1)
    /// - Last slope (m2)
    /// - Optimal utilization (uo)
    /// - Maximum borrow rate (r_max)
    ///
    #[view(getModelParameters)]
    fn get_model_parameters(&self) -> (BigUint, BigUint, BigUint, BigUint, BigUint) {
        let r0 = self.base_rate().get();
        let m1 = self.first_slope().get();
        let m2 = self.last_slope().get();
        let uo = self.optimal_utilization().get();
        let r_max = self.max_borrow_rate().get();
        (r0, m1, m2, uo, r_max)
    }

    /// Computes the borrow rate per second based on the current state of the model, the total borrows and total liquidity.
    ///
    /// # Arguments:
    ///
    /// - `borrows` - The total amount of borrows in the market.
    /// - `liquidity` - The total amount of cash available to be borrowed or used as collateral.
    ///
    #[view(getBorrowRate)]
    fn get_borrow_rate(&self, borrows: &BigUint, liquidity: &BigUint) -> BigUint {
        let zero = BigUint::zero();
        let r0 = self.base_rate().get();
        let r_max = self.max_borrow_rate().get();

        // utilization is zero and borrow rate equals to base rate
        if *borrows == zero {
            return r0;
        }

        // utilization is infinity and borrow rate reaches it maximum
        if *liquidity == zero {
            self.reached_max_borrow_rate_event(borrows, liquidity);
            return r_max;
        }

        let wad = BigUint::from(WAD);
        let u = self.get_utilization(borrows, liquidity);
        let m1 = self.first_slope().get();
        let uo = self.optimal_utilization().get();

        if u <= uo {
            return m1 * u / &wad + r0;
        }
        let m2 = self.last_slope().get();
        let r_max = self.max_borrow_rate().get();
        let r1 = m1 * &uo / &wad + r0;
        let r = r1 + m2 * (u - uo) / &wad;

        if r >= r_max {
            self.reached_max_borrow_rate_event(borrows, liquidity);
            return r_max;
        }

        r
    }

    /// Calculates the current supply rate per second given the total amount of borrows, the total amount of underlying
    /// assets, and the current reserve factor.
    ///
    /// # Arguments:
    ///
    /// - `borrows` - The total amount of outstanding borrows of the underlying asset.
    /// - `liquidity` - The total amount of the underlying asset supplied by the users.
    /// - `reserve_factor` - The current reserve factor applied to the market.
    ///
    #[view(getSupplyRate)]
    fn get_supply_rate(&self, borrows: &BigUint, liquidity: &BigUint, reserve_factor: &BigUint) -> BigUint {
        let borrow_rate = self.get_borrow_rate(borrows, liquidity);
        self.get_supply_rate_internal(borrows, liquidity, &borrow_rate, reserve_factor)
    }

    /// Calculates the supply rate per second given the total amount of borrows, the total amount of underlying assets, and
    /// the current reserve factor.
    ///
    /// # Arguments:
    ///
    /// - `borrows` - The total amount of outstanding borrows of the underlying asset.
    /// - `liquidity` - The total amount of the underlying asset supplied by the users.
    /// - `borrow_rate` - The current borrow rate.
    /// - `reserve_factor` - The current reserve factor applied to the market.
    ///
    fn get_supply_rate_internal(&self, borrows: &BigUint, liquidity: &BigUint, borrow_rate: &BigUint, reserve_factor: &BigUint) -> BigUint {
        let wad = BigUint::from(WAD);
        let utilization = self.get_utilization(borrows, liquidity);
        utilization * borrow_rate / &wad * (&wad - reserve_factor) / &wad
    }

    // Computes the borrow rate and supply rate per second for the given borrow balance, liquidity, and reserve factor.
    ///
    /// - `borrows` - The current borrow balance.
    /// - `liquidity` - The current liquidity balance.
    /// - `reserve_factor` - The reserve factor for the asset.
    ///
    #[view(getRates)]
    fn get_rates(&self, borrows: &BigUint, liquidity: &BigUint, reserve_factor: &BigUint) -> (BigUint, BigUint) {
        let borrow_rate = self.get_borrow_rate(borrows, liquidity);
        let supply_rate = self.get_supply_rate_internal(borrows, liquidity, &borrow_rate, reserve_factor);
        (borrow_rate, supply_rate)
    }

    /// Stores the base rate used in the interest rate calculation.
    #[view(getBaseRate)]
    #[storage_mapper("base_rate")]
    fn base_rate(&self) -> SingleValueMapper<BigUint>;

    /// Stores the slope of the borrow rate up to the optimal utilization point.
    #[view(getFirstSlope)]
    #[storage_mapper("first_slope")]
    fn first_slope(&self) -> SingleValueMapper<BigUint>;

    /// Stores the slope of the borrow rate after the optimal utilization point.
    #[view(getLastSlope)]
    #[storage_mapper("last_slope")]
    fn last_slope(&self) -> SingleValueMapper<BigUint>;

    /// Stores the optimal utilization point for the interest rate calculation.
    #[view(getOptimalUtilization)]
    #[storage_mapper("optimal_utilization")]
    fn optimal_utilization(&self) -> SingleValueMapper<BigUint>;

    /// Stores the maximum borrow rate allowed by the interest rate model.
    #[view(getMaxBorrowRate)]
    #[storage_mapper("max_borrow_rate")]
    fn max_borrow_rate(&self) -> SingleValueMapper<BigUint>;

    /// Emitted when the liquidity is zero and utilization is infinity or when liquidity != 0 and utilization is high.
    #[event("reached_max_borrow_rate_event")]
    fn reached_max_borrow_rate_event(&self, #[indexed] borrows: &BigUint, #[indexed] liquidity: &BigUint);
}
