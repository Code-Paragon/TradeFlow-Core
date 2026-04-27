# Event Documentation

This document describes the events emitted by the TradeFlow-Core smart contracts, specifically administrative actions for transparency.

## Factory Contract Events

### Pool Created

Emitted when a new liquidity pool is successfully created and deployed. This is the primary event that off-chain indexers should monitor to discover new trading pairs.

*   **Topics**:
    1.  `"PoolCreated"` (Symbol)
    2.  `token_a` (Address) - The first token of the pair (original order)
    3.  `token_b` (Address) - The second token of the pair (original order)
*   **Data**: `pool_address: Address` - The address of the newly deployed pool contract

**Trigger Conditions**:
- Two different token addresses are provided
- Valid fee tier is specified (5, 30, or 100 basis points)
- Pool does not already exist for the token pair
- Factory contract is properly initialized

**Impact**:
- New trading market becomes available
- Frontend can display the new pair to users
- Analytics systems can start tracking the new pool
- Liquidity providers can begin adding liquidity

### Admin Action: Set Fee Recipient

Emitted when the admin updates the address that receives protocol fees.

*   **Topics**:
    1.  `"Admin"` (Symbol)
    2.  `"SetFeeTo"` (Symbol)
*   **Data**: `(old_fee_recipient: Address, new_fee_recipient: Address)`

### Admin Action: Toggle Pool Status

Emitted when the admin changes the status of a specific liquidity pool (e.g., pausing or unpausing).

*   **Topics**:
    1.  `"Admin"` (Symbol)
    2.  `"PoolStatus"` (Symbol)
    3.  `token_a` (Address)
    4.  `token_b` (Address)
*   **Data**: `status: u32`
    *   `0`: Paused
    *   `1`: Active
    *   (Other codes may be defined in the future)

## AMM Pool Events

### Critical: Protocol Emergency Eject

Emitted when the admin executes an emergency eject function to forcefully withdraw all liquidity from a deprecated pool. This is a critical security event that immediately alerts all indexers and monitoring systems.

*   **Topics**:
    1.  `"ProtocolEmergencyEject"` (Symbol)
    2.  `"CRITICAL"` (Symbol) - Initial alert
    3.  `"ProtocolEmergencyEject"` (Symbol)
    4.  `"COMPLETED"` (Symbol) - Completion notification
*   **Data**:
    *   Initial alert: `(pool_address: Address, token_a: Address, token_b: Address, reserve_a: i128, reserve_b: i128)`
    *   Completion: `pool_address: Address`

**Trigger Conditions**:
- Pool must be marked as deprecated (`is_deprecated = true`)
- Must be called by authorized admin address
- Pool must not be locked by reentrancy protection

**Impact**:
- All liquidity is forcefully withdrawn from the pool
- Underlying tokens are returned to LPs based on their snapshot balances
- Pool reserves are reset to zero
- This is an irreversible emergency action