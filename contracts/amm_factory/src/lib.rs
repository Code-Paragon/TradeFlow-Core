#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env, IntoVal, Map, Symbol, vec, xdr::ToXdr};

mod tests;

#[contracttype]
#[derive(Clone)]
pub struct Pool {
    pub address: Address,
    pub token_a: Address,
    pub token_b: Address,
    pub fee_tier: u32, // Fee tier in basis points (5, 30, or 100)
    pub paused: bool,
}

#[contracttype]
pub enum DataKey {
    FeeTo, // The address that receives protocol fees
    Pools, // Map of (TokenA, TokenB) -> Pool
    PoolWasmHash, // The Wasm hash of the Pool contract to deploy
    Admin, // The address of the factory admin
    PendingAdmin, // The address of the proposed new admin (two-step transfer)
    TotalPools, // The total number of pools deployed
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
        env.storage().instance().set(&DataKey::TotalPools, &0u32);
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
    /// Token order doesn't matter - `pair_exists(A, B)` returns the same result as `pair_exists(B, A)`.
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
        salt_data.append(&token_0.clone().to_xdr(&env));
        salt_data.append(&token_1.clone().to_xdr(&env));
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

        // Increment total pools counter for UI metrics and pagination logic
        let mut total_pools: u32 = env.storage().instance().get(&DataKey::TotalPools).unwrap_or(0);
        total_pools += 1;
        env.storage().instance().set(&DataKey::TotalPools, &total_pools);

        // Emit PoolCreated event
        env.events().publish(
            (symbol_short!("PoolCreated"), token_a, token_b),
            pool_address.clone()
        );

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

    /// Proposes a new admin address for a two-step ownership transfer.
    /// Only the current admin can call this. The proposed address must then call
    /// `accept_admin` to complete the transfer, preventing accidental key burns.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `new_admin` - The address being proposed as the new admin.
    pub fn propose_admin(env: Env, new_admin: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        env.storage().instance().set(&DataKey::PendingAdmin, &new_admin);

        env.events().publish(
            (symbol_short!("Admin"), symbol_short!("Proposed")),
            new_admin
        );
    }

    /// Completes the two-step admin ownership transfer.
    /// Must be called by the address previously set via `propose_admin`.
    /// This confirms the new admin's key is accessible before relinquishing control.
    ///
    /// # Panics
    /// If there is no pending admin proposal, or the caller is not the pending admin.
    pub fn accept_admin(env: Env) {
        let pending_admin: Address = env.storage().instance()
            .get(&DataKey::PendingAdmin)
            .expect("No pending admin proposal");
        pending_admin.require_auth();

        let old_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");

        env.storage().instance().set(&DataKey::Admin, &pending_admin);
        env.storage().instance().remove(&DataKey::PendingAdmin);

        // Emit AdminTransferred event so indexers and frontends can track ownership changes
        env.events().publish(
            (symbol_short!("Admin"), Symbol::new(&env, "Transferred")),
            (old_admin, pending_admin)
        );
    }

    /// Toggles the status of a specific pool.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `token_a` - The first token of the pair.
    /// * `token_b` - The second token of the pair.
    /// * `status` - The new status (e.g., 0 = Paused, 1 = Active).
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
            (symbol_short!("Admin"), Symbol::new(&env, "PoolStatus"), token_a, token_b),
            pool.paused
        );
    }

    /// Returns the total number of pools created by the factory.
    /// This is used for UI metrics and pagination logic.
    pub fn get_all_pools_length(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::TotalPools).unwrap_or(0)
    }
}