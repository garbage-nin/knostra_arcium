#![allow(deprecated)]
use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;

const COMP_DEF_OFFSET_ADD_TOGETHER: u32 = comp_def_offset("add_together");
const COMP_DEF_OFFSET_INIT_GAME: u32 = comp_def_offset("init_game");
const COMP_DEF_OFFSET_JOIN_GAME: u32 = comp_def_offset("join_game");

pub mod errors;
pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;
use crate::errors::CustomError;
declare_id!("8KmHKtMP2hsBjk1NEySV3ukWAaUCoxRV22iHcG1YmCWv");

#[arcium_program]
pub mod knostra_arcium {
    use super::*;

    pub fn init_add_together_comp_def(ctx: Context<InitAddTogetherCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }
    
    pub fn init_init_game_comp_def(ctx: Context<InitInitGameCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    pub fn init_join_game_comp_def(ctx: Context<InitJoinGameCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    pub fn add_together(
        ctx: Context<AddTogether>,
        computation_offset: u64,
        ciphertext_0: [u8; 32],
        ciphertext_1: [u8; 32],
        pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let args = vec![
            Argument::ArcisPubkey(pub_key),
            Argument::PlaintextU128(nonce),
            Argument::EncryptedU8(ciphertext_0),
            Argument::EncryptedU8(ciphertext_1),
        ];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![AddTogetherCallback::callback_ix(&[])],
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "add_together")]
    pub fn add_together_callback(
        ctx: Context<AddTogetherCallback>,
        output: ComputationOutputs<AddTogetherOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(AddTogetherOutput { field_0 }) => field_0,
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        emit!(SumEvent {
            sum: o.ciphertexts[0],
            nonce: o.nonce.to_le_bytes(),
        });
        Ok(())
    }

    pub fn init_game(
        ctx: Context<InitGame>,
        computation_offset: u64,
        id: u64,
        nonce: u128,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game_account;

        game.market_account = game.market_account.key();
        game.game_id = id;
        game.nonce = nonce;

        game.player_yes = Pubkey::default();
        game.player_no = Pubkey::default();
        game.player_yes_deck = Pubkey::default();
        game.player_no_deck = Pubkey::default();

        // Start game state defaults
        game.current_turn = 0;
        game.result = 0;

        game.bump = ctx.bumps.game_account;

        let args = vec![Argument::PlaintextU128(nonce)];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![InitGameCallback::callback_ix(&[CallbackAccount {
                pubkey: ctx.accounts.game_account.key(),
                is_writable: true,
            }])],
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "init_game")]
    pub fn init_game_callback(
        ctx: Context<InitGameCallback>,
        output: ComputationOutputs<InitGameOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(InitGameOutput { field_0 }) => field_0,
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let nonce = o.nonce;

        let card_ciphertexts: [[u8; 32]; 6] = o.ciphertexts;

        let game = &mut ctx.accounts.game_account;

        game.yes_cards1 = card_ciphertexts[0];
        game.yes_cards2 = card_ciphertexts[1];
        game.yes_cards3 = card_ciphertexts[2];
        game.no_cards1 = card_ciphertexts[3];
        game.no_cards2 = card_ciphertexts[4];
        game.no_cards3 = card_ciphertexts[5];

        game.nonce = nonce;

        Ok(())
    }

    pub fn join_game(
        ctx: Context<JoinGame>,
        computation_offset: u64,
        player_cards1: [u8; 32],
        player_cards2: [u8; 32],
        player_cards3: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let payer_key = ctx.accounts.payer.key();
        let game_key = ctx.accounts.game_account.key();
        let game_nonce = ctx.accounts.game_account.nonce;

        let bet_account = &mut ctx.accounts.bet_account;
        let deck_account = &mut ctx.accounts.deck_account;
        let game = &mut ctx.accounts.game_account;

        require_keys_eq!(bet_account.user, payer_key, CustomError::InvalidPayer);

        // Determine player side: 0 = yes, 1 = no
        let player_side = if bet_account.choice { 0 } else { 1 };

        // Assign the player and deck if not already joined
        if player_side == 0 {
            if game.player_yes == Pubkey::default() {
                game.player_yes = payer_key;
                game.player_yes_deck = deck_account.key();
            } else {
                return Err(CustomError::PlayerAlreadyJoined.into());
            }
        } else {
            if game.player_no == Pubkey::default() {
                game.player_no = payer_key;
                game.player_no_deck = deck_account.key();
            } else {
                return Err(CustomError::PlayerAlreadyJoined.into());
            }
        }

        // Prepare Arcium encrypted computation args
        let args = vec![
            Argument::ArcisPubkey(payer_key.to_bytes()),
            Argument::PlaintextU128(nonce),
            Argument::PlaintextU8(player_side),
            Argument::EncryptedU8(player_cards1),
            Argument::EncryptedU8(player_cards2),
            Argument::EncryptedU8(player_cards3),
            Argument::PlaintextU128(game_nonce),
            Argument::Account(game_key, 8, 32 * 6),
        ];

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![JoinGameCallback::callback_ix(&[CallbackAccount {
                pubkey: game_key,
                is_writable: true,
            }])],
        )?;

