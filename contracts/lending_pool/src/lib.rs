#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env, BytesN, Bytes, symbol_short, Symbol, vec, Val, panic_with_error, IntoVal};

pub mod flash_loan_receiver;
pub use flash_loan_receiver::FlashLoanReceiver;

/// Flash loan fee in basis points (8 bps = 0.08%)
/// This fee compensates Liquidity Providers for temporary risk exposure
/// while generating additional protocol revenue.
const FLASH_LOAN_FEE_BPS: i128 = 8;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    ContractPaused = 2,
    InsufficientLiquidity = 3,
    LoanNotFound = 4,
    LoanAlreadyRepaid = 5,
    LoanDefaulted = 6,
    InsufficientBalance = 7,
    CannotLiquidateHealthyLoan = 8,
    Unauthorized = 9,
    MathOverflow = 10,
    PoolIsPaused = 11,
    EmptyPool = 12,
    TradeSizeTooLarge = 13,
    UnauthorizedBorrower = 14,
}

#[contracttype]
#[derive(Clone)]
pub struct Loan {
    pub id: u64,
    pub borrower: Address,
    pub invoice_id: u64,
    pub principal: i128,
    pub interest: i128,
    pub start_time: u64,
    pub due_date: u64,
    pub is_repaid: bool,
    pub is_defaulted: bool,
}

#[contracttype]
pub enum LoanStatus {
    Active,
    Repaid,
    Defaulted,
}

#[contracttype]
pub enum DataKey {
    Admin,
    TokenAddress, // The address of the USDC token
    Paused,       // Contract pause state
    Loan(u64),    // Maps ID -> Loan
    LoanId,       // Tracks the next available loan ID
    BackendPubkey, // Backend public key for signature verification
    WhitelistActive,
    Whitelisted(Address),
    MaxTradePercentage,
    ApprovedFlashBorrower(Address), // Flash loan borrower whitelist
}

#[contract]
pub struct LendingPool;

