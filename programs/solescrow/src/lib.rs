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
}