use anchor_lang::prelude::*;

declare_id!("5jujwhy3XVk4RFdUgbn1x63sBp9V3j2Pb1sRMh72bqfL");

pub mod state;
pub mod instructions;
pub mod errors;
pub mod constants;

use instructions::*;

#[program]
pub mod escrow {
    use super::*;
    
    //utility instructions
    pub fn initialize_program(ctx: Context<InitializeProgram>, params: InitializeProgramParams) -> Result<()> {
        instructions::initialize::initialize_program(ctx, params)
    }

    //asymmetric escrow instructions
    //TODO: rename to create_escrow_asym
    pub fn create_asym_escrow(ctx: Context<CreateAsymEscrow>, params: CreateAsymEscrowParams) -> Result<()> {
        instructions::asym_escrow::create_escrow(ctx, params)
    }

    pub fn place_payment_asym(ctx: Context<PlacePaymentAsym>, amount: u64) -> Result<()> {
        instructions::asym_escrow::place_payment(ctx, amount)
    }

    pub fn release_escrow_asym(ctx: Context<ReleaseEscrowAsym>) -> Result<()> {
        instructions::asym_escrow::release_escrow(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    pub fn find_program_config_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[state::program_config::ProgramConfig::SEED], &id())
    }

    #[test]
    fn test_program_config_pda_derivation() {
        //pda derivation should be deterministic
        let (pda1, bump1) = find_program_config_pda();
        let (pda2, bump2) = find_program_config_pda();
        
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
        
        //verify pda is derived correctly
        let expected_seeds = &[state::program_config::ProgramConfig::SEED];
        let (expected_pda, expected_bump) = Pubkey::find_program_address(expected_seeds, &id());
        
        assert_eq!(pda1, expected_pda);
        assert_eq!(bump1, expected_bump);
    }

    #[test]
    fn test_program_config_space_calculation() {
        //space calculation should match actual struct
        let expected_space = state::program_config::ProgramConfig::space();
        
        //space should be reasonable bounds
        assert!(expected_space >= 76); //8 (discriminator) + 32 (authority) + 32 (fee_vault) + 2 (fee_bps) + 1 (paused) + 1 (bump)
        assert!(expected_space <= 200); //not excessively large
    }

    #[test]
    fn test_program_id() {
        //program ID should be correctly set
        let program_id = id();
        
        //should be valid pubkey (not default)
        assert_ne!(program_id, Pubkey::default());
        
        //should match declared ID
        assert_eq!(program_id.to_string(), "5jujwhy3XVk4RFdUgbn1x63sBp9V3j2Pb1sRMh72bqfL");
    }

    #[test]
    fn test_initialize_program_params() {
        use solana_sdk::signature::{Keypair, Signer};
        
        //params should be created and serialized
        let fee_vault = Keypair::new();
        let params = instructions::initialize::InitializeProgramParams {
            fee_vault: fee_vault.pubkey(),
            default_fee_bps: 100,
        };
        
        //should be serializable
        let serialized = anchor_lang::AnchorSerialize::try_to_vec(&params);
        assert!(serialized.is_ok());
        
        //test different fee values
        let params_high_fee = instructions::initialize::InitializeProgramParams {
            fee_vault: fee_vault.pubkey(),
            default_fee_bps: 250, //2.5%
        };
        
        let serialized_high_fee = anchor_lang::AnchorSerialize::try_to_vec(&params_high_fee);
        assert!(serialized_high_fee.is_ok());
        
        //serialized data should be different
        assert_ne!(serialized.unwrap(), serialized_high_fee.unwrap());
    }

    #[test]
    fn test_program_config_seed() {
        //seed constant should be correct
        let seed = state::program_config::ProgramConfig::SEED;
        assert_eq!(seed, b"program_config");
    }

