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
    
    // Utility Instructions
    pub fn initialize_program(ctx: Context<InitializeProgram>, params: InitializeProgramParams) -> Result<()> {
        instructions::initialize::initialize_program(ctx, params)
    }

    // Asymmetric Escrow Instructions
    pub fn create_asym_escrow(ctx: Context<CreateAsymEscrow>, params: CreateAsymEscrowParams) -> Result<()> {
        instructions::asym_escrow::create_escrow(ctx, params)
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
}