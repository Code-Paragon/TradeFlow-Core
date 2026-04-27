#![no_std]
use soroban_sdk::{
    contract,
    contracterror,
    contractimpl,
    contracttype,
    panic_with_error,
    symbol_short,
    token,
    Address,
    Env,
    U256,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    InvariantViolated = 1,
}

mod tests;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolState {
    pub token_a: Address,
    pub token_b: Address,
    // Storing as u32 to match token interface
    pub token_a_decimals: u32,
    pub token_b_decimals: u32,
    pub reserve_a: i128,
    pub reserve_b: i128,
    pub fee_tier: u32, // Fee tier in basis points (5, 30, or 100)
    pub is_deprecated: bool,
    pub _status: u32, // 0 = unlocked, 1 = locked (reentrancy protection)
    pub deposits_paused: bool,    // When true, new deposits and swaps are blocked
    pub withdrawals_paused: bool, // When true, liquidity removals are blocked
    // TWAP Oracle state variables
    pub price_0_cumulative_last: u128, // Cumulative price for token_0
    pub price_1_cumulative_last: u128, // Cumulative price for token_1
    pub block_timestamp_last: u32,     // Last update timestamp
}

#[contracttype]
pub enum DataKey {
    State,
    Admin,
    FrozenAddress(Address),
    PendingAdmin,
}

#[contract]
pub struct AmmPool;

#[contractimpl]
impl AmmPool {
    /// Initialize the AMM pool with two tokens, admin, and fee tier.
    /// 1. Queries the Stellar network to fetch exact decimal precision via Soroban token interface.
    /// 2. Validates that both values are positive integers <= 18.
    /// 3. Validates fee tier is one of the supported values (5, 30, or 100 basis points).
    /// 4. Aborts initialization if validation fails.
    pub fn init(env: Env, admin: Address, token_a: Address, token_b: Address, fee_tier: u32) {
        if env.storage().instance().has(&DataKey::State) {
            panic!("Already initialized");
        }

        let client_a = token::Client::new(&env, &token_a);
        let client_b = token::Client::new(&env, &token_b);

        let decimals_a = client_a.decimals();
        let decimals_b = client_b.decimals();

        if decimals_a == 0 || decimals_a > 18 {
            panic!("Invalid decimals for token_a");
        }
        if decimals_b == 0 || decimals_b > 18 {
            panic!("Invalid decimals for token_b");
        }

        // Validate fee tier
        if fee_tier != 5 && fee_tier != 30 && fee_tier != 100 {
            panic!("Invalid fee tier. Only 5, 30, or 100 basis points are supported");
        }

        let state = PoolState {
            token_a,
            token_b,
            token_a_decimals: decimals_a,
            token_b_decimals: decimals_b,
            reserve_a: 0,
            reserve_b: 0,
            fee_tier,
            is_deprecated: false,
            _status: 0, // Start unlocked
            deposits_paused: false,
            withdrawals_paused: false,
            // Initialize TWAP oracle state
            price_0_cumulative_last: 0,
            price_1_cumulative_last: 0,
            block_timestamp_last: 0,
        };

        env.storage().instance().set(&DataKey::State, &state);
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Provide liquidity after verifying the user holds sufficient balance and allowance
    /// for both tokens. Call-sites 1 and 2 for verify_balance_and_allowance.
    ///
    /// # Arguments
    /// * `env` - The Soroban execution environment.
    /// * `user` - The address of the liquidity provider.
    /// * `amount_a` - The amount of `token_a` to deposit into the pool.
    /// * `amount_b` - The amount of `token_b` to deposit into the pool.
    pub fn provide_liquidity(env: Env, user: Address, amount_a: i128, amount_b: i128) {
        user.require_auth();
        Self::require_not_frozen(&env, &user);
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        if state.is_deprecated {
            panic!("Pool is deprecated");
        }
        if state.deposits_paused {
            panic!("deposits are paused");
        }

        let client_a = token::Client::new(&env, &state.token_a);
        let client_b = token::Client::new(&env, &state.token_b);

        Self::verify_balance_and_allowance(&env, &state.token_a, &user, amount_a);
        Self::verify_balance_and_allowance(&env, &state.token_b, &user, amount_b);

        client_a.transfer(&user, &env.current_contract_address(), &amount_a);
        client_b.transfer(&user, &env.current_contract_address(), &amount_b);

        state.reserve_a = state.reserve_a.saturating_add(amount_a);
        state.reserve_b = state.reserve_b.saturating_add(amount_b);
        env.storage().instance().set(&DataKey::State, &state);

        // Invariant check: ensure physical balances back internal reserves
        if client_a.balance(&env.current_contract_address()) < state.reserve_a || 
           client_b.balance(&env.current_contract_address()) < state.reserve_b {
            panic_with_error!(&env, Error::InvariantViolated);
        }
    }

    /// Helper function to check admin authorization
    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();
    }