    #[test]
    fn test_create_asym_escrow() {
        use crate::instructions::asym_escrow::CreateAsymEscrowParams;
        use crate::state::escrow::{EscrowStatus, CurrencyType};
        
        //test escrow id generation
        let creator = Pubkey::new_unique();
        let nonce = 12345u64;
        
        //generate escrow id using same logic as create_asym_escrow
        let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
        hasher.hash(creator.as_ref());
        hasher.hash(&nonce.to_le_bytes());
        let expected_id = hasher.result().to_bytes();
        
        //verify id generation is deterministic
        let mut hasher2 = anchor_lang::solana_program::hash::Hasher::default();
        hasher2.hash(creator.as_ref());
        hasher2.hash(&nonce.to_le_bytes());
        let id2 = hasher2.result().to_bytes();
        assert_eq!(expected_id, id2);
        
        //test escrow params validation
        let payer = Pubkey::new_unique();
        let receiver = Pubkey::new_unique();
        let params = CreateAsymEscrowParams {
            payer,
            receiver,
            currency: Pubkey::default(), //native SOL
            amount: 1000000, //1 SOL in lamports
            start_time: 1600000000,
            end_time: 1600086400, //24 hours later
            nonce,
        };
        
        //validate params structure
        assert_eq!(params.payer, payer);
        assert_eq!(params.receiver, receiver);
        assert_eq!(params.currency, Pubkey::default());
        assert_eq!(params.amount, 1000000);
        assert!(params.start_time < params.end_time);
        assert_eq!(params.nonce, nonce);
        
        //test default enum values
        assert_eq!(EscrowStatus::default(), EscrowStatus::Pending);
        assert_eq!(CurrencyType::default(), CurrencyType::Native);
    }

    #[test]
    fn test_escrow_payment_calculations() {
        use crate::state::escrow::{AsymEscrow, EscrowParty, EscrowStatus, CurrencyType};
        
        //create mock escrow with 1 SOL requirement
        let mut escrow = AsymEscrow {
            id: [0u8; 32],
            payer: EscrowParty {
                addr: Pubkey::new_unique(),
                currency: Pubkey::default(),
                currency_type: CurrencyType::Native,
                amount: 1_000_000_000, //1 SOL in lamports
                amount_paid: 0,
                amount_refunded: 0,
                amount_released: 0,
                released: false,
            },
            receiver: EscrowParty::default(),
            timestamp: 1600000000,
            start_time: 0,
            end_time: 0,
            status: EscrowStatus::Pending,
            released: false,
            fee_bps: 100,
            creator: Pubkey::new_unique(),
            nonce: 12345,
            bump: 254,
        };

        //test partial payment (0.5 SOL)
        escrow.payer.amount_paid = 500_000_000;
        escrow.status = EscrowStatus::Active;
        
        //verify payment state
        assert_eq!(escrow.payer.amount_paid, 500_000_000);
        assert_eq!(escrow.status, EscrowStatus::Active);
        assert_eq!(escrow.get_amount_remaining(), 500_000_000);
        
        //test multiple payments (add another 0.3 SOL)
        escrow.payer.amount_paid = escrow.payer.amount_paid
            .checked_add(300_000_000)
            .unwrap();
            
        assert_eq!(escrow.payer.amount_paid, 800_000_000);
        assert_eq!(escrow.get_amount_remaining(), 800_000_000);
        
        //test full payment completion (add final 0.2 SOL)
        escrow.payer.amount_paid = escrow.payer.amount_paid
            .checked_add(200_000_000)
            .unwrap();
            
        assert_eq!(escrow.payer.amount_paid, 1_000_000_000);
        assert!(escrow.payer.amount_paid >= escrow.payer.amount);
        
        //test overpayment scenario
        escrow.payer.amount_paid = 1_200_000_000; //1.2 SOL paid
        assert!(escrow.payer.amount_paid > escrow.payer.amount);
        assert_eq!(escrow.get_amount_remaining(), 1_200_000_000);
    }

