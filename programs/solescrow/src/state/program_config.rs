use anchor_lang::prelude::*;

/// Program configuration account
#[account]
#[derive(Debug)]
pub struct ProgramConfig {
    /// Program authority
    pub authority: Pubkey,
    /// Fee vault address where fees are collected
    pub fee_vault: Pubkey,
    /// Default fee in basis points
    pub default_fee_bps: u16,
    /// Whether the program is paused
    pub paused: bool,
    /// Bump seed for PDA
    pub bump: u8,
}

impl ProgramConfig {
    /// Calculate space needed for account
    pub const fn space() -> usize {
        8 + // discriminator
        32 + // authority
        32 + // fee_vault
        2 + // default_fee_bps
        1 + // paused
        1 // bump
    }

    /// Program config PDA seed
    pub const SEED: &'static [u8] = b"program_config";
}