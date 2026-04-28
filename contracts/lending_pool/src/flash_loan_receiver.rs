//! Flash loan receiver trait for standard cross-contract callback interface.
/// External borrower contracts must implement this trait to receive flash loans.
///
/// # Requirements
/// - Transfer `amount + fee` back to the lending pool before returning.
/// - Revert/panic if unable to repay.
/// - No return value expected (void function).

use soroban_sdk::{Env, Bytes};

#[soroban_sdk::contracttrait]
pub trait FlashLoanReceiver {
    /// Callback executed by lending pool after transferring `amount` tokens.
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `amount` - Borrowed amount (must repay fully)
    /// * `fee` - Flash loan fee (0.08% = 8 bps)
    /// * `params` - Caller-provided params (user data)
    ///
    /// # Panics
    /// Panics if unable to repay `amount + fee` before returning.
    fn execute_operation(env: Env, amount: i128, fee: i128, params: Bytes);
}

