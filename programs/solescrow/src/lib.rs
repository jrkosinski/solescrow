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
    use solana_program_test::*;
    use solana_sdk::{
        signature::{Keypair, Signer},
    };
    use std::rc::Rc;
    
    /// Test helper to find PDA for program config
    pub fn find_program_config_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[state::program_config::ProgramConfig::SEED], &id())
    }

    #[tokio::test]
    async fn test_initialize_program_config() {
        // Create a program test with our program
        let program_test = ProgramTest::new(
            "solana_escrow",
            id(),
            processor!(entry),
        );
        
        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

        // Generate test keypairs
        let admin = Keypair::new();
        let fee_vault = Keypair::new();

        // Find program config PDA
        let (program_config, _bump) = find_program_config_pda();

        // Create instruction data manually (simpler approach)
        let instruction_data = {
            use anchor_lang::InstructionData;
            
            crate::instruction::InitializeProgram {
                params: instructions::initialize::InitializeProgramParams {
                    fee_vault: fee_vault.pubkey(),
                    default_fee_bps: 100,
                }
            }.data()
        };

        let accounts = {
            use anchor_lang::ToAccountMetas;
            
            crate::accounts::InitializeProgram {
                authority: admin.pubkey(),
                program_config,
                system_program: solana_sdk::system_program::id(),
            }.to_account_metas(None)
        };

        let initialize_ix = solana_sdk::instruction::Instruction {
            program_id: id(),
            accounts,
            data: instruction_data,
        };

        // Create and process transaction
        let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
            &[initialize_ix],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &admin], recent_blockhash);

        // This should succeed
        banks_client.process_transaction(transaction).await.unwrap();

        // Verify the account was created
        let program_config_account = banks_client
            .get_account(program_config)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(program_config_account.owner, id());
        assert!(program_config_account.data.len() >= 8); // Has discriminator
    }

    #[tokio::test] 
    async fn test_initialize_program_config_different_settings() {
        // Each test gets a fresh environment - this is the key benefit of Rust tests!
        let program_test = ProgramTest::new(
            "solana_escrow",
            id(),
            processor!(entry),
        );
        
        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

        let admin = Keypair::new();
        let fee_vault = Keypair::new();
        let (program_config, _bump) = find_program_config_pda();

        // Test with different fee (2.5%)
        let instruction_data = {
            use anchor_lang::InstructionData;
            
            crate::instruction::InitializeProgram {
                params: instructions::initialize::InitializeProgramParams {
                    fee_vault: fee_vault.pubkey(),
                    default_fee_bps: 250, // Different fee
                }
            }.data()
        };

        let accounts = {
            use anchor_lang::ToAccountMetas;
            
            crate::accounts::InitializeProgram {
                authority: admin.pubkey(),
                program_config,
                system_program: solana_sdk::system_program::id(),
            }.to_account_metas(None)
        };

        let initialize_ix = solana_sdk::instruction::Instruction {
            program_id: id(),
            accounts,
            data: instruction_data,
        };

        let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
            &[initialize_ix],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &admin], recent_blockhash);

        banks_client.process_transaction(transaction).await.unwrap();

        let program_config_account = banks_client
            .get_account(program_config)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(program_config_account.owner, id());
        
        // Both tests run in complete isolation - no shared state!
        // Test 1 uses 100 bps, Test 2 uses 250 bps - both work perfectly
    }
}