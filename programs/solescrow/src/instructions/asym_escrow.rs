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
    
    //validate inputs
    require!(params.payer != Pubkey::default(), EscrowError::InvalidPayer);
    require!(params.receiver != Pubkey::default(), EscrowError::InvalidReceiver);
    require!(params.payer != params.receiver, EscrowError::InvalidReceiver);
    require!(params.amount > 0, EscrowError::InvalidAmount);
    
    //validate currency
    if params.currency != Pubkey::default() {
        require!(
            ctx.accounts.token_mint.is_some(),
            EscrowError::InvalidToken
        );
    }
    
    //validate dates
    validate_escrow_dates(params.start_time, params.end_time)?;
    
    //initialize escrow
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
    
    //validate payer
    require!(
        ctx.accounts.payer.key() == escrow.payer.addr,
        EscrowError::Unauthorized
    );
    
    //check escrow timing
    require!(escrow.is_active_time(), EscrowError::EscrowNotActive);
    
    //validate amount
    require!(amount > 0, EscrowError::InvalidAmount);
    
    //transfer payment based on currency type
    match escrow.payer.currency_type {
        CurrencyType::Native => {
            //transfer SOL to escrow vault
            transfer_native_sol(
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.escrow_vault.to_account_info(),
                amount,
                ctx.accounts.system_program.to_account_info(),
            )?;
        },
        CurrencyType::SplToken => {
            //transfer SPL tokens to escrow token account
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
    
    //update escrow state
    escrow.status = EscrowStatus::Active;
    escrow.payer.amount_paid = escrow.payer.amount_paid
        .checked_add(amount)
        .ok_or(EscrowError::ArithmeticOverflow)?;
    
    //check if fully paid
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

/// Release escrow (consent-based)
#[derive(Accounts)]
pub struct ReleaseEscrowAsym<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    
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
    
    /// Escrow vault
    #[account(
        mut,
        seeds = [seeds::ESCROW_VAULT, escrow.key().as_ref()],
        bump
    )]
    pub escrow_vault: SystemAccount<'info>,
    
    /// Receiver account for native transfers
    #[account(mut)]
    pub receiver: SystemAccount<'info>,
    
    /// Fee vault
    #[account(mut)]
    pub fee_vault: SystemAccount<'info>,
    
    /// For SPL token transfers
    #[account(mut)]
    pub escrow_token_account: Option<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    pub receiver_token_account: Option<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    pub fee_token_account: Option<Account<'info, TokenAccount>>,
    
    pub token_program: Option<Program<'info, Token>>,
    pub system_program: Program<'info, System>,
}

pub fn release_escrow(ctx: Context<ReleaseEscrowAsym>) -> Result<()> {
    require_not_paused(&ctx.accounts.program_config)?;
    
    let escrow = &mut ctx.accounts.escrow;
    
    //check authorization (payer or receiver)
    let is_payer = ctx.accounts.signer.key() == escrow.payer.addr;
    let is_receiver = ctx.accounts.signer.key() == escrow.receiver.addr;
    require!(is_payer || is_receiver, EscrowError::Unauthorized);
    
    //check escrow timing
    require!(escrow.is_active_time(), EscrowError::EscrowNotActive);
    
    let remaining_amount = escrow.get_amount_remaining();
    require!(remaining_amount > 0, EscrowError::InvalidEscrowState);
    
    //record consent
    if is_payer && !escrow.payer.released {
        escrow.payer.released = true;
        emit!(ReleaseAssentGivenEvent {
            escrow_id: escrow.id,
            assenting_address: ctx.accounts.signer.key(),
            assent_type: ReleaseAssentType::Payer,
        });
    }
    
    if is_receiver && !escrow.receiver.released {
        escrow.receiver.released = true;
        emit!(ReleaseAssentGivenEvent {
            escrow_id: escrow.id,
            assenting_address: ctx.accounts.signer.key(),
            assent_type: ReleaseAssentType::Receiver,
        });
    }
    
    //execute release if both parties consent
    if escrow.payer.released && escrow.receiver.released {
        execute_release(ctx, remaining_amount)?;
    }
    
    Ok(())
}

