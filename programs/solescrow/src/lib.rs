use anchor_lang::prelude::*;

declare_id!("3zG5YyBSndYJBKPvMo8vnBLSqF5j56QRGcYv2JBN8haJ");

#[program]
pub mod solescrow {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
