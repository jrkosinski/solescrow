use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::errors::*;
use crate::constants::*;

/// Check if contract is not paused
pub fn require_not_paused(program_config: &ProgramConfig) -> Result<()> {
    require!(!program_config.paused, EscrowError::ProgramPaused);
    Ok(())
}

/// Validate escrow timing
pub fn validate_escrow_dates(start_time: i64, end_time: i64) -> Result<()> {
    if end_time > 0 {
        let now = Clock::get()?.unix_timestamp;
        require!(
            end_time > now + MIN_END_TIME_BUFFER && end_time > start_time,
            EscrowError::InvalidEndDate
        );
    }
    Ok(())
}