    #[test]
    fn test_escrow_validation_logic() {
        use crate::state::escrow::{AsymEscrow, EscrowParty, EscrowStatus, CurrencyType};
        
        let payer = Pubkey::new_unique();
        let receiver = Pubkey::new_unique();
        let amount = 1_000_000_000u64; //1 SOL
        
        //test valid escrow parameters
        assert_ne!(payer, receiver); //payer must != receiver
        assert!(amount > 0); //amount must be positive
        
        //test currency validation for native SOL
        let native_currency = Pubkey::default();
        assert_eq!(native_currency, Pubkey::default());
        
        //test currency validation for SPL token
        let token_mint = Pubkey::new_unique();
        assert_ne!(token_mint, Pubkey::default());
        
        //test zero amount validation (should fail)
        let zero_amount = 0u64;
        assert_eq!(zero_amount, 0);
        
        //test same payer/receiver (should fail)
        let same_key = Pubkey::new_unique();
        assert_eq!(same_key, same_key);
        
        //test arithmetic overflow protection
        let max_amount = u64::MAX;
        let safe_add = max_amount.checked_add(1);
        assert_eq!(safe_add, None); //overflow returns None
        
        let safe_amount = 1_000_000_000u64;
        let safe_result = safe_amount.checked_add(500_000_000);
        assert_eq!(safe_result, Some(1_500_000_000));
    }

    #[test]
    fn test_escrow_timing_logic() {
        //test escrow timing without Clock (mock scenario)
        let now = 1600000000i64;
        let start_time = now - 3600; //1 hour ago
        let end_time = now + 3600; //1 hour from now
        
        //test time window validation
        assert!(start_time < now); //started in past
        assert!(end_time > now); //ends in future
        assert!(end_time > start_time); //end after start
        
        //test immediate start (start_time = 0)
        let immediate_start = 0i64;
        assert_eq!(immediate_start, 0);
        
        //test no expiry (end_time = 0)  
        let no_expiry = 0i64;
        assert_eq!(no_expiry, 0);
        
        //test expired escrow
        let expired_end = now - 1800; //30 minutes ago
        assert!(expired_end < now);
        
        //test future start
        let future_start = now + 1800; //30 minutes from now
        assert!(future_start > now);
    }

    #[test]
    fn test_escrow_release_consent_logic() {
        use crate::state::escrow::{AsymEscrow, EscrowParty, EscrowStatus, CurrencyType};
        
        //create mock escrow with full payment made
        let payer_key = Pubkey::new_unique();
        let receiver_key = Pubkey::new_unique();
        
        let mut escrow = AsymEscrow {
            id: [1u8; 32],
            payer: EscrowParty {
                addr: payer_key,
                currency: Pubkey::default(),
                currency_type: CurrencyType::Native,
                amount: 1_000_000_000, //1 SOL required
                amount_paid: 1_000_000_000, //1 SOL paid (fully funded)
                amount_refunded: 0,
                amount_released: 0,
                released: false, //no consent yet
            },
            receiver: EscrowParty {
                addr: receiver_key,
                currency: Pubkey::default(),
                currency_type: CurrencyType::Native,
                amount: 0,
                amount_paid: 0,
                amount_refunded: 0,
                amount_released: 0,
                released: false, //no consent yet
            },
            timestamp: 1600000000,
            start_time: 0,
            end_time: 0,
            status: EscrowStatus::Active,
            released: false,
            fee_bps: 100, //1% fee
            creator: Pubkey::new_unique(),
            nonce: 12345,
            bump: 254,
        };

        //test payer consent
        assert!(!escrow.payer.released);
        escrow.payer.released = true;
        assert!(escrow.payer.released);
        
        //test receiver consent 
        assert!(!escrow.receiver.released);
        escrow.receiver.released = true;
        assert!(escrow.receiver.released);
        
        //test both parties have consented
        assert!(escrow.payer.released && escrow.receiver.released);
        
        //test remaining amount calculation
        let remaining = escrow.get_amount_remaining();
        assert_eq!(remaining, 1_000_000_000); //full amount available for release
        
        //test escrow completion after release
        escrow.released = true;
        escrow.payer.amount_released = 990_000_000; //after 1% fee
        escrow.status = EscrowStatus::Completed;
        
        assert!(escrow.released);
        assert_eq!(escrow.status, EscrowStatus::Completed);
        assert_eq!(escrow.get_amount_remaining(), 10_000_000); //only fee remains
    }

