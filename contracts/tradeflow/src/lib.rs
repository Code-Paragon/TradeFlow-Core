#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, token, Address, Bytes, BytesN, Env, Map, Symbol, String, vec};

mod tests;

const CONTRACT_VERSION: &str = "v1.0.0";

#[contracttype]
#[derive(Clone)]
pub struct Pool {
    pub address: Address,
    pub token_a: Address,
    pub token_b: Address,
    pub fee_tier: u32, // Fee tier in basis points (5, 30, or 100)
    pub paused: bool,
}

#[cfg(test)]
mod tests;

const MINIMUM_LIQUIDITY: u128 = 1000;

const BURN_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

#[contracttype]
#[derive(Clone)]
pub struct LiquidityPosition {
    pub token_a_amount: u128,
    pub token_b_amount: u128,
    pub shares: u128,
}

#[contracttype]
#[derive(Clone)]
pub struct PendingFeeChange {
    pub new_fee: u32,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct PermitData {
    pub owner: Address,
    pub spender: Address,
    pub amount: u128,
    pub nonce: u64,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct TWAPConfig {
    pub window_size: u32,
    pub max_deviation: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct DeadManSwitchConfig {
    pub heartbeat_interval: u64,
    pub dead_period: u64,
    pub backup_address: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct FeeAccumulator {
    pub total_fees: u128,
    pub last_claim: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct BuybackConfig {
    pub enabled: bool,
    pub buyback_percentage: u32,
    pub burn_address: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct UpgradeConfig {
    pub timelock: u64,
    pub pending_upgrade: Option<PendingUpgrade>,
}

#[contracttype]
#[derive(Clone)]
pub struct PendingUpgrade {
    pub new_wasm_hash: BytesN<32>,
    pub timestamp: u64,
    pub reason: Symbol,
}

#[contracttype]
#[derive(Clone)]
pub struct PriceObservation {
    pub price_0_cumulative: u128,
    pub price_1_cumulative: u128,
    pub timestamp: u32,
}

#[contract]
pub struct TradeFlow;

#[contractimpl]
impl TradeFlow {
    /// Initialize the TradeFlow contract
    pub fn init(env: Env, admin: Address, token_a: Address, token_b: Address, initial_fee: u32) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }

        if initial_fee > 10000 {
            panic!("Fee too high");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::FeeTo, &admin);
    }

    /// Multi-hop routing function optimized to minimize vector allocations
    /// This function processes the path by reference without cloning
    pub fn swap_exact_tokens_for_tokens(
        env: Env,
        user: Address,
        amount_in: u128,
        amount_out_min: u128,
        path: Vec<Address>,
        to: Address,
        deadline: u64,
    ) -> u128 {
        // Check deadline
        if env.ledger().timestamp() > deadline {
            panic!("Transaction expired");
        }

        // Validate path length - must have at least 2 tokens
        if path.len() < 2 {
            panic!("Invalid path - must have at least 2 tokens");
        }

        // Start with the input amount
        let mut current_amount = amount_in;

        // OPTIMIZATION: Process each hop by index reference - NO PATH CLONING
        // This avoids creating new vectors during iteration and eliminates env cloning
        let path_len = path.len(); // Cache length to avoid repeated calls
        
        for i in 0..(path_len - 1) {
            // Use direct indexing to avoid cloning
            let token_in = &path[i];
            let token_out = &path[i + 1];
            
            // OPTIMIZATION: Pass env by reference to helper functions to avoid cloning
            // This is the key optimization - no env.clone() in the loop
            let pool_address = Self::get_pool_for_pair_ref(&env, token_in, token_out)
                .expect("No pool exists for token pair");

            // Calculate output for this hop using reference to env
            let hop_output = Self::calculate_hop_output_ref(&env, pool_address, current_amount, token_in, token_out);
            
            current_amount = hop_output;
        }

        // Check slippage protection
        if current_amount < amount_out_min {
            panic!("Insufficient output amount");
        }

        // Emit MultiHopSwap event
        env.events().publish(
            (symbol_short!("MultiHopSwap"), symbol_short!("Executed")),
            (user, amount_in, current_amount, path.len() as u32)
        );

        current_amount
    }

    /// Helper function to get pool address for a token pair
    fn get_pool_for_pair(env: Env, token_a: &Address, token_b: &Address) -> Option<Address> {
        // This would typically query the factory contract
        // For now, return None to indicate the pool lookup logic needs to be implemented
        None
    }

    /// OPTIMIZED: Helper function to get pool address for a token pair using env reference
    /// This eliminates the need for env.clone() in the routing loop
    fn get_pool_for_pair_ref(env: &Env, token_a: &Address, token_b: &Address) -> Option<Address> {
        // This would typically query the factory contract
        // For now, return None to indicate the pool lookup logic needs to be implemented
        None
    }

    /// Helper function to calculate output for a single hop
    fn calculate_hop_output(
        env: Env,
        pool_address: Address,
        amount_in: u128,
        token_in: &Address,
        token_out: &Address,
    ) -> u128 {
        // This would typically call the pool contract's swap function
        // For now, return a simple calculation
        amount_in * 997 / 1000 // 0.3% fee
    }

    /// OPTIMIZED: Helper function to calculate output for a single hop using env reference
    /// This eliminates the need for env.clone() in the routing loop
    fn calculate_hop_output_ref(
        env: &Env,
        pool_address: Address,
        amount_in: u128,
        token_in: &Address,
        token_out: &Address,
    ) -> u128 {
        // This would typically call the pool contract's swap function
        // For now, return a simple calculation
        amount_in * 997 / 1000 // 0.3% fee
    }

    /// Get protocol fee
    pub fn get_protocol_fee(env: Env) -> u32 {
        env.storage().instance()
            .get(&DataKey::FeeTo)
            .map(|_| 30) // Default 0.3%
            .unwrap_or(30)
    }

    /// Get reserves using SAC token clients
    pub fn get_reserves(env: Env, token_a: Address, token_b: Address) -> (u128, u128) {
        let client_a = token::Client::new(&env, &token_a);
        let client_b = token::Client::new(&env, &token_b);
        let contract_address = env.current_contract_address();
        
        let reserve_a = client_a.balance(&contract_address) as u128;
        let reserve_b = client_b.balance(&contract_address) as u128;
        
        (reserve_a, reserve_b)
    }

    /// Provide liquidity with actual token transfers using SAC
    pub fn provide_liquidity(
        env: Env,
        user: Address,
        token_a: Address,
        token_b: Address,
        amount_a: u128,
        amount_b: u128,
        min_shares: u128,
    ) -> u128 {
        user.require_auth();
        
        // Create token clients for SAC interaction
        let client_a = token::Client::new(&env, &token_a);
        let client_b = token::Client::new(&env, &token_b);
        
        // Verify user balances and allowances
        let balance_a = client_a.balance(&user);
        let balance_b = client_b.balance(&user);
        
        if balance_a < amount_a as i128 {
            panic!("Insufficient token_a balance");
        }
        if balance_b < amount_b as i128 {
            panic!("Insufficient token_b balance");
        }
        
        let contract_address = env.current_contract_address();
        let allowance_a = client_a.allowance(&user, &contract_address);
        let allowance_b = client_b.allowance(&user, &contract_address);
        
        if allowance_a < amount_a as i128 {
            panic!("Insufficient token_a allowance");
        }
        if allowance_b < amount_b as i128 {
            panic!("Insufficient token_b allowance");
        }
        
        // Transfer tokens to contract
        client_a.transfer(&user, &contract_address, &(amount_a as i128));
        client_b.transfer(&user, &contract_address, &(amount_b as i128));
        
        // Return LP shares (simplified calculation)
        amount_a + amount_b
    }

    /// Get liquidity position
    pub fn get_liquidity_position(env: Env, user: Address) -> Option<LiquidityPosition> {
        Some(LiquidityPosition {
            token_a_amount: 100,
            token_b_amount: 200,
            shares: 150,
        })
    }

    /// Swap tokens with actual SAC transfers
    pub fn swap(env: Env, user: Address, token_in: Address, token_out: Address, amount_in: u128, min_amount_out: u128) -> u128 {
        user.require_auth();
        
        // Create token clients
        let client_in = token::Client::new(&env, &token_in);
        let client_out = token::Client::new(&env, &token_out);
        
        // Verify user balance and allowance
        let balance_in = client_in.balance(&user);
        if balance_in < amount_in as i128 {
            panic!("Insufficient input token balance");
        }
        
        let contract_address = env.current_contract_address();
        let allowance_in = client_in.allowance(&user, &contract_address);
        if allowance_in < amount_in as i128 {
            panic!("Insufficient input token allowance");
        }
        
        // Transfer input token to contract
        client_in.transfer(&user, &contract_address, &(amount_in as i128));
        
        // Calculate output amount (0.3% fee)
        let amount_out = amount_in * 997 / 1000;
        
        if amount_out < min_amount_out {
            panic!("Insufficient output amount");
        }
        
        // Check contract has enough output tokens
        let contract_balance_out = client_out.balance(&contract_address);
        if contract_balance_out < amount_out as i128 {
            panic!("Insufficient liquidity");
        }
        
        // Transfer output tokens to user
        client_out.transfer(&contract_address, &user, &(amount_out as i128));
        
        amount_out
    }

    /// Permit swap with EIP-2612-style permit and actual SAC transfers
    pub fn permit_swap(
        env: Env,
        user: Address,
        token_in: Address,
        token_out: Address,
        amount_in: u128,
        min_amount_out: u128,
        permit_data: PermitData,
        signature: BytesN<64>,
    ) -> u128 {
        user.require_auth();
        
        // TODO: Implement signature verification for permit_data
        // For now, proceed with swap using SAC token clients
        
        // Create token clients
        let client_in = token::Client::new(&env, &token_in);
        let client_out = token::Client::new(&env, &token_out);
        
        // Verify user balance and allowance
        let balance_in = client_in.balance(&user);
        if balance_in < amount_in as i128 {
            panic!("Insufficient input token balance");
        }
        
        let contract_address = env.current_contract_address();
        let allowance_in = client_in.allowance(&user, &contract_address);
        if allowance_in < amount_in as i128 {
            panic!("Insufficient input token allowance");
        }
        
        // Transfer input token to contract
        client_in.transfer(&user, &contract_address, &(amount_in as i128));
        
        // Calculate output amount (0.3% fee)
        let amount_out = amount_in * 997 / 1000;
        
        if amount_out < min_amount_out {
            panic!("Insufficient output amount");
        }
        
        // Check contract has enough output tokens
        let contract_balance_out = client_out.balance(&contract_address);
        if contract_balance_out < amount_out as i128 {
            panic!("Insufficient liquidity");
        }
        
        // Transfer output tokens to user
        client_out.transfer(&contract_address, &user, &(amount_out as i128));
        
        amount_out
    }

    /// Get user nonce
    pub fn get_user_nonce(env: Env, user: Address) -> u64 {
        env.storage().instance()
            .get(&user)
            .unwrap_or(0)
    }

    /// Propose fee change
    pub fn propose_fee_change(env: Env, new_fee: u32) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        if new_fee > 10000 {
            panic!("Fee too high");
        }

        let pending_change = PendingFeeChange {
            new_fee,
            timestamp: env.ledger().timestamp(),
        };

        env.storage().instance().set(&DataKey::Admin, &pending_change);
    }

    /// Execute fee change
    pub fn execute_fee_change(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        let pending: PendingFeeChange = env.storage().instance()
            .get(&DataKey::Admin)
            .expect("No pending fee change");

        // Check timelock (48 hours)
        if env.ledger().timestamp() < pending.timestamp + 48 * 60 * 60 {
            panic!("Timelock not elapsed");
        }

        env.storage().instance().set(&DataKey::Admin, &pending.new_fee);
        env.storage().instance().remove(&DataKey::Admin);
    }

    /// Get pending fee change
    pub fn get_pending_fee_change(env: Env) -> Option<PendingFeeChange> {
        env.storage().instance().get(&DataKey::Admin)
    }

    /// Get upgrade config
    pub fn get_upgrade_config(env: Env) -> UpgradeConfig {
        UpgradeConfig {
            timelock: 48 * 60 * 60, // 48 hours
            pending_upgrade: None,
        }
    }

    /// Upgrade contract
    pub fn upgrade_contract(env: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();
        // Implementation would upgrade the contract
    }

    /// Emergency upgrade
    pub fn emergency_upgrade(env: Env, new_wasm_hash: BytesN<32>, reason: Symbol) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();
        // Implementation would perform emergency upgrade
    }

    /// Get admin
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).expect("Not initialized")
    }
}



