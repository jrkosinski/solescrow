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
        space = AsymEscrow::space(),
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

/// Place payment in asymmetric escrow
#[derive(Accounts)]
pub struct PlacePaymentAsym<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(
        mut,
        constraint = escrow.status != EscrowStatus::Completed @ EscrowError::InvalidEscrowState,
        constraint = escrow.status != EscrowStatus::Arbitration @ EscrowError::InvalidEscrowState,
    )]
    pub escrow: Account<'info, AsymEscrow>,
    
    #[account(
        seeds = [ProgramConfig::SEED],
        bump = program_config.bump
    )]
    pub program_config: Account<'info, ProgramConfig>,
    
    /// Escrow vault to hold funds
    #[account(
        mut,
        seeds = [seeds::ESCROW_VAULT, escrow.key().as_ref()],
        bump
    )]
    pub escrow_vault: SystemAccount<'info>,
    
    /// For SPL token payments
    #[account(mut)]
    pub payer_token_account: Option<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    pub escrow_token_account: Option<Account<'info, TokenAccount>>,
    
    pub token_program: Option<Program<'info, Token>>,
    pub system_program: Program<'info, System>,
}

pub fn place_payment(
    ctx: Context<PlacePaymentAsym>,
    amount: u64,
) -> Result<()> {
    require_not_paused(&ctx.accounts.program_config)?;
    
    let escrow = &mut ctx.accounts.escrow;
    
    // Validate payer
    require!(
        ctx.accounts.payer.key() == escrow.payer.addr,
        EscrowError::Unauthorized
    );
    
    // Check escrow timing
    require!(escrow.is_active_time(), EscrowError::EscrowNotActive);
    
    // Validate amount
    require!(amount > 0, EscrowError::InvalidAmount);
    
    // Transfer payment based on currency type
    match escrow.payer.currency_type {
        CurrencyType::Native => {
            // Transfer SOL to escrow vault
            transfer_native_sol(
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.escrow_vault.to_account_info(),
                amount,
                ctx.accounts.system_program.to_account_info(),
            )?;
        },
        CurrencyType::SplToken => {
            // Transfer SPL tokens to escrow token account
            let payer_token_account = ctx.accounts.payer_token_account
                .as_ref()
                .ok_or(EscrowError::InvalidToken)?;
            let escrow_token_account = ctx.accounts.escrow_token_account
                .as_ref()
                .ok_or(EscrowError::InvalidToken)?;
            let token_program = ctx.accounts.token_program
                .as_ref()
                .ok_or(EscrowError::InvalidToken)?;
            
            transfer_spl_tokens(
                payer_token_account,
                escrow_token_account,
                &ctx.accounts.payer,
                amount,
                token_program,
            )?;
        },
    }
    
    // Update escrow state
    escrow.status = EscrowStatus::Active;
    escrow.payer.amount_paid = escrow.payer.amount_paid
        .checked_add(amount)
        .ok_or(EscrowError::ArithmeticOverflow)?;
    
    // Check if fully paid
    let is_fully_paid = escrow.payer.amount_paid >= escrow.payer.amount;
    
    emit!(PaymentReceivedEvent {
        escrow_id: escrow.id,
        payer: ctx.accounts.payer.key(),
        amount,
        total_paid: escrow.payer.amount_paid,
        fully_paid: is_fully_paid,
    });
    
    if is_fully_paid {
        emit!(EscrowFullyPaidEvent {
            escrow_id: escrow.id,
            total_amount: escrow.payer.amount_paid,
        });
    }
    
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

#[event]
pub struct PaymentReceivedEvent {
    pub escrow_id: [u8; 32],
    pub payer: Pubkey,
    pub amount: u64,
    pub total_paid: u64,
    pub fully_paid: bool,
}

#[event]
pub struct EscrowFullyPaidEvent {
    pub escrow_id: [u8; 32],
    pub total_amount: u64,
}
