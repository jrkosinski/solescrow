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