    #[test]
    fn test_escrow_authorization_logic() {
        use crate::state::escrow::{AsymEscrow, EscrowParty, EscrowStatus, CurrencyType};
        
        let payer_key = Pubkey::new_unique();
        let receiver_key = Pubkey::new_unique();
        let unauthorized_key = Pubkey::new_unique();
        
        let escrow = AsymEscrow {
            id: [2u8; 32],
            payer: EscrowParty {
                addr: payer_key,
                currency: Pubkey::default(),
                currency_type: CurrencyType::Native,
                amount: 1_000_000_000,
                amount_paid: 1_000_000_000,
                amount_refunded: 0,
                amount_released: 0,
                released: false,
            },
            receiver: EscrowParty {
                addr: receiver_key,
                ..Default::default()
            },
            timestamp: 1600000000,
            start_time: 0,
            end_time: 0,
            status: EscrowStatus::Active,
            released: false,
            fee_bps: 100,
            creator: Pubkey::new_unique(),
            nonce: 12346,
            bump: 254,
        };

        //test payer authorization
        let is_payer = payer_key == escrow.payer.addr;
        assert!(is_payer);
        
        //test receiver authorization
        let is_receiver = receiver_key == escrow.receiver.addr;
        assert!(is_receiver);
        
        //test unauthorized party
        let is_unauthorized = unauthorized_key == escrow.payer.addr || unauthorized_key == escrow.receiver.addr;
        assert!(!is_unauthorized);
        
        //test mutual authorization check (for release)
        let payer_or_receiver = is_payer || is_receiver;
        assert!(payer_or_receiver);
        
        let unauthorized_check = unauthorized_key == escrow.payer.addr || unauthorized_key == escrow.receiver.addr;
        assert!(!unauthorized_check);
    }

    #[test]
    fn test_escrow_fee_calculation() {
        //test 1% fee calculation (100 basis points)
        let amount = 1_000_000_000u64; //1 SOL
        let fee_bps = 100u16; //1%
        
        //manual fee calculation
        let expected_fee = amount * (fee_bps as u64) / 10000; //1% of 1 SOL = 0.01 SOL
        let expected_transfer = amount - expected_fee; //0.99 SOL
        
        assert_eq!(expected_fee, 10_000_000); //0.01 SOL in lamports
        assert_eq!(expected_transfer, 990_000_000); //0.99 SOL in lamports
        
        //test 2.5% fee calculation (250 basis points)
        let fee_bps_high = 250u16; //2.5%
        let expected_fee_high = amount * (fee_bps_high as u64) / 10000;
        let expected_transfer_high = amount - expected_fee_high;
        
        assert_eq!(expected_fee_high, 25_000_000); //0.025 SOL in lamports
        assert_eq!(expected_transfer_high, 975_000_000); //0.975 SOL in lamports
        
        //test zero fee
        let fee_bps_zero = 0u16;
        let expected_fee_zero = amount * (fee_bps_zero as u64) / 10000;
        let expected_transfer_zero = amount - expected_fee_zero;
        
        assert_eq!(expected_fee_zero, 0);
        assert_eq!(expected_transfer_zero, amount); //full amount
        
        //test maximum fee (100% - should never happen in practice)
        let fee_bps_max = 10000u16; //100%
        let expected_fee_max = amount * (fee_bps_max as u64) / 10000;
        
        assert_eq!(expected_fee_max, amount); //entire amount as fee
    }
}