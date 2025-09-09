use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::errors::*;
use crate::constants::*;

/// Transfer native SOL
pub fn transfer_native_sol<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    amount: u64,
    system_program: AccountInfo<'info>,
) -> Result<()> {
    let ix = anchor_lang::solana_program::system_instruction::transfer(
        from.key,
        to.key,
        amount,
    );
    
    anchor_lang::solana_program::program::invoke(
        &ix,
        &[from, to, system_program],
    )?;
    
    Ok(())
}

/// Transfer SPL tokens
pub fn transfer_spl_tokens<'info>(
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    amount: u64,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };
    
    let cpi_program = token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    
    token::transfer(cpi_ctx, amount)?;
    
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

/// Check if escrow is not paused
pub fn require_not_paused(program_config: &ProgramConfig) -> Result<()> {
    require!(!program_config.paused, EscrowError::ProgramPaused);
    Ok(())
}

/// Calculate fee and remaining amount
pub fn calculate_fee_and_amount(amount: u64, fee_bps: u16) -> Result<(u64, u64)> {
    if fee_bps == 0 {
        return Ok((0, amount));
    }
    
    let fee = amount
        .checked_mul(fee_bps as u64)
        .ok_or(EscrowError::ArithmeticOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(EscrowError::ArithmeticOverflow)?;
    
    if fee > amount {
        return Ok((0, amount));
    }
    
    let amount_to_pay = amount
        .checked_sub(fee)
        .ok_or(EscrowError::ArithmeticOverflow)?;
    
    Ok((fee, amount_to_pay))
}