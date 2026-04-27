# TradeFlow-Core: Decentralized Trade Finance on Soroban

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Soroban](https://img.shields.io/badge/soroban-ready-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

**TradeFlow-Core** is the smart contract layer for the TradeFlow protocol. It enables Real-World Asset (RWA) tokenization and decentralized factoring on the Stellar network.

## 🏗 Architecture

The system consists of multiple smart contracts:

1.  **`invoice_nft`**: A standard-compliant NFT representing a verified invoice. It holds metadata (IPFS hash, face value, currency, due date).
2.  **`lending_pool`**: An escrow vault where liquidity providers deposit stablecoins (USDC). It accepts `invoice_nft` as collateral to automate loan origination and repayment.
3.  **`factory`**: Factory contract for deploying liquidity pools with specific fee tiers.
4.  **`amm_pool`**: Automated Market Maker pool contract with configurable fee tiers.

---

## 🔖 Contract Versioning

TradeFlow-Core uses a hardcoded version string stored in instance storage to let frontends and indexers identify which contract version they are communicating with.

### Version Format

All versions follow [Semantic Versioning](https://semver.org/) (SemVer):

```
vMAJOR.MINOR.PATCH
```

| Segment | When to increment |
| :--- | :--- |
| **MAJOR** | Breaking change to the public API or storage layout |
| **MINOR** | New functionality added in a backward-compatible way |
| **PATCH** | Backward-compatible bug fixes |

### Current Version

```
v1.0.0
```

### How It Works

The version is defined as a compile-time constant and written to instance storage during `initialize_factory`. It can be read at any time using the public `get_version()` function.

```rust
// Reading the version on-chain (Rust test or cross-contract call)
let version = factory_client.get_version();
// Returns: "v1.0.0"
```

```bash
# Reading the version via Stellar CLI
stellar contract invoke \
  --id <FACTORY_CONTRACT_ID> \
  --network testnet \
  -- get_version
```

### Upgrade Path

When a new version is deployed, update the constant in `lib.rs` before building:

```rust
const CONTRACT_VERSION: &str = "v2.0.0";
```

Then rebuild and redeploy. The new `initialize_factory` call will store the updated string automatically.

---

## 💰 Fee Tiers

The Factory contract supports creating pools with different fee tiers to optimize for various token pair characteristics:

| Fee Tier | Basis Points | Percentage | Use Case |
| :--- | :--- | :--- | :--- |
| **Stable** | 5 | 0.05% | Stablecoin pairs (USDC/USDT, DAI/USDC) |
| **Standard** | 30 | 0.30% | Standard token pairs (ETH/USDC, BTC/USDC) |
| **Volatile** | 100 | 1.00% | Highly volatile exotic pairs |
| **Recovery** | - | - | Emergency admin withdrawal enabled |

## 🔗 Deterministic Address Derivation (#104)

TradeFlow uses Soroban's `deployer().with_current_contract(salt)` for deterministic pool address derivation. This allows off-chain tools to calculate the pool address without querying the Factory.

The `salt` is derived using SHA-256 hashing of the XDR-encoded token addresses:
1. Sort `token_a` and `token_b` lexicographically.
2. Hashing algorithm: `sha256(token_0_xdr + token_1_xdr)`.
3. Deployment: `env.deployer().with_current_contract(salt).deploy(wasm_hash)`.

### Local Pool Address Calculation
External developers can calculate the pool address locally using the token addresses and the factory's contract ID.

---

### Creating a Pool with Specific Fee Tier

```rust
// Create a stablecoin pool with 0.05% fee
let pool_address = factory.create_pool(
    token_a,
    token_b,
    5  // 5 basis points = 0.05%
);

// Create a standard pool with 0.30% fee
let pool_address = factory.create_pool(
    token_a,
    token_b,
    30  // 30 basis points = 0.30%
);

// Create a volatile pool with 1.00% fee
let pool_address = factory.create_pool(
    token_a,
    token_b,
    100  // 100 basis points = 1.00%
);
```

**Important:** Only fee tiers of 5, 30, or 100 basis points are supported. Any other value will cause the transaction to fail.

## 💾 Storage Architecture

To optimize for ledger rent costs and scalability, the protocol uses a tiered storage approach:

- **Instance Storage**: Global configuration settings (Admin, Paused state, Version) and counters (Loan IDs).
- **Persistent Storage**: High-cardinality user data (Loans, Invoices, Whitelists).
- **Temporary Storage**: Used for transient data where applicable.

## ⛓️ Live Testnet Deployments

The following contracts are currently active for frontend integration and testing.

| Contract Name | Network | Contract ID |
| :--- | :--- | :--- |
| **Invoice NFT** | Testnet | `CCYU3LOQI34VHVN3ZOSEBHHKL4YK36FMTOEGLRYDUDRGS7JOLLRKCEQM` |
| **Lending Pool** | Testnet | `CDVJMVPLZJKXSJFDY5AWBOUIRN73BKU2SG674MQDH4GRE6BGBPQD33IQ` |

- **Network Passphrase:** `Test SDF Network ; September 2015`
- **RPC Endpoint:** `https://soroban-testnet.stellar.org`

## � Stellar Asset Contract (SAC) Integration

TradeFlow-Core now natively integrates with the official Stellar Asset Contract (SAC) for real-world token operations on Mainnet. All token interactions use `soroban_sdk::token::Client` for secure, standardized cross-contract calls.

### SAC Client Usage

All contracts create token clients to interact with SAC-compliant tokens:

```rust
use soroban_sdk::token;

// Create a token client for any SAC-compliant token
let token_client = token::Client::new(&env, &token_address);

// Check balance
let balance = token_client.balance(&user_address);

// Check allowance
let allowance = token_client.allowance(&user_address, &spender_address);

// Transfer tokens
token_client.transfer(&from, &to, &amount);

// Approve spending
token_client.approve(&owner, &spender, &amount, &expiration_ledger);
```

### Supported Token Operations

| Operation | Function | Description |
| :--- | :--- | :--- |
| **Balance Check** | `balance(address)` | Query token balance for any address |
| **Allowance Check** | `allowance(owner, spender)` | Check approved spending limit |
| **Transfer** | `transfer(from, to, amount)` | Move tokens between addresses |
| **Approve** | `approve(owner, spender, amount, expiry)` | Set spending allowance |
| **Decimals** | `decimals()` | Get token decimal precision |

### Adding New Assets to Protocol

To add a new token asset to TradeFlow:

1. **Deploy SAC-Compatible Token**: Ensure token follows Stellar Asset Contract interface
2. **Get Token Address**: Obtain the contract address after deployment
3. **Create Liquidity Pool**: Use Factory to create pool with existing token
   ```rust
   factory.create_pool(
       new_token_address,
       existing_token_address,
       30  // 0.30% fee tier
   );
   ```
4. **Provide Initial Liquidity**: Add tokens to enable trading
   ```rust
   pool.provide_liquidity(
       user_address,
       new_token_address,
       existing_token_address,
       amount_new_token,
       amount_existing_token,
       min_shares
   );
   ```

### Mainnet Considerations

- **Real Token Transfers**: All operations move actual tokens on Stellar ledger
- **Gas Costs**: Each SAC operation consumes gas for cross-contract calls
- **Security**: Built-in reentrancy protection and balance checks
- **Precision**: Automatic decimal handling for different token precisions

### Testing with Mock Tokens

For local testing, contracts use `StellarAssetClient` for token minting:

```rust
// Test environment setup
token::StellarAssetClient::new(&env, &token_address).mint(&user, &amount);
token::Client::new(&env, &token_address).approve(&user, &contract, &amount, &expiry);
```

---

## �🚀 Quick Start

### Prerequisites
- Rust & Cargo (latest stable)
- Stellar CLI v25.1.0+ (`cargo install stellar-cli`)
- WASM Target: `rustup target add wasm32v1-none`

### Build & Test
```bash
# Build all contracts (optimized for WASM)
stellar contract build

# Run the test suite
cargo test
```