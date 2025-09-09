use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeProgramParams {
    pub fee_vault: Pubkey,
    pub default_fee_bps: u16,
}

/// Initialize program configuration
#[derive(Accounts)]
pub struct InitializeProgram<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = ProgramConfig::space(),
        seeds = [ProgramConfig::SEED],
        bump
    )]
    pub program_config: Account<'info, ProgramConfig>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_program(
    ctx: Context<InitializeProgram>,
    params: InitializeProgramParams,
) -> Result<()> {
    let program_config = &mut ctx.accounts.program_config;
    
    program_config.authority = ctx.accounts.authority.key();
    program_config.fee_vault = params.fee_vault;
    program_config.default_fee_bps = params.default_fee_bps;
    program_config.paused = false;
    program_config.bump = ctx.bumps.program_config;
    
    Ok(())
}