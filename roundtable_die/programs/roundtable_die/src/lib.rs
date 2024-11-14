use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use orao_vrf_solana::program::OraoVrfSolana;
use orao_vrf_solana::state::VrfAccountData; // Adjust the path based on the ORAO crate

declare_id!("GFjQ5uhLf4K9p3JgF4nGHp7mQswqzU6p9y9BgBykhUMX");

#[program]
pub mod number_guessing_game {

    #[account]
    pub struct ChooseNumber {
        pub players: Vec<Player>,
        pub vrf_key: Pubkey,
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
        pub oracle_queue_account: Account<'info, VrfAccountData>,
    }

    #[event]
    pub struct PlayerChoseNumber {
        pub player: Pubkey,
        pub chosen_number: u8,
        pub bet_amount: u64,
        pub bet_currency: String,
        pub timestamp: i64,
    }

    #[event]
    pub struct PlayerWon {
        pub player: Pubkey,
        pub winning_number: u8,
        pub prize_amount: u64,
        pub prize_currency: String,
        pub timestamp: i64,
    }

    pub fn choose_number(ctx: Context<ChooseNumber>, number: u8) -> ProgramResult {
        const BET_AMOUNT: u64 = 10; // Adjust the bet amount as needed

        // Program account that stores the game state
        let game_account = &mut ctx.accounts.game_account;
        let player_account = &ctx.accounts.player;
        // game_account.bet_amount = BET_AMOUNT;

        // Check if the number is valid and not already chosen
        require!(number > 0 && number <= 6, CustomError::InvalidNumber);
        require!(
            game_account.players.iter().all(|p| p.number != number),
            CustomError::NumberAlreadyChosen
        );
        require!(
            player_account.lamports() >= BET_AMOUNT,
            CustomError::InsufficientFunds
        );

        // Add the player to the game
        game_account.players.push(Player {
            address: *player_account.to_account_info().key,
            number,
        });

        // Transfers bet from player to game
        token::transfer(
            ctx.accounts.into_context(token::Transfer {
                from: player_account.to_account_info(),
                to: game_account.to_account_info(),
                authority: player_account.to_account_info(),
            })?,
            required_lamports,
        )?;

        emit!(PlayerChoseNumber {
            player: *ctx.accounts.player.key,
            chosen_number,
            bet_amount: BET_AMOUNT,
            bet_currency: "SOL".to_string(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        // If this is the sixth player, reveal the winner
        if game_account.players.len() == 6 {
            let oracle_queue_account = &ctx.accounts.oracle_queue_account;

            let vrf_key = make_oraro_vrf_request(
                ctx.program_id,
                ctx.accounts.game_account.key(),
                ctx.accounts.oracle_account.to_account_info(),
                // settle_game
            )?;
            game_account.vrf_key = vrf_key;


            // Request VRF from Switchboard (adapt to your specific Switchboard setup)
            // let vrf_key = switchboard_program::instruction::request_randomness(
            //     ctx.accounts.oracle_queue_account.to_account_info(),
            //     ctx.accounts.game_account.to_account_info(), // Callback address
            //     // ... other parameters as needed
            // )?;

            // Store the vrf_key in the game account for later retrieval
            // game_account.vrf_key = vrf_key;
        }

        Ok(())
    }

    pub fn settle_game(ctx: Context<SettleGame>) -> ProgramResult {
        let game_account = &mut ctx.accounts.game_account;
        let vrf_account = &ctx.accounts.vrf_account;

         // Retrieve the randomness result using `vrf_key`.
        if game_account.vrf_key != *vrf_account.to_account_info().key {
            return Err(ProgramError::InvalidArgument.into());
        }

        let result = vrf_account.result.ok_or(CustomError::RandomnessNotAvailable)?;
        let winning_number = (result % 6) + 1;
        let winning_player = game_account.players.iter().find(|p| p.number == winning_number);

        // Determine the winning player based on the random number
        let winning_player = game_account
            .players
            .iter()
            .find(|p| p.number == winning_number);

        // Transfer funds to the winner
        if let Some(winner) = winning_player {
            let prize_amount = BET_AMOUNT + BET_AMOUNT * 42 / 10; // 4.2x multiplier + original bet
            token::transfer(
                ctx.accounts.into_context(token::Transfer {
                    from: game_account.to_account_info(),
                    to: winner.to_account_info(),
                    authority: game_account.to_account_info(),
                })?,
                prize_amount,
            )?;

            emit!(PlayerWon {
                player: *ctx.accounts.player.key,
                winning_number,
                prize_amount,
                prize_currency: "SOL".to_string(),
                timestamp: Clock::get()?.unix_timestamp,
            });
        }

        // Reset the game state for the next round
        game_account.players.clear();
        game_account.vrf_key = Pubkey::default;

        Ok(())
    }
}

/// Helper function to make an ORAO VRF request.
fn make_oraro_vrf_request(
    program_id: &Pubkey,
    game_account_key: &Pubkey,
    vrf_account_info: AccountInfo<'_>,
) -> Result<Pubkey, ProgramError> {
    let callback_ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*game_account_key, false),
        ],
        data: number_guessing_game::instruction::SettleGame {},
        // maybe this? 

        // Get the function discriminator for "settle_game"
        // let settle_game_discriminator = anchor_lang::sighash("global", "settle_game");

        // // Prepare callback data (8-byte discriminator for "settle_game" and additional parameters)
        // let callback_data = settle_game_discriminator.to_vec();
        // data: callback_data,
    };

    orao_vrf_solana::instruction::request_randomness(
        vrf_account_info,
        game_account_key, // Callback address
        callback_ix,
    )?;

    // Return the vrf_key (you may need to adapt based on the ORAO setup).
    Ok(vrf_account_info.key.clone())
}