        Ok(())
    }


    #[arcium_callback(encrypted_ix = "join_game")]
    pub fn join_game_callback(
        ctx: Context<JoinGameCallback>,
        output: ComputationOutputs<JoinGameOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(JoinGameOutput { field_0 }) => field_0,
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let nonce = o.nonce;

        let card_ciphertexts: [[u8; 32]; 6] = o.ciphertexts;

        let game = &mut ctx.accounts.game_account;

        game.yes_cards1 = card_ciphertexts[0];
        game.yes_cards2 = card_ciphertexts[1];
        game.yes_cards3 = card_ciphertexts[2];
        game.no_cards1 = card_ciphertexts[3];
        game.no_cards2 = card_ciphertexts[4];
        game.no_cards3 = card_ciphertexts[5];

        game.nonce = nonce;

        Ok(())
    }

    pub fn create(
        ctx: Context<CreateMarket>,
        seed: u64,
        params: CreateMarketParams,
        bump: u8,
        treasury_bump: u8,
    ) -> Result<()> {
        instructions::handle_create_market(ctx, seed, params, bump, treasury_bump)
    }

    pub fn bet(ctx: Context<PlaceBet>, amount: u64, choice: bool, bump: u8) -> Result<()> {
        instructions::handle_place_bet(ctx, amount, choice, bump)
    }
    pub fn resolve(ctx: Context<ResolveMarket>, resolve_value: u64) -> Result<()> {
        instructions::handle_resolve_market(ctx, resolve_value)
    }

    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        instructions::handle_claim(ctx)
    }

    pub fn cancel(ctx: Context<CancelMarket>) -> Result<()> {
        instructions::handle_cancel_market(ctx)
    }

    pub fn claim_fees(ctx: Context<ClaimFees>) -> Result<()> {
        instructions::handle_claim_fees(ctx)
    }

    pub fn create_deck<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateDeckAccount<'info>>,
        seed: u64,
        mints: Vec<Pubkey>,
        bump: u8,
    ) -> Result<()> {
        instructions::handle_create_deck(ctx, seed, mints, bump)
    }
}



#[queue_computation_accounts("add_together", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct AddTogether<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!()
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_ADD_TOGETHER)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("add_together")]
#[derive(Accounts)]
pub struct AddTogetherCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_ADD_TOGETHER)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("add_together", payer)]
#[derive(Accounts)]
pub struct InitAddTogetherCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    /// Can't check it here as it's not initialized yet.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("init_game", payer)]
#[derive(Accounts)]
pub struct InitInitGameCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    /// Can't check it here as it's not initialized yet.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("init_game", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, id: u64)]
pub struct InitGame<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!()
    )]
    /// CHECK: mempool_account, checked by the arcium program
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    /// CHECK: executing_pool, checked by the arcium program
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_GAME)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(init,
        payer = payer,
        space = 8 + GameAccount::INIT_SPACE,
        seeds = [b"rps_game", id.to_le_bytes().as_ref()],
        bump,
    )]
    pub game_account: Account<'info, GameAccount>,
}

#[callback_accounts("init_game")]
#[derive(Accounts)]
pub struct InitGameCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_GAME)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub game_account: Account<'info, GameAccount>,
}

#[init_computation_definition_accounts("join_game", payer)]
#[derive(Accounts)]
pub struct InitJoinGameCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    /// Can't check it here as it's not initialized yet.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}


#[callback_accounts("join_game")]
#[derive(Accounts)]
pub struct JoinGameCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_JOIN_GAME)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub game_account: Account<'info, GameAccount>,
}

#[queue_computation_accounts("join_game", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct JoinGame<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!()
    )]
    /// CHECK: mempool_account, checked by the arcium program
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    /// CHECK: executing_pool, checked by the arcium program
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_JOIN_GAME)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(mut)]
    pub game_account: Account<'info, GameAccount>,
    #[account(mut)]
    pub deck_account: Account<'info, DeckAccount>,
    #[account(mut)]
    pub bet_account: Account<'info, BetAccount>
}

#[event]
pub struct SumEvent {
    pub sum: [u8; 32],
    pub nonce: [u8; 16],
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
}

