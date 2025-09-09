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
}