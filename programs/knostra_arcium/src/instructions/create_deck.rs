use crate::errors::CustomError;
use crate::state::*;
use anchor_lang::prelude::*;
use mpl_core::accounts::BaseAssetV1;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct CreateDeckAccount<'info> {
    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + DeckAccount::INIT_SPACE,
        seeds = [b"deck", owner.key().as_ref(), &seed.to_le_bytes()],
        bump,
    )]
    pub deck_account: Account<'info, DeckAccount>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateDeckAccount<'info> {
    pub fn create_deck_account(
        &mut self,
        _seed: u64,
        mints: Vec<Pubkey>,
        bump: u8,
        remaining_accounts: &[AccountInfo<'info>], // 👈 lifetime must match
    ) -> Result<()> {
        let deck_account = &mut self.deck_account;

        deck_account.set_inner(DeckAccount {
            owner: self.owner.key(),
            bump,
            nfts: Vec::new(),
        });

        for (i, mint) in mints.iter().enumerate() {
            let asset_info = &remaining_accounts[i];

            require_keys_eq!(*asset_info.key, *mint, CustomError::InvalidMint);

            let data = asset_info.try_borrow_data()?;
            let asset = BaseAssetV1::deserialize(&mut data.as_ref())
                .map_err(|_| CustomError::InvalidMint)?;

            // require_eq!(asset.key, MplKey::AssetV1, CustomError::InvalidMint);
            
            require_keys_eq!(asset.owner, self.owner.key(), CustomError::NotNftOwner);

            deck_account.nfts.push(*mint);
        }

        Ok(())
    }
}

pub fn handle_create_deck<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateDeckAccount<'info>>,
    seed: u64,
    mints: Vec<Pubkey>,
    bump: u8,
) -> Result<()> {
    ctx.accounts
        .create_deck_account(seed, mints, bump, &ctx.remaining_accounts)
}