/// Release escrow (consent-based)
#[derive(Accounts)]
pub struct RefundEscrowAsym<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    
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
    
    /// Escrow vault
    #[account(
        mut,
        seeds = [seeds::ESCROW_VAULT, escrow.key().as_ref()],
        bump
    )]
    pub escrow_vault: SystemAccount<'info>,
    
    /// Payer account for refunds
    #[account(mut)]
    pub payer: SystemAccount<'info>,
    
    /// For SPL token refunds
    #[account(mut)]
    pub escrow_token_account: Option<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    pub payer_token_account: Option<Account<'info, TokenAccount>>,
    
    pub token_program: Option<Program<'info, Token>>,
    pub system_program: Program<'info, System>,
}

pub fn refund_escrow(ctx: Context<RefundEscrowAsym>, amount: u64) -> Result<()> {
    require_not_paused(&ctx.accounts.program_config)?;
    
    let escrow = &mut ctx.accounts.escrow;
    
    //check authorization (receiver)
    require!(ctx.accounts.signer.key() == escrow.receiver.addr, EscrowError::Unauthorized);
    
    //check escrow timing
    require!(escrow.is_active_time(), EscrowError::EscrowNotActive);

    //validate refund amount
    let remaining_amount = escrow.get_amount_remaining();
    require!(remaining_amount >= amount, EscrowError::AmountExceeded);
    require!(amount > 0, EscrowError::InvalidAmount);
    require!(!escrow.released, EscrowError::AlreadyReleased);

    //execute refund
    execute_refund(ctx, amount);

    Ok(())
}

//helper function to execute release
fn execute_release(ctx: Context<ReleaseEscrowAsym>, amount: u64) -> Result<()> {
    let escrow = &mut ctx.accounts.escrow;
    
    //calculate fee and amount to transfer
    let (fee, amount_to_transfer) = calculate_fee_and_amount(amount, escrow.fee_bps)?;
    
    //generate signer seeds for escrow vault
    let escrow_key = escrow.key();
    let vault_seeds = &[
        seeds::ESCROW_VAULT,
        escrow_key.as_ref(),
        &[ctx.bumps.escrow_vault],
    ];
    let vault_signer = &[&vault_seeds[..]];
    
    //transfer funds based on currency type
    match escrow.payer.currency_type {
        CurrencyType::Native => {
            //transfer to receiver
            if amount_to_transfer > 0 {
                **ctx.accounts.escrow_vault.to_account_info().try_borrow_mut_lamports()? -= amount_to_transfer;
                **ctx.accounts.receiver.to_account_info().try_borrow_mut_lamports()? += amount_to_transfer;
            }
            
            //transfer fee
            if fee > 0 {
                **ctx.accounts.escrow_vault.to_account_info().try_borrow_mut_lamports()? -= fee;
                **ctx.accounts.fee_vault.to_account_info().try_borrow_mut_lamports()? += fee;
            }
        },
        CurrencyType::SplToken => {
            let escrow_token_account = ctx.accounts.escrow_token_account
                .as_ref()
                .ok_or(EscrowError::InvalidToken)?;
            let receiver_token_account = ctx.accounts.receiver_token_account
                .as_ref()
                .ok_or(EscrowError::InvalidToken)?;
            let token_program = ctx.accounts.token_program
                .as_ref()
                .ok_or(EscrowError::InvalidToken)?;
            
            //transfer to receiver
            if amount_to_transfer > 0 {
                let cpi_accounts = anchor_spl::token::Transfer {
                    from: escrow_token_account.to_account_info(),
                    to: receiver_token_account.to_account_info(),
                    authority: ctx.accounts.escrow_vault.to_account_info(),
                };
                let cpi_program = token_program.to_account_info();
                let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, vault_signer);
                anchor_spl::token::transfer(cpi_ctx, amount_to_transfer)?;
            }
            
            //transfer fee
            if fee > 0 {
                let fee_token_account = ctx.accounts.fee_token_account
                    .as_ref()
                    .ok_or(EscrowError::InvalidToken)?;
                
                let cpi_accounts = anchor_spl::token::Transfer {
                    from: escrow_token_account.to_account_info(),
                    to: fee_token_account.to_account_info(),
                    authority: ctx.accounts.escrow_vault.to_account_info(),
                };
                let cpi_program = token_program.to_account_info();
                let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, vault_signer);
                anchor_spl::token::transfer(cpi_ctx, fee)?;
            }
        },
    }
    
    //update escrow state
    escrow.released = true;
    escrow.payer.amount_released = escrow.payer.amount_released
        .checked_add(amount_to_transfer)
        .ok_or(EscrowError::ArithmeticOverflow)?;
    
    if escrow.get_amount_remaining() == 0 {
        escrow.status = EscrowStatus::Completed;
    }
    
    //emit event
    emit!(EscrowReleasedEvent {
        escrow_id: escrow.id,
        amount: amount_to_transfer,
        fee,
    });
    
    Ok(())
}