    /// Helper function to check if an address is frozen
    fn is_address_frozen(env: &Env, address: &Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::FrozenAddress(address.clone()))
            .unwrap_or(false)
    }

    /// Helper function to require address is not frozen
    fn require_not_frozen(env: &Env, address: &Address) {
        if Self::is_address_frozen(env, address) {
            panic!("address is frozen");
        }
    }

    /// Admin-only function to freeze or unfreeze a specific address.
    /// When an address is frozen, it cannot execute swaps, provide liquidity, or remove liquidity.
    /// This is an emergency measure for compliance and security against known malicious actors.
    pub fn set_address_freeze_status(env: Env, address: Address, frozen: bool) {
        Self::require_admin(&env);
        env.storage()
            .instance()
            .set(&DataKey::FrozenAddress(address.clone()), &frozen);

        // Emit event for transparency
        env.events().publish(
            (symbol_short!("Freeze"), symbol_short!("Status")),
            (address, frozen)
        );
    }

    /// Query function to check if an address is currently frozen
    pub fn is_frozen(env: Env, address: Address) -> bool {
        Self::is_address_frozen(&env, &address)
    }

    /// Step 1 of ownership transfer: Current admin proposes a new admin address.
    /// Only the current admin can call this.
    pub fn propose_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::PendingAdmin, &new_admin);
    }

    /// Step 2 of ownership transfer: The proposed admin accepts the role.
    /// Only the pending admin can call this.
    pub fn accept_admin(env: Env) {
        let pending_admin: Address = env.storage().instance()
            .get(&DataKey::PendingAdmin)
            .expect("No pending admin");
        
        pending_admin.require_auth();

        let old_admin: Address = env.storage().instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        
        env.storage().instance().set(&DataKey::Admin, &pending_admin);
        env.storage().instance().remove(&DataKey::PendingAdmin);

        // Emit event for transparency
        env.events().publish(
            (symbol_short!("Admin"), symbol_short!("Transfer")),
            (old_admin, pending_admin)
        );
    }

    /// Admin: pause or unpause new deposits and swaps into the pool.
    /// When paused, provide_liquidity and swap will reject all calls,
    /// but existing LPs can still withdraw via remove_liquidity.
    pub fn set_deposits_paused(env: Env, paused: bool) {
        Self::require_admin(&env);
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        state.deposits_paused = paused;
        env.storage().instance().set(&DataKey::State, &state);
    }

    /// Admin: pause or unpause liquidity withdrawals from the pool.
    /// When paused, remove_liquidity will reject all calls,
    /// but new deposits can still enter via provide_liquidity.
    pub fn set_withdrawals_paused(env: Env, paused: bool) {
        Self::require_admin(&env);
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        state.withdrawals_paused = paused;
        env.storage().instance().set(&DataKey::State, &state);
    }

    /// Admin-only: Permanently deprecate the pool.
    /// This is an irreversible one-way toggle.
    /// Swaps and new liquidity provision are disabled, but withdrawals remain active.
    pub fn set_deprecated(env: Env) {
        Self::require_admin(&env);
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        state.is_deprecated = true;
        env.storage().instance().set(&DataKey::State, &state);

        // Emit event for transparency
        env.events().publish(
            (symbol_short!("Admin"), symbol_short!("Deprecat")),
            env.current_contract_address()
        );
    }

    /// Verify that `user` holds at least `required_amount` of `token` and has granted
    /// at least that much allowance to this contract. Panics early with a descriptive
    /// message if either check fails. No-ops when `required_amount <= 0`.
    fn verify_balance_and_allowance(env: &Env, token: &Address, user: &Address, required_amount: i128) {
        if required_amount <= 0 {
            return;
        }
        let client = token::Client::new(env, token);
        let balance = client.balance(user);
        if balance < required_amount {
            panic!("insufficient balance");
        }
        let allowance = client.allowance(user, &env.current_contract_address());
        if allowance < required_amount {
            panic!("insufficient allowance");
        }
    }

    /// Emergency eject liquidity - Admin only function to forcefully withdraw all liquidity
    /// from a deprecated pool and return underlying tokens to LPs based on snapshot balances.
    /// Requirements:
    /// - Must be called by Admin
    /// - Pool must be deprecated (is_deprecated = true)
    /// - Pool must not be locked by reentrancy (_status = 0)
    pub fn emergency_eject_liquidity(env: Env) {
        // Check admin authorization
        Self::require_admin(&env);

        // Get current pool state
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");

        // Verify pool is deprecated
        if !state.is_deprecated {
            panic!("Pool is not deprecated - emergency eject not allowed");
        }

        // Verify pool is not locked by reentrancy
        if state._status != 0 {
            panic!("Pool is locked - reentrancy protection active");
        }

        // Set reentrancy lock
        state._status = 1;
        env.storage().instance().set(&DataKey::State, &state);

        // Emit massive ProtocolEmergencyEject event to alert all indexers
        env.events().publish(
            (symbol_short!("EmergEjct"), symbol_short!("CRITICAL")),
            (env.current_contract_address(), state.token_a.clone(), state.token_b.clone(), state.reserve_a, state.reserve_b)
        );

        // TODO: This is where the complex iteration over LP token holders would happen
        // For this issue, we're just scaffolding the state requirements and modifiers
        // The actual implementation would:
        // 1. Iterate through all LP token holders
        // 2. Calculate each LP's share based on their snapshot balances
        // 3. Transfer proportional underlying tokens to each LP
        // 4. Burn LP tokens
        // 5. Reset pool reserves to zero

        // Reset pool state after eject
        state.reserve_a = 0;
        state.reserve_b = 0;
        state._status = 0; // Unlock reentrancy protection

        env.storage().instance().set(&DataKey::State, &state);

        // Emit completion event
        env.events().publish(
            (symbol_short!("EmergEjct"), symbol_short!("COMPLETED")),
            env.current_contract_address()
        );
    }

    /// Calculate the output amount for a given input amount, applying the pool's fee tier.
    ///
    /// Scaling formulas:
    /// - scaled = raw * 10^(18 - token_decimals)
    /// - output_native = output_scaled / 10^(18 - target_decimals)
    ///
    /// Uses constant-product formula (x*y=k) with fee deduction from input.
    /// Uses saturating arithmetic to prevent overflows and rounding half-up for output.
    pub fn calculate_amount_out(env: Env, amount_in: i128, is_a_in: bool) -> i128 {
        let state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");

        let (decimals_in, decimals_out, reserve_in, reserve_out) = if is_a_in {
            (state.token_a_decimals, state.token_b_decimals, state.reserve_a, state.reserve_b)
        } else {
            (state.token_b_decimals, state.token_a_decimals, state.reserve_b, state.reserve_a)
        };

        if amount_in <= 0 {
            return 0;
        }
        if reserve_in <= 0 || reserve_out <= 0 {
            return 0; // Pool is empty
        }

        // Scale inputs to 18-decimal precision using saturating arithmetic
        let scale_in = 10i128.pow((18 - decimals_in) as u32);
        let amount_in_scaled = amount_in.saturating_mul(scale_in);
        let reserve_in_scaled = reserve_in.saturating_mul(scale_in);

        let scale_out = 10i128.pow((18 - decimals_out) as u32);
        let reserve_out_scaled = reserve_out.saturating_mul(scale_out);

        // Constant-product calculation (x * y = k) with fee
        let fee_multiplier = 10000i128.saturating_sub(state.fee_tier as i128);
        let amount_in_with_fee = amount_in_scaled.saturating_mul(fee_multiplier);

        // amount_out_scaled = (reserve_out_scaled * amount_in_with_fee) / (reserve_in_scaled * 10000 + amount_in_with_fee)
        let numerator = reserve_out_scaled.saturating_mul(amount_in_with_fee);
        let denominator = reserve_in_scaled.saturating_mul(10000).saturating_add(amount_in_with_fee);

        if denominator == 0 {
            return 0;
        }

        let output_scaled = numerator / denominator;

        // Scale back to target token's native decimals.
        // We MUST round down (truncate) to ensure the pool never gives out more than the formula allows.
        let output_native = output_scaled / scale_out;

        // Return zero if the scaled output is below the target token's smallest unit
        if output_native == 0 {
            return 0;
        }

        // Calculate precision loss
        let output_scaled_from_native = output_native.saturating_mul(scale_out);
        let loss = output_scaled.abs_diff(output_scaled_from_native);

        // Emit debug event if precision loss exceeds 0.01% (i.e., loss * 10000 > output_scaled)
        if loss.saturating_mul(10000) > output_scaled.unsigned_abs() {
            env.events().publish((symbol_short!("warn"), symbol_short!("prec_loss")), loss as i128);
        }

        output_native
    }

    /// Calculate the input amount required for a given output amount (Exact Output), 
    /// accounting for the pool's fee tier.
    ///
    /// # Formula:
    /// numerator = reserve_in * amount_out * 10000
    /// denominator = (reserve_out - amount_out) * (10000 - fee_tier)
    /// amount_in = (numerator / denominator) + 1 (to round up)
    ///
    /// # Arguments:
    /// * `amount_out` - The desired output amount.
    /// * `reserve_in` - Current reserve of the input token.
    /// * `reserve_out` - Current reserve of the output token.
    ///
    /// # Returns:
    /// The calculated amount_in required to receive the desired amount_out.
    pub fn calculate_amount_in(
        env: Env,
        amount_out: i128,
        reserve_in: i128,
        reserve_out: i128,
    ) -> i128 {
        let state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        if amount_out <= 0 || reserve_in <= 0 || reserve_out <= 0 {
            return 0;
        }
        if amount_out >= reserve_out {
            panic!("Insufficient liquidity for requested output");
        }

        let fee_multiplier = 10000i128.saturating_sub(state.fee_tier as i128);

        let numerator = reserve_in
            .saturating_mul(amount_out)
            .saturating_mul(10000);
        let denominator = (reserve_out.saturating_sub(amount_out))
            .saturating_mul(fee_multiplier);

        // Ceiling division: (numerator + denominator - 1) / denominator
        let amount_in = (numerator.saturating_add(denominator).saturating_sub(1)) / denominator;
        
        amount_in
    }

    /// Swap tokens: verify user balance/allowance for the input token (call-site 3),
    /// then calculate and return the output amount using the constant-product formula.
    /// Does not perform actual token transfers (out of scope for this feature).
    ///
    /// # Arguments
    /// * `env` - The Soroban execution environment.
    /// * `user` - The address of the user initiating the swap.
    /// * `amount_in` - The amount of input tokens being swapped.
    /// * `is_a_in` - Boolean flag: `true` if input is token A, `false` if input is token B.
    ///
    /// # Returns
    /// The calculated amount of the output token based on the constant-product formula.
    pub fn swap(env: Env, user: Address, amount_in: i128, is_a_in: bool) -> i128 {
        user.require_auth();
        Self::require_not_frozen(&env, &user);

        if state.deposits_paused {
            panic!("deposits are paused");
        }

        let client_a = token::Client::new(&env, &state.token_a);
        let client_b = token::Client::new(&env, &state.token_b);

        // 1. Capture physical balances before any transfers occur
        let bal_a_before = client_a.balance(&env.current_contract_address());
        let bal_b_before = client_b.balance(&env.current_contract_address());

        let token_in = if is_a_in { &state.token_a } else { &state.token_b };

        // 2. Execute internal math and cross-contract token transfers
        Self::verify_balance_and_allowance(&env, token_in, &user, amount_in);
        let amount_out = Self::calculate_amount_out(env.clone(), amount_in, is_a_in);

        if amount_out <= 0 {
            panic!("insufficient output amount");
        }

        let client_in = if is_a_in { &client_a } else { &client_b };
        let client_out = if is_a_in { &client_b } else { &client_a };

        client_in.transfer(&user, &env.current_contract_address(), &amount_in);
        client_out.transfer(&env.current_contract_address(), &user, &amount_out);

        // Update reserves to reflect actual transfers
        if is_a_in {
            state.reserve_a = state.reserve_a.saturating_add(amount_in);
            state.reserve_b = state.reserve_b.saturating_sub(amount_out);
        } else {
            state.reserve_b = state.reserve_b.saturating_add(amount_in);
            state.reserve_a = state.reserve_a.saturating_sub(amount_out);
        }
        env.storage().instance().set(&DataKey::State, &state);

        // 3. Capture physical balances again after transfers
        let bal_a_after = client_a.balance(&env.current_contract_address());
        let bal_b_after = client_b.balance(&env.current_contract_address());

        // 4. Assert that the new balance matches the mathematical expectation (Invariant Verification)
        // We scale to 18 decimals to ensure precision parity during the multiplication check.
        let scale_a = 10i128.pow(18 - state.token_a_decimals);
        let scale_b = 10i128.pow(18 - state.token_b_decimals);

        let a_old = bal_a_before.saturating_mul(scale_a);
        let b_old = bal_b_before.saturating_mul(scale_b);
        let a_new = bal_a_after.saturating_mul(scale_a);
        let b_new = bal_b_after.saturating_mul(scale_b);

        // Use U256 to prevent i128 saturation on deep liquidity pools (10^18 * 10^18 = 10^36)
        let k_before = U256::from_i128(&env, a_old).mul(&U256::from_i128(&env, b_old));
        let k_after = U256::from_i128(&env, a_new).mul(&U256::from_i128(&env, b_new));

        // The constant product K must never decrease (it should increase by the collected fee).
        // We use compare because U256 doesn't support standard comparison operators directly in all SDK versions
        if k_after < k_before {
            panic_with_error!(&env, Error::InvariantViolated);
        }

        // Final safety check: internal reserves MUST be physically covered by contract balances.
        if bal_a_after < state.reserve_a || bal_b_after < state.reserve_b {
            panic_with_error!(&env, Error::InvariantViolated);
        }

        amount_out
    }

    /// Remove liquidity from the pool, returning underlying tokens to the user.
    /// Withdrawals are permitted even when deposits are paused, allowing LPs to
    /// rescue funds during an emergency. Only a separate withdrawals_paused flag
    /// (set by admin) can block this function.
    pub fn remove_liquidity(env: Env, user: Address, amount_a: i128, amount_b: i128) {
        user.require_auth();
        Self::require_not_frozen(&env, &user);
        let mut state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");
        if state.withdrawals_paused {
            panic!("withdrawals are paused");
        }
        if amount_a < 0 || amount_b < 0 {
            panic!("amounts must be non-negative");
        }
        if state.reserve_a < amount_a || state.reserve_b < amount_b {
            panic!("insufficient reserves");
        }

        let client_a = token::Client::new(&env, &state.token_a);
        let client_b = token::Client::new(&env, &state.token_b);

        client_a.transfer(&env.current_contract_address(), &user, &amount_a);
        client_b.transfer(&env.current_contract_address(), &user, &amount_b);

        state.reserve_a = state.reserve_a.saturating_sub(amount_a);
        state.reserve_b = state.reserve_b.saturating_sub(amount_b);
        env.storage().instance().set(&DataKey::State, &state);

        // Invariant check: physical balances must back internal reserves
        if client_a.balance(&env.current_contract_address()) < state.reserve_a || 
           client_b.balance(&env.current_contract_address()) < state.reserve_b {
            panic_with_error!(&env, Error::InvariantViolated);
        }
    }

    /// Read the current pool reserve ratio (reserve_a / reserve_b) scaled by 10^7.
    pub fn get_spot_price(env: Env) -> u128 {
        let state: PoolState = env.storage().instance().get(&DataKey::State).expect("Not initialized");

        if state.reserve_b == 0 {
            panic!("reserve_b is zero");
        }

        let reserve_a = state.reserve_a as u128;
        let reserve_b = state.reserve_b as u128;

        reserve_a.saturating_mul(10_000_000) / reserve_b
    }

    /// Calculate the amount of token_in to swap for token_out to achieve a balanced 50/50 
    /// liquidity provision for a single-sided deposit.
    ///
    /// # Requirements:
    /// - Account for a standard 0.3% swap fee (30 basis points).
    /// - Determine the split that results in zero "dust" (leftover tokens) after the swap and deposit.
    ///
    /// # Mathematical Derivation:
    /// For a constant product pool (x * y = k), let:
    /// A = amount_in (total amount user has)
    /// R = reserve_in (current pool reserve of input asset)
    /// s = swap_amount (what we are solving for)
    /// f = fee (0.003 for 0.3%)
    /// g = 1 - f (0.997)
    ///
    /// To avoid dust, the ratio of the user's remaining asset to the new pool reserve
    /// must match the ratio of the received asset to its new pool reserve:
    /// (A - s) / (R + s) = swap_out / (reserve_out - swap_out)
    ///
    /// Since (swap_out / reserve_out_new) = (g * s) / R for a constant product pool:
    /// (A - s) / (R + s) = (g * s) / R
    /// R * (A - s) = g * s * (R + s) 
    /// RA - Rs = gRs + gs^2  =>  gs^2 + R(1+g)s - RA = 0
    ///
    /// Solving for s using the quadratic formula:
    /// s = [-R(1+g) + sqrt((R(1+g))^2 + 4 * g * A * R)] / (2 * g)
    pub fn calculate_single_sided_deposit_split(
        _env: Env,
        amount_in: i128,
        reserve_in: i128,
        reserve_out: i128,
    ) -> i128 {
        if amount_in <= 0 || reserve_in <= 0 || reserve_out <= 0 {
            return 0;
        }

        // Using basis points for precision (B = 10000, fee = 30 for 0.3%)
        let b: i128 = 10000;
        let fee: i128 = 30;
        let gamma: i128 = b.saturating_sub(fee); // 9970 (represents 1-f)
        let sum_b_gamma: i128 = b.saturating_add(gamma); // 19970 (represents 2-f)

        // Quadratic Coefficients for solving: gamma*s^2 + R(1+gamma)*s - AR = 0
        // scaled to maintain integer precision using basis points (b)
        let a_coeff = gamma;
        let b_coeff = reserve_in.saturating_mul(sum_b_gamma);
        let c_coeff_abs = amount_in.saturating_mul(reserve_in).saturating_mul(b);

        // Discriminant: D = b^2 - 4ac
        // Since c is negative (-AR), we add the absolute value: D = b^2 + 4a|c|
        // Note: For 18-decimal tokens, these intermediate values may exceed i128. 
        // Saturating arithmetic is used here as a protective scaffold.
        let term_4ac = (4i128).saturating_mul(a_coeff).saturating_mul(c_coeff_abs);
        let discriminant = b_coeff.saturating_mul(b_coeff).saturating_add(term_4ac);
        let sqrt_d = Self::isqrt(discriminant);

        let numerator = sqrt_d.saturating_sub(b_coeff);
        let denominator = (2i128).saturating_mul(a_coeff);

        if denominator == 0 {
            return 0;
        }

        // Final swap amount in native units
        numerator / denominator
    }

    /// Internal helper: Integer Square Root using Newton's Method.
    fn isqrt(n: i128) -> i128 {
        if n <= 0 {
            return 0;
        }
        let mut x = n;
        let mut y = (x.saturating_add(1)) / 2;
        while y < x {
            x = y;
            let div = n / x;
            y = (x.saturating_add(div)) / 2;
        }
        x
    }

    /// Calculate a volatility fee multiplier based on the ratio of a trade size to the pool reserve.
    ///
    /// This protects Liquidity Providers (LPs) from impermanent loss during extreme market events.
    /// Large trades relative to pool depth cause significant price impact and expose LPs to
    /// arbitrage losses. By increasing the fee for large trades, the protocol compensates LPs
    /// for the elevated risk they bear during high-volatility periods.
    ///
    /// # Multiplier Logic
    /// The return value is scaled by 100 to avoid floating-point arithmetic on-chain:
    /// - `100` represents a multiplier of 1.0 (standard fee, no adjustment)
    /// - `150` represents a multiplier of 1.5 (50% fee increase for high-impact trades)
    ///
    /// # Thresholds
    /// - `trade_size < 1% of pool_reserve`  → multiplier = 1.0 (return 100)
    ///   Small trades have negligible price impact; standard fee applies.
    /// - `trade_size > 5% of pool_reserve`  → multiplier = 1.5 (return 150)
    ///   Large trades cause significant slippage and impermanent loss risk for LPs.
    /// - `1% ≤ trade_size ≤ 5%`             → multiplier = 1.0 (return 100)
    ///   Medium trades fall within the standard fee band.
    ///
    /// # Arguments
    /// * `trade_size`   - The size of the incoming trade (in the input token's native units).
    /// * `pool_reserve` - The current reserve of the input token in the pool.
    ///
    /// # Returns
    /// A `u32` multiplier scaled by 100: either `100` (1.0×) or `150` (1.5×).
    pub fn calculate_volatility_fee_multiplier(trade_size: i128, pool_reserve: i128) -> u32 {
        // Guard: if reserve is zero or inputs are non-positive, return standard multiplier
        if pool_reserve <= 0 || trade_size <= 0 {
            return 100;
        }

        // Check if trade_size > 5% of pool_reserve
        // Equivalent to: trade_size * 100 > pool_reserve * 5
        // Using integer arithmetic to avoid division and floating-point
        if trade_size.saturating_mul(100) > pool_reserve.saturating_mul(5) {
            return 150; // 1.5× multiplier — high-volatility trade, protect LPs
        }

        100 // 1.0× multiplier — standard fee applies
    }
}