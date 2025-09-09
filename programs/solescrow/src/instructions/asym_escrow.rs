use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::state::*;
use crate::errors::*;
use crate::constants::*;
use crate::instructions::utils::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CreateAsymEscrowParams {
    pub payer: Pubkey,
    pub receiver: Pubkey,
    pub currency: Pubkey, // Pubkey::default() for native SOL
    pub amount: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub nonce: u64,
}

/// Create asymmetric escrow
#[derive(Accounts)]
#[instruction(params: CreateAsymEscrowParams)]
pub struct CreateAsymEscrow<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    #[account(
        init,
        payer = creator,
        space = 1, //AsymEscrow::space(params.arbiters.len()),
        seeds = [seeds::ASYM_ESCROW, creator.key().as_ref(), &params.nonce.to_le_bytes()],
        bump
    )]
    pub escrow: Account<'info, AsymEscrow>,
    
    #[account(
        seeds = [ProgramConfig::SEED],
        bump = program_config.bump
    )]
    pub program_config: Account<'info, ProgramConfig>,
    
    /// Token mint (only required for SPL token escrows)
    pub token_mint: Option<Account<'info, Mint>>,
    
    pub system_program: Program<'info, System>,
}

pub fn create_escrow(
    ctx: Context<CreateAsymEscrow>,
    params: CreateAsymEscrowParams,
) -> Result<()> {
    require_not_paused(&ctx.accounts.program_config)?;
    
    // Validate inputs
    require!(params.payer != Pubkey::default(), EscrowError::InvalidPayer);
    require!(params.receiver != Pubkey::default(), EscrowError::InvalidReceiver);
    require!(params.payer != params.receiver, EscrowError::InvalidReceiver);
    require!(params.amount > 0, EscrowError::InvalidAmount);
    
    // Validate currency
    if params.currency != Pubkey::default() {
        require!(
            ctx.accounts.token_mint.is_some(),
            EscrowError::InvalidToken
        );
    }
    
    // Validate dates
    validate_escrow_dates(params.start_time, params.end_time)?;
    
    // Initialize escrow
    let escrow = &mut ctx.accounts.escrow;
    let escrow_id = generate_escrow_id(&ctx.accounts.creator.key(), params.nonce);
    
    escrow.id = escrow_id;
    escrow.payer = EscrowParty {
        addr: params.payer,
        currency: params.currency,
        currency_type: if params.currency == Pubkey::default() {
            CurrencyType::Native
        } else {
            CurrencyType::SplToken
        },
        amount: params.amount,
        ..Default::default()
    };
    escrow.receiver = EscrowParty {
        addr: params.receiver,
        ..Default::default()
    };
    escrow.timestamp = Clock::get()?.unix_timestamp;
    escrow.start_time = params.start_time;
    escrow.end_time = params.end_time;
    escrow.status = EscrowStatus::Pending;
    escrow.released = false;
    escrow.fee_bps = ctx.accounts.program_config.default_fee_bps;
    escrow.creator = ctx.accounts.creator.key();
    escrow.nonce = params.nonce;
    escrow.bump = ctx.bumps.escrow;
    
    emit!(EscrowCreatedEvent {
        escrow_id,
        creator: ctx.accounts.creator.key(),
        payer: params.payer,
        receiver: params.receiver,
        amount: params.amount,
    });
    
    Ok(())
}


// Helper function to generate escrow ID
fn generate_escrow_id(creator: &Pubkey, nonce: u64) -> [u8; 32] {
    let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
    hasher.hash(creator.as_ref());
    hasher.hash(&nonce.to_le_bytes());
    hasher.result().to_bytes()
}

// Events
#[event]
pub struct EscrowCreatedEvent {
    pub escrow_id: [u8; 32],
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub receiver: Pubkey,
    pub amount: u64,
}
