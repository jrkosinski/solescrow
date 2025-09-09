use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Invalid escrow")]
    InvalidEscrow,
    
    #[msg("Invalid payer address")]
    InvalidPayer,
    
    #[msg("Invalid receiver address")]
    InvalidReceiver,
    
    #[msg("Invalid party address")]
    InvalidPartyAddress,
    
    #[msg("Invalid amount")]
    InvalidAmount,

    #[msg("Program is paused")]
    ProgramPaused,
    
    #[msg("Invalid end date")]
    InvalidEndDate,
    
    #[msg("Invalid token")]
    InvalidToken,
    
    #[msg("Insufficient funds")]
    InsufficientFunds,
    
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    
    #[msg("Invalid escrow state")]
    InvalidEscrowState,
    
    #[msg("Escrow not active")]
    EscrowNotActive,
    
    #[msg("Invalid currency")]
    InvalidCurrency,
    
    #[msg("Unauthorized")]
    Unauthorized,
}