fn execute_refund(ctx: Context<RefundEscrowAsym>, amount: u64) -> Result<()> {
    let escrow = &mut ctx.accounts.escrow;

    // Generate signer seeds for escrow vault
    let escrow_key = escrow.key();
    let vault_seeds = &[
        seeds::ESCROW_VAULT,
        escrow_key.as_ref(),
        &[ctx.bumps.escrow_vault],
    ];
    let vault_signer = &[&vault_seeds[..]];

    //transfer funds based on currency type
    match escrow.payer.currency_type {
        CurrencyType::Native => {
            //transfer to payer
            if amount > 0 {
                **ctx.accounts.escrow_vault.to_account_info().try_borrow_mut_lamports()? -= amount;
                **ctx.accounts.payer.to_account_info().try_borrow_mut_lamports()? += amount;
            }
        },

        CurrencyType::SplToken => {
            let escrow_token_account = ctx.accounts.escrow_token_account
                .as_ref()
                .ok_or(EscrowError::InvalidToken)?;
            let payer_token_account = ctx.accounts.payer_token_account
                .as_ref()
                .ok_or(EscrowError::InvalidToken)?;
            let token_program = ctx.accounts.token_program
                .as_ref()
                .ok_or(EscrowError::InvalidToken)?;
            
            //transfer to payer
            if amount > 0 {
                let cpi_accounts = anchor_spl::token::Transfer {
                    from: escrow_token_account.to_account_info(),
                    to: payer_token_account.to_account_info(),
                    authority: ctx.accounts.escrow_vault.to_account_info(),
                };
                let cpi_program = token_program.to_account_info();
                let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, vault_signer);
                anchor_spl::token::transfer(cpi_ctx, amount)?;
            }
        },
    }
    
    //update escrow state
    escrow.payer.amount_refunded = escrow.payer.amount_refunded
        .checked_add(amount)
        .ok_or(EscrowError::ArithmeticOverflow)?;
    
    if escrow.get_amount_remaining() == 0 {
        escrow.status = EscrowStatus::Completed;
    }
    
    //emit event
    emit!(EscrowRefundedEvent {
        escrow_id: escrow.id,
        amount,
    });
    
    Ok(())
}

//helper function to generate escrow ID
fn generate_escrow_id(creator: &Pubkey, nonce: u64) -> [u8; 32] {
    let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
    hasher.hash(creator.as_ref());
    hasher.hash(&nonce.to_le_bytes());
    hasher.result().to_bytes()
}

//events
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

#[event]
pub struct ReleaseAssentGivenEvent {
    pub escrow_id: [u8; 32],
    pub assenting_address: Pubkey,
    pub assent_type: ReleaseAssentType,
}

#[event]
pub struct EscrowReleasedEvent {
    pub escrow_id: [u8; 32],
    pub amount: u64,
    pub fee: u64,
}

#[event]
pub struct EscrowRefundedEvent {
    pub escrow_id: [u8; 32],
    pub amount: u64,
}
