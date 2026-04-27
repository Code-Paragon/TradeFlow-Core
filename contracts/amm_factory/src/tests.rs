#![cfg(test)]

use crate::{FactoryContract, FactoryContractClient};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

#[test]
fn test_initialize_factory() {
    let env = Env::default();
    let factory_id = env.register_contract(None, FactoryContract);
    let client = FactoryContractClient::new(&env, &factory_id);

    let admin = Address::generate(&env);
    let fee_to = Address::generate(&env);
    // Create a dummy hash for testing
    let wasm_hash = BytesN::from_array(&env, &[0; 32]);

    // Initialize
    client.initialize_factory(&admin, &fee_to, &wasm_hash);

    // Create random token addresses
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    // Check that pool does not exist yet
    assert_eq!(client.get_pool(&token_a, &token_b), None);

    // Note: We cannot test create_pool here easily without registering 
    // a valid contract WASM code for the pool, but the logic 
    // for deployment is implemented in lib.rs.
}

#[test]
fn test_admin_actions() {
    let env = Env::default();
    env.mock_all_auths();

    let factory_id = env.register_contract(None, FactoryContract);
    let client = FactoryContractClient::new(&env, &factory_id);

    let admin = Address::generate(&env);
    let fee_to = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[0; 32]);

    client.initialize_factory(&admin, &fee_to, &wasm_hash);

    let new_fee_to = Address::generate(&env);
    client.set_fee_recipient(&new_fee_to);
}

#[test]
fn test_pair_exists_returns_false_for_nonexistent_pair() {
    let env = Env::default();
    let factory_id = env.register_contract(None, FactoryContract);
    let client = FactoryContractClient::new(&env, &factory_id);

    let admin = Address::generate(&env);
    let fee_to = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[0; 32]);

    client.initialize_factory(&admin, &fee_to, &wasm_hash);

    // Create random token addresses
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    // Pool doesn't exist, so pair_exists should return false
    assert_eq!(client.pair_exists(&token_a, &token_b), false);
}

#[test]
fn test_pair_exists_returns_false_for_reversed_tokens() {
    let env = Env::default();
    let factory_id = env.register_contract(None, FactoryContract);
    let client = FactoryContractClient::new(&env, &factory_id);

    let admin = Address::generate(&env);
    let fee_to = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[0; 32]);

    client.initialize_factory(&admin, &fee_to, &wasm_hash);

    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    // Check both orderings - should both return false for non-existent pair
    assert_eq!(client.pair_exists(&token_a, &token_b), false);
    assert_eq!(client.pair_exists(&token_b, &token_a), false);
}

#[test]
fn test_pair_exists_on_uninitialized_factory() {
    let env = Env::default();
    let factory_id = env.register_contract(None, FactoryContract);
    let client = FactoryContractClient::new(&env, &factory_id);

    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    // Factory not initialized - should return false (empty map)
    assert_eq!(client.pair_exists(&token_a, &token_b), false);
}

#[test]
fn test_propose_and_accept_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let factory_id = env.register_contract(None, FactoryContract);
    let client = FactoryContractClient::new(&env, &factory_id);

    let admin = Address::generate(&env);
    let fee_to = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[0; 32]);
    client.initialize_factory(&admin, &fee_to, &wasm_hash);

    let new_admin = Address::generate(&env);

    // Current admin proposes a new admin
    client.propose_admin(&new_admin);

    // New admin accepts — ownership transfers
    client.accept_admin();
}

#[test]
fn test_accept_admin_panics_with_no_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let factory_id = env.register_contract(None, FactoryContract);
    let client = FactoryContractClient::new(&env, &factory_id);

    let admin = Address::generate(&env);
    let fee_to = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[0; 32]);
    client.initialize_factory(&admin, &fee_to, &wasm_hash);

    // No proposal made — accept_admin must fail
    let result = client.try_accept_admin();
    assert!(result.is_err(), "accept_admin must fail when no proposal exists");
}