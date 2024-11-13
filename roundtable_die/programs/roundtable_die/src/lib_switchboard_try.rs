use anchor_lang::prelude::*;

declare_id!("GFjQ5uhLf4K9p3JgF4nGHp7mQswqzU6p9y9BgBykhUMX");

// #[program]
// pub mod roundtable_die {
//     use super::*;

//     pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
//         msg!("Greetings from: {:?}", ctx.program_id);
//         Ok(())
//     }
// }

// #[derive(Accounts)]
// pub struct Initialize {}

use anchor_lang::prelude::*;
use switchboard_program::state::OracleQueueAccount;

#[program]
pub mod number_guessing_game {
  
    #[account]
    pub struct ChooseNumber {
        // pub bet_amount: u64,
        pub players: Vec<Player>,
        pub vrf_key: [u8; 32],
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct Player {
        pub address: Pubkey,
        pub number: u8,
    }

    #[error]
    pub enum CustomError {
        #[msg("Invalid number")]
        InvalidNumber,
        #[msg("Number already chosen")]
        NumberAlreadyChosen,
        #[msg("Invalid bet amount")]
        InvalidBetAmount,
        #[msg("Insufficient funds")]
        InsufficientFunds,
    }

    #[account]
    pub struct SettleGame<'info> {
        #[account(mut)]
        pub game_account: Account<'info, ChooseNumber>,
        #[account(mut)]
        pub oracle_queue_account: Account<'info, OracleQueueAccount>,
    }

    pub fn choose_number(ctx: Context<ChooseNumber>, number: u8) -> ProgramResult {
        
        const BET_AMOUNT: u64 = 10; // Adjust the bet amount as needed
        
        // Program account that stores the game state
        let game_account = &mut ctx.accounts.game_account;
        let player_account = &ctx.accounts.player;
        // game_account.bet_amount = BET_AMOUNT;

        // Check if the number is valid and not already chosen
        require!(number > 0 && number <= 6, CustomError::InvalidNumber);
        require!(game_account.players.iter().all(|p| p.number != number), CustomError::NumberAlreadyChosen);

        // Check if the player has sent the correct amount of SOL
        // let required_lamports = BET_AMOUNT * LAMPORTS_PER_SOL;
        let required_lamports = BET_AMOUNT;
        require!(player_account.lamports() >= required_lamports, CustomError::InsufficientFunds);


        // Transfers bet from player to game
        token::transfer(
            ctx.accounts.into_context(token::Transfer {
                from: player_account.to_account_info(),
                to: game_account.to_account_info(),
                authority: player_account.to_account_info(),
            })?,
            required_lamports,
        )?;

        // Add the player to the game
        game_account.players.push(Player {
            address: *ctx.accounts.player.to_account_info().key,
            number,
        });

        // If this is the sixth player, reveal the winner
        if game_account.players.len() == 6 {

            let oracle_queue_account = &ctx.accounts.oracle_queue_account;

            // Request VRF from Switchboard (adapt to your specific Switchboard setup)
            let vrf_key = switchboard_program::instruction::request_randomness(
                ctx.accounts.oracle_queue_account.to_account_info(),
                ctx.accounts.game_account.to_account_info(), // Callback address
                // ... other parameters as needed
            )?;

            // Store the vrf_key in the game account for later retrieval
            game_account.vrf_key = vrf_key;
        }

        Ok(())
    }

    pub fn settle_game(ctx: Context<SettleGame>) -> ProgramResult {
        let game_account = &mut ctx.accounts.game_account;
        let oracle_queue_account = &ctx.accounts.oracle_queue_account;
    
        // Fetch the latest VRF result
        let latest_round = oracle_queue_account.load_current_round_data()?;
        let random_number = (latest_round.result % 6) + 1;
    
        // Determine the winning player
        let winning_player = game_account.players.iter().find(|p| p.number == random_number);

        // Determine the winning player based on the random number
        let winning_player = game_account.players.iter().find(|p| p.number == random_number);

        // Transfer funds to the winner
        if let Some(winner) = winning_player {
            let prize_amount = BET_AMOUNT * 42 / 10; // 4.2x multiplier
            token::transfer(
                ctx.accounts.into_context(token::Transfer {
                    from: game_account.to_account_info(),
                    to: winner.to_account_info(),
                    authority: game_account.to_account_info(),
                })?,
                prize_amount,
            )?;
        }

        // Reset the game state for the next round
        game_account.players.clear();
        game_account.vrf_key = [0; 32];
    }
}
