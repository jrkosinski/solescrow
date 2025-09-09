use anchor_lang::prelude::*;

/// Escrow status enumeration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum EscrowStatus {
    /// Escrow has been created, but no payment has been made
    Pending = 0,
    /// Escrow has been created and at least some payment has been made  
    Active = 1,
    /// Escrow has been either refunded or released
    Completed = 2,
    /// Escrow has an arbitration proposal pending
    Arbitration = 3,
}

impl Default for EscrowStatus {
    fn default() -> Self {
        EscrowStatus::Pending
    }
}

/// Currency type enumeration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum CurrencyType {
    /// Native SOL
    Native = 0,
    /// SPL Token
    SplToken = 1,
}

impl Default for CurrencyType {
    fn default() -> Self {
        CurrencyType::Native
    }
}

/// Release assent type for asymmetric escrows
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum ReleaseAssentType {
    Payer = 0,
    Receiver = 1,
}

/// Escrow party data structure
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct EscrowParty {
    /// Party's wallet address
    pub addr: Pubkey,
    /// Currency mint address (Pubkey::default() for native SOL)
    pub currency: Pubkey,
    /// Type of currency
    pub currency_type: CurrencyType,
    /// Required amount for this party
    pub amount: u64,
    /// Amount refunded to this party
    pub amount_refunded: u64,
    /// Amount released to the other party
    pub amount_released: u64,
    /// Amount paid by this party
    pub amount_paid: u64,
    /// Whether this party has given release consent
    pub released: bool,
}

/// Asymmetrical escrow account
/// 
/// Asymmetrical escrow contract for managing exchanges between on-chain assets and off-chain deliverables.
/// 
/// An asymmetrical escrow involves one party paying an on-chain asset (native SOL or SPL token) in exchange 
/// for an off-chain asset or service (such as real-world assets, digital goods, or services). Since the 
/// off-chain component cannot be verified programmatically, arbitration mechanisms are essential for dispute 
/// resolution when parties disagree about delivery or quality.
#[account]
#[derive(Debug)]
pub struct AsymEscrow {
    /// Unique identifier for the escrow (derived from creator + nonce)
    pub id: [u8; 32],
    /// Payer party information
    pub payer: EscrowParty,
    /// Receiver party information  
    pub receiver: EscrowParty,
    /// Timestamp when the escrow was created
    pub timestamp: i64,
    /// Timestamp when the escrow period begins (0 = immediate)
    pub start_time: i64,
    /// Timestamp when the escrow period ends (0 = no expiry)
    pub end_time: i64,
    /// Current escrow status
    pub status: EscrowStatus,
    /// Whether the escrow has been released
    pub released: bool,
    /// Fee in basis points (bps)
    pub fee_bps: u16,
    /// Escrow creator (for PDA derivation)
    pub creator: Pubkey,
    /// Nonce for unique escrow generation
    pub nonce: u64,
    /// Bump seed for PDA
    pub bump: u8,
}

impl AsymEscrow {
    /// Calculate space needed for account
    pub const fn space() -> usize {
        8 + // discriminator
        32 + // id
        200 + // payer (EscrowParty)
        200 + // receiver (EscrowParty)
        8 + // timestamp
        8 + // start_time
        8 + // end_time
        1 + // status
        1 + // released
        2 + // fee_bps
        32 + // creator
        8 + // nonce
        1 // bump
    }

    /// Get remaining escrow amount
    pub fn get_amount_remaining(&self) -> u64 {
        self.payer.amount_paid
            .saturating_sub(self.payer.amount_refunded)
            .saturating_sub(self.payer.amount_released)
    }

    /// Check if escrow is within valid time window
    pub fn is_active_time(&self) -> bool {
        let now = Clock::get().unwrap().unix_timestamp;
        
        let start_valid = if self.start_time > 0 {
            now >= self.start_time
        } else {
            true
        };

        let end_valid = if self.end_time > 0 {
            now <= self.end_time
        } else {
            true
        };

        start_valid && end_valid
    }
}