#[contractimpl]
impl LendingPool {
    // 1. INITIALIZE: Set the token we are lending (e.g., USDC)
    pub fn init(env: Env, admin: Address, token_address: Address) {
        // Simple check to ensure we don't overwrite
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TokenAddress, &token_address);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().set(&DataKey::WhitelistActive, &true);
        Self::extend_instance_ttl(&env);
    }

    // Helper function to check if contract is paused
    fn check_paused(env: &Env) {
        if env.storage().instance().get(&DataKey::Paused).unwrap_or(false) {
            panic_with_error!(env, Error::ContractPaused);
        }
    }

    // Helper function to check admin authorization
    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized));
        admin.require_auth();
    }

    // PAUSE CONTROL: Set contract pause state (admin only)
    pub fn set_paused(env: Env, paused: bool) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::Paused, &paused);
        Self::extend_instance_ttl(&env);
        env.events().publish((symbol_short!("pause_set"), paused), env.ledger().sequence());
    }

    // SET WHITELIST ACTIVE: Toggle whitelist mechanism (admin only)
    pub fn set_whitelist_active(env: Env, active: bool) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::WhitelistActive, &active);
        Self::extend_instance_ttl(&env);
    }

    // ADD TO WHITELIST: Add address to approved LPs (admin only)
    pub fn add_to_whitelist(env: Env, address: Address) {
        Self::require_admin(&env);
        env.storage().persistent().set(&DataKey::Whitelisted(address.clone()), &true);
        env.storage().persistent().extend_ttl(&DataKey::Whitelisted(address), 535_680, 535_680);
        
        Self::extend_instance_ttl(&env);
    }

    // SET FLASH BORROWER STATUS: Add/remove address from flash loan whitelist (admin only)
    pub fn set_flash_borrower_status(env: Env, borrower_address: Address, is_approved: bool) {
        Self::require_admin(&env);
        
        if is_approved {
            // Add to approved flash borrowers whitelist
            env.storage().persistent().set(&DataKey::ApprovedFlashBorrower(borrower_address.clone()), &true);
            env.storage().persistent().extend_ttl(&DataKey::ApprovedFlashBorrower(borrower_address.clone()), 535_680, 535_680);
            
            // Emit event for adding borrower to whitelist
            env.events().publish((symbol_short!("flash_add"), borrower_address.clone()), env.ledger().sequence());
        } else {
            // Remove from approved flash borrowers whitelist
            if env.storage().persistent().has(&DataKey::ApprovedFlashBorrower(borrower_address.clone())) {
                env.storage().persistent().remove(&DataKey::ApprovedFlashBorrower(borrower_address.clone()));
                
                // Emit event for removing borrower from whitelist
                env.events().publish((symbol_short!("flash_rem"), borrower_address.clone()), env.ledger().sequence());
            }
        }
        
        Self::extend_instance_ttl(&env);
    }

    // GET PAUSE STATE: Check if contract is paused
    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Paused).unwrap_or(false)
    }

    // MAX TRADE PERCENTAGE: Set the max trade percentage (admin only)
    pub fn set_max_trade_percentage(env: Env, percentage: u32) {
        Self::require_admin(&env);
        if percentage > 100 {
            panic_with_error!(&env, Error::MathOverflow);
        }
        env.storage().instance().set(&DataKey::MaxTradePercentage, &percentage);
        Self::extend_instance_ttl(&env);
        env.events().publish((symbol_short!("max_trade"), percentage), env.ledger().sequence());
    }

    // GET MAX TRADE PERCENTAGE
    pub fn get_max_trade_percentage(env: Env) -> u32 {
        // Default to 10% if not set
        env.storage().instance().get(&DataKey::MaxTradePercentage).unwrap_or(10u32)
    }

    // 2. DEPOSIT: LPs add capital to the pool
    pub fn deposit(env: Env, from: Address, amount: i128) {
        Self::check_paused(&env);
        from.require_auth();

        // Check whitelist if active
        if env.storage().instance().get(&DataKey::WhitelistActive).unwrap_or(false) {
            if !env.storage().persistent().has(&DataKey::Whitelisted(from.clone())) {
                panic_with_error!(&env, Error::Unauthorized);
            }
            // Extend TTL for the whitelist entry on access
            env.storage().persistent().extend_ttl(&DataKey::Whitelisted(from.clone()), 535_680, 535_680);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        let client = token::Client::new(&env, &token_addr);

        // Transfer from User -> Contract
        client.transfer(&from, &env.current_contract_address(), &amount);
        
        // (In a real app, we would mint "Pool Share Tokens" here)
        env.events().publish((symbol_short!("deposit"), from), amount);
        Self::extend_instance_ttl(&env);
    }

    // 3. SWAP / BORROW: Withdraw/Borrow against an invoice (Simplified with max trade protection)
    pub fn swap(env: Env, user: Address, amount_in: i128) -> Result<(), Error> {
        if env.storage().instance().get(&DataKey::Paused).unwrap_or(false) {
            return Err(Error::PoolIsPaused);
        }
        user.require_auth();

        // 1. Check total pool reserves
        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        let client = token::Client::new(&env, &token_addr);
        
        let total_reserves = client.balance(&env.current_contract_address());
        if total_reserves == 0 {
            return Err(Error::EmptyPool);
        }

        // 2. Compute max allowed trade size based on configurable percentage
        let max_trade_pct = Self::get_max_trade_percentage(env.clone());
        let max_allowed = total_reserves
            .checked_mul(max_trade_pct as i128)
            .ok_or(Error::MathOverflow)?
            .checked_div(100)
            .ok_or(Error::MathOverflow)?;

        // 3. Validation check against max allowed threshold
        if amount_in > max_allowed {
            // Reverts transaction with specific error type. 
            // Note: Soroban contracterror enums cannot carry payloads.
            // The frontend should catch TradeSizeTooLarge and display appropriate messages.
            return Err(Error::TradeSizeTooLarge);
        }

        if amount_in > total_reserves {
            return Err(Error::InsufficientLiquidity);
        }

        // 4. Transfer funds Contract -> User
        client.transfer(&env.current_contract_address(), &user, &amount_in);

        env.events().publish((symbol_short!("swap"), user), amount_in);
        Self::extend_instance_ttl(&env);
        Ok(())
    }

    // Legacy borrow function mapped to swap logic for compatibility
    pub fn borrow(env: Env, borrower: Address, amount: i128) {
        Self::swap(env.clone(), borrower, amount)
            .unwrap_or_else(|e| panic_with_error!(&env, e));
    }

    // Helper function to calculate interest (5% APY)
    fn calculate_interest(principal: i128, start_time: u64, end_time: u64) -> Result<i128, Error> {
        const YEAR_IN_SECONDS: u64 = 31_536_000; // 365.25 days
        const APY_BPS: u64 = 500; // 5% expressed in basis points
        
        if end_time <= start_time {
            return Ok(0);
        }
        
        // Use checked_sub to prevent underflow
        let duration = end_time.checked_sub(start_time).ok_or(Error::MathOverflow)?;
        
        // Calculate interest using checked_mul and checked_div to prevent overflow
        // principal * APY_BPS * duration / (10_000 * YEAR_IN_SECONDS)
        let interest_part1 = principal.checked_mul(APY_BPS as i128).ok_or(Error::MathOverflow)?;
        let interest_part2 = interest_part1.checked_mul(duration as i128).ok_or(Error::MathOverflow)?;
        
        let denominator = 10_000_i128.checked_mul(YEAR_IN_SECONDS as i128).ok_or(Error::MathOverflow)?;
        let interest = interest_part2.checked_div(denominator).ok_or(Error::MathOverflow)?;
        
        Ok(interest)
    }

    // Helper function to extend storage TTL
    fn extend_instance_ttl(env: &Env) {
        // Extend TTL to 535,680 ledgers (approx 30 days)
        env.storage().instance().extend_ttl(535_680, 535_680);
    }

    // SET BACKEND PUBKEY: Initialize backend public key for signature verification
    pub fn set_backend_pubkey(env: Env, pubkey: BytesN<32>) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::BackendPubkey, &pubkey);
        Self::extend_instance_ttl(&env);
    }

    // CREATE LOAN: Create a new loan record
    pub fn create_loan(env: Env, borrower: Address, invoice_id: u64, principal: i128, due_date: u64) -> u64 {
        Self::check_paused(&env);
        borrower.require_auth();

        let current_time = env.ledger().timestamp();
        
        if due_date <= current_time {
            panic_with_error!(&env, Error::LoanDefaulted);
        }

        let interest = Self::calculate_interest(principal, current_time, due_date)
            .unwrap_or_else(|_| panic_with_error!(&env, Error::MathOverflow));

        // Use checked_add to prevent overflow when generating new loan IDs
        let loan_id_current = env.storage().instance().get(&DataKey::LoanId).unwrap_or(0u64);
        let loan_id = loan_id_current.checked_add(1).unwrap_or_else(|| panic_with_error!(&env, Error::MathOverflow));

        let loan = Loan {
            id: loan_id,
            borrower: borrower.clone(),
            invoice_id,
            principal,
            interest,
            start_time: current_time,
            due_date,
            is_repaid: false,
            is_defaulted: false,
        };

        env.storage().persistent().set(&DataKey::Loan(loan_id), &loan);
        env.storage().persistent().extend_ttl(&DataKey::Loan(loan_id), 535_680, 535_680);
        env.storage().instance().set(&DataKey::LoanId, &loan_id);
        Self::extend_instance_ttl(&env);

        env.events().publish((Symbol::new(&env, "loan_create"), borrower), loan_id);
        loan_id
    }

    // REPAY LOAN: Repay a loan and unlock collateral
    pub fn repay_loan(env: Env, loan_id: u64) {
        Self::check_paused(&env);
        
        let mut loan: Loan = env.storage().persistent().get(&DataKey::Loan(loan_id))
            .unwrap_or_else(|| panic_with_error!(&env, Error::LoanNotFound));
        
        if loan.is_repaid {
            panic_with_error!(&env, Error::LoanAlreadyRepaid);
        }
        
        if loan.is_defaulted {
            panic_with_error!(&env, Error::LoanDefaulted);
        }
        
        loan.borrower.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        let client = token::Client::new(&env, &token_addr);

        let current_time = env.ledger().timestamp();
        let current_interest = Self::calculate_interest(loan.principal, loan.start_time, current_time)
            .unwrap_or_else(|_| panic_with_error!(&env, Error::MathOverflow));
            
        // Use checked_add to prevent overflow when calculating total repayment
        let total_repayment = loan.principal.checked_add(current_interest)
            .unwrap_or_else(|| panic_with_error!(&env, Error::MathOverflow));

        // Check borrower's USDC balance
        let borrower_balance = client.balance(&loan.borrower);
        if borrower_balance < total_repayment {
            panic_with_error!(&env, Error::InsufficientBalance);
        }

        // Transfer repayment from borrower to contract
        client.transfer(&loan.borrower, &env.current_contract_address(), &total_repayment);

        // Update loan status
        loan.is_repaid = true;
        env.storage().persistent().set(&DataKey::Loan(loan_id), &loan);
        env.storage().persistent().extend_ttl(&DataKey::Loan(loan_id), 535_680, 535_680);
        Self::extend_instance_ttl(&env);

        // In a real implementation, we would transfer the NFT back to the borrower
        // For now, we just emit an event
        env.events().publish((Symbol::new(&env, "loan_repaid"), loan.borrower), loan_id);
    }

    // LIQUIDATE: Liquidate a defaulted loan
    pub fn liquidate(env: Env, liquidator: Address, loan_id: u64) {
        Self::check_paused(&env);
        
        let mut loan: Loan = env.storage().persistent().get(&DataKey::Loan(loan_id))
            .unwrap_or_else(|| panic_with_error!(&env, Error::LoanNotFound));
        
        if loan.is_repaid {
            panic_with_error!(&env, Error::LoanAlreadyRepaid);
        }
        
        if loan.is_defaulted {
            panic_with_error!(&env, Error::LoanDefaulted);
        }

        let current_time = env.ledger().timestamp();
        if current_time <= loan.due_date {
            panic_with_error!(&env, Error::CannotLiquidateHealthyLoan);
        }

        liquidator.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        let client = token::Client::new(&env, &token_addr);

        // Transfer principal from liquidator to contract
        client.transfer(&liquidator, &env.current_contract_address(), &loan.principal);

        // Update loan status
        loan.is_defaulted = true;
        env.storage().persistent().set(&DataKey::Loan(loan_id), &loan);
        env.storage().persistent().extend_ttl(&DataKey::Loan(loan_id), 535_680, 535_680);
        Self::extend_instance_ttl(&env);

        // In a real implementation, we would transfer the NFT to the liquidator
        env.events().publish((Symbol::new(&env, "loan_liquid"), liquidator), loan_id);
    }

    // GET LOAN: Retrieve loan details
    pub fn get_loan(env: Env, loan_id: u64) -> Option<Loan> {
        let loan = env.storage().persistent().get(&DataKey::Loan(loan_id));
        if loan.is_some() {
            // Extend TTL on access
            env.storage().persistent().extend_ttl(&DataKey::Loan(loan_id), 535_680, 535_680);
        }
        loan
    }

    // 4. VIEW: Check contract balance
    pub fn get_pool_balance(env: Env) -> i128 {
        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        let client = token::Client::new(&env, &token_addr);
        client.balance(&env.current_contract_address())
    }

    /// Calculates the flash loan fee for a given amount.
    /// The fee is computed as amount * FLASH_LOAN_FEE_BPS / 10000.
    /// Handles precision correctly by multiplying before dividing.
    pub fn calculate_flash_fee(env: Env, amount: i128) -> i128 {
        if amount < 0 {
            panic_with_error!(&env, Error::MathOverflow);
        }
        // amount * 8 / 10000
        amount
            .checked_mul(FLASH_LOAN_FEE_BPS)
            .and_then(|v| v.checked_div(10_000))
            .unwrap_or_else(|| panic_with_error!(&env, Error::MathOverflow))
    }

    /// Executes a flash loan, transferring `amount` to `receiver` and expecting
    /// `amount + calculate_flash_fee(amount)` to be returned before callback returns.
    /// 
    /// # Flash Loan Fee & LP Share Value
    /// A fee of 0.08% (8 bps) is charged. Fees stay in pool reserves, increasing LP share value.
    /// 
    /// # Callback Interface
    /// Receiver must implement [`FlashLoanReceiver`] trait:
    /// `execute_operation(env: Env, amount: i128, fee: i128, params: Bytes)`
    /// Must repay via token.transfer before returning, else entire tx reverts.
    pub fn flash_loan(env: Env, receiver: Address, amount: i128, params: Bytes) {
        Self::check_paused(&env);
        
        // SECURITY: Check if caller is on the approved flash loan borrower whitelist
        if !env.storage().persistent().has(&DataKey::ApprovedFlashBorrower(receiver.clone())) {
            panic_with_error!(&env, Error::UnauthorizedBorrower);
        }
        
        if amount <= 0 {
            panic_with_error!(&env, Error::MathOverflow);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        let client = token::Client::new(&env, &token_addr);
        
        let initial_balance = client.balance(&env.current_contract_address());
        if initial_balance < amount {
            panic_with_error!(&env, Error::InsufficientLiquidity);
        }

        let fee = Self::calculate_flash_fee(env.clone(), amount);

        // Transfer funds to receiver
        client.transfer(&env.current_contract_address(), &receiver, &amount);

        // Invoke the receiver's callback via standard FlashLoanReceiver trait.
        // The receiver must implement `execute_operation(amount: i128, fee: i128, params: Bytes)`
        let args = vec![&env, amount.into_val(&env), fee.into_val(&env), params.into_val(&env)];
        let _res: Val = env.invoke_contract(&receiver, &Symbol::new(&env, "execute_operation"), args);


        // Verify repayment (borrowed_amount + calculated_fee)
        let final_balance = client.balance(&env.current_contract_address());
        let required_balance = initial_balance
            .checked_add(fee)
            .unwrap_or_else(|| panic_with_error!(&env, Error::MathOverflow));
            
        if final_balance < required_balance {
            panic_with_error!(&env, Error::InsufficientBalance);
        }

        env.events().publish((Symbol::new(&env, "flash_loan"), receiver), (amount, fee));
        Self::extend_instance_ttl(&env);
    }
}