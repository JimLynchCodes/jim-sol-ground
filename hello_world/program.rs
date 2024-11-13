// Import anchor
use anchor_lang::prelude::*;

declare_id!("");

#[program]
mod hello_world {
    use super::*;

    #[derive(Accounts)]
    pub struct Hello {}

    pub fn hello(ctx: Context<Hello>) -> Result<()> {
        msg!("Hello, World!");
        Ok(())
    }

}