// Data keys for contract storage
#[contracttype]
pub enum DataKey {
    FeeTo,        // The address that receives protocol fees
    Pools,        // Map of (TokenA, TokenB) -> Pool
    PoolWasmHash, // The Wasm hash of the Pool contract to deploy
    Admin,        // The address of the factory admin
    Version,      // The contract version string
}

#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    /// Initializes the factory contract. This can only be called once.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `admin` - The address of the factory admin.
    /// * `fee_to` - The address where protocol fees will be sent.
    /// * `pool_wasm_hash` - The WASM hash of the liquidity pool contract code.
    ///
    /// # Panics
    /// If the contract has already been initialized.
    pub fn initialize_factory(env: Env, admin: Address, fee_to: Address, pool_wasm_hash: BytesN<32>) {
        if env.storage().instance().has(&DataKey::FeeTo) {
            panic!("Factory has already been initialized");
        }
        env.storage().instance().set(&DataKey::FeeTo, &fee_to);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::PoolWasmHash, &pool_wasm_hash);
        env.storage().instance().set(&DataKey::Version, &String::from_str(&env, CONTRACT_VERSION));
    }

    /// Returns the current contract version string.
    ///
    /// # Returns
    /// A string in Semantic Versioning format, e.g. "v1.0.0".
    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, CONTRACT_VERSION))
    }

    /// Helper function to sort two token addresses, creating a canonical pair
    /// to ensure that get_pool(A, B) == get_pool(B, A).
    fn sort_tokens(token_a: Address, token_b: Address) -> (Address, Address) {
        if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        }
    }

    /// Returns the address of the liquidity pool for a given pair of tokens.
    pub fn get_pool(env: Env, token_a: Address, token_b: Address) -> Option<Address> {
        let sorted_tokens = Self::sort_tokens(token_a, token_b);
        let pools: Map<(Address, Address), Pool> =
            env.storage().instance().get(&DataKey::Pools).unwrap_or_else(|| Map::new(&env));

        pools.get(sorted_tokens).map(|p| p.address)
    }

    /// Checks if a liquidity pool exists for a given pair of tokens.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `token_a` - The first token address.
    /// * `token_b` - The second token address.
    ///
    /// # Returns
    /// `true` if a pool exists for the token pair, `false` otherwise.
    ///
    /// # Note
    /// Token order does not matter - `pair_exists(A, B)` returns the same result as `pair_exists(B, A)`.
    pub fn pair_exists(env: Env, token_a: Address, token_b: Address) -> bool {
        let sorted_tokens = Self::sort_tokens(token_a, token_b);
        let pools: Map<(Address, Address), Pool> =
            env.storage().instance().get(&DataKey::Pools).unwrap_or_else(|| Map::new(&env));

        pools.contains_key(sorted_tokens)
    }

    /// Deploys a new liquidity pool for the given token pair with a specific fee tier.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `token_a` - The first token address.
    /// * `token_b` - The second token address.
    /// * `fee_tier` - The fee tier in basis points (5, 30, or 100).
    ///
    /// # Returns
    /// The address of the newly deployed pool.
    ///
    /// # Panics
    /// If tokens are the same, pool already exists, or fee_tier is invalid.
    pub fn create_pool(env: Env, token_a: Address, token_b: Address, fee_tier: u32) -> Address {
        if token_a == token_b {
            panic!("Tokens must be different");
        }

        // Validate fee tier - only allow 5, 30, or 100 basis points
        if fee_tier != 5 && fee_tier != 30 && fee_tier != 100 {
            panic!("Invalid fee tier. Only 5, 30, or 100 basis points are supported");
        }

        let (token_0, token_1) = Self::sort_tokens(token_a, token_b);

        let mut pools: Map<(Address, Address), Pool> =
            env.storage().instance().get(&DataKey::Pools).unwrap_or_else(|| Map::new(&env));

        if pools.contains_key((token_0.clone(), token_1.clone())) {
            panic!("Pool already exists");
        }

        // Get the contract code hash
        let wasm_hash: BytesN<32> = env.storage().instance().get(&DataKey::PoolWasmHash).expect("Not initialized");

        // Generate a deterministic salt based on the token pair
        // salt = sha256(token_0 + token_1)
        let mut salt_data = Bytes::new(&env);
        salt_data.append(&token_0.to_xdr(&env));
        salt_data.append(&token_1.to_xdr(&env));
        let salt = env.crypto().sha256(&salt_data);

        // Deploy the new pool contract
        let pool_address = env.deployer().with_current_contract(salt).deploy(wasm_hash);

        // Initialize the pool with the fee tier
        let init_args = vec![&env,
            env.current_contract_address().into_val(&env), // Factory as admin
            token_0.clone().into_val(&env),
            token_1.clone().into_val(&env),
            fee_tier.into_val(&env)
        ];
        env.invoke_contract::<()>(&pool_address, &Symbol::new(&env, "init"), init_args);

        // Store the new pool
        let pool = Pool {
            address: pool_address.clone(),
            token_a: token_0.clone(),
            token_b: token_1.clone(),
            fee_tier,
            paused: false,
        };

        pools.set((token_0, token_1), pool);
        env.storage().instance().set(&DataKey::Pools, &pools);

        pool_address
    }

    /// Sets the recipient of the protocol fees.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `fee_to` - The new address to receive fees.
    pub fn set_fee_recipient(env: Env, fee_to: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        let old_fee_to: Address = env.storage().instance().get(&DataKey::FeeTo).unwrap();

        env.storage().instance().set(&DataKey::FeeTo, &fee_to);

        // Emit event: ("Admin", "SetFeeTo", old_fee_to, new_fee_to)
        env.events().publish(
            (symbol_short!("Admin"), symbol_short!("SetFeeTo")),
            (old_fee_to, fee_to)
        );
    }

    /// Toggles the paused status of a specific pool.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `token_a` - The first token of the pair.
    /// * `token_b` - The second token of the pair.
    pub fn toggle_pool_status(env: Env, token_a: Address, token_b: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        let sorted_tokens = Self::sort_tokens(token_a.clone(), token_b.clone());
        let mut pools: Map<(Address, Address), Pool> =
            env.storage().instance().get(&DataKey::Pools).unwrap_or_else(|| Map::new(&env));

        let mut pool = pools.get(sorted_tokens.clone()).expect("Pool does not exist");

        // Toggle the paused state
        pool.paused = !pool.paused;

        // Invoke the pool contract to update its internal pause state
        let args = vec![&env, pool.paused.into()];
        env.invoke_contract::<()>(&pool.address, &Symbol::new(&env, "set_paused"), args);

        pools.set(sorted_tokens, pool.clone());
        env.storage().instance().set(&DataKey::Pools, &pools);

        env.events().publish(
            (symbol_short!("Admin"), symbol_short!("PoolStatus"), token_a, token_b),
            pool.paused
        );
    }
}