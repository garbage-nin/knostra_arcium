use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct CreateMarket<'info> {
    #[account(
        init,
        payer = user,
        space = 8 + MarketAccount::INIT_SPACE,
        seeds = [b"market", user.key().as_ref(), &seed.to_le_bytes()],
        bump,
    )]
    pub market_account: Account<'info, MarketAccount>,

    #[account(
        init,
        payer = user,
        space = 8 + TreasuryAccount::INIT_SPACE,
        seeds = [b"treasury", market_account.key().as_ref()],
        bump,
    )]
    pub treasury_account: Account<'info, TreasuryAccount>,

    #[account(
        mut,
        seeds = [b"treasury_vault", market_account.key().as_ref()],
        bump,
    )]
    pub treasury_vault: SystemAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateMarket<'info> {
    pub fn create_market(
        &mut self,
        seed: u64,
        params: CreateMarketParams,
        bump: u8,
        treasury_bump: u8,
    ) -> Result<()> {
        let clock = Clock::get()?;
        self.market_account.set_inner(MarketAccount {
            bump,
            owner: self.user.key(),
            name: params.name,
            description: params.description,
            token: params.token,
            market_start: params.market_start,
            market_end: params.market_end,
            relational_value: params.relational_value,
            target_value: params.target_value,
            status: Status::NotStarted,
            required_bet_amount: params.required_bet_amount,
            max_player_count: params.max_player_count,
            created_at: clock.unix_timestamp,
            updated_at: clock.unix_timestamp,
            resolve_value: 0,
            market_id: seed,
        });

        self.treasury_account.set_inner(TreasuryAccount {
            market_account: self.market_account.key(),
            bump: treasury_bump,
            total_amount: 0,
            fee_amount: 0,
            no_count: 0,
            yes_count: 0,
            creator_fee_amount: 0,
            status: Status::NotStarted,
            creator: self.user.key(),
        });
        Ok(())
    }
}

pub fn handle_create_market(
    ctx: Context<CreateMarket>,
    seed: u64,
    params: CreateMarketParams,
    bump: u8,
    treasury_bump: u8,
) -> Result<()> {
    ctx.accounts
        .create_market(seed, params, bump, treasury_bump)
}
