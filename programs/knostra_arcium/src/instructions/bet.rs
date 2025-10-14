use crate::errors::CustomError;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(
        mut,
        has_one = market_account,
        seeds = [b"treasury", market_account.key().as_ref()],
        bump = treasury_account.bump,
    )]
    pub treasury_account: Account<'info, TreasuryAccount>,

    #[account(
        init,
        payer = user,
        space = 8 + BetAccount::INIT_SPACE,
        seeds = [b"bet", market_account.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub bet_account: Account<'info, BetAccount>,

    #[account(
        mut,
        seeds = [b"market", market_account.owner.as_ref(), &market_account.market_id.to_le_bytes()],
        bump = market_account.bump,
    )]
    pub market_account: Account<'info, MarketAccount>,

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

impl<'info> PlaceBet<'info> {
    fn place_bet(&mut self, bet_amount: u64, choice: bool, bump: u8) -> Result<()> {
        let bet_account = &mut self.bet_account;
        let treasury_account = &mut self.treasury_account;
        let market_account = &mut self.market_account;
        let treasury_vault = &mut self.treasury_vault;

        require!(
            market_account.status == Status::NotStarted,
            CustomError::InvalidMarketStatus
        );

        require!(
            bet_amount == market_account.required_bet_amount,
            CustomError::InvalidBetAmount
        );
        require!(
            treasury_account.yes_count <= market_account.max_player_count
                || treasury_account.no_count <= market_account.max_player_count,
            CustomError::MaxPlayersReached
        );
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: treasury_vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(self.system_program.to_account_info(), cpi_accounts);
        transfer(cpi_ctx, bet_amount)?;

        bet_account.set_inner(BetAccount {
            market_account: market_account.key(),
            user: self.user.key(),
            bump,
            bet_amount,
            choice,
            claimed: false,
        });

        treasury_account.total_amount = treasury_account
            .total_amount
            .checked_add(bet_amount)
            .unwrap();
        if choice {
            treasury_account.yes_count = treasury_account.yes_count.checked_add(1).unwrap();
        } else {
            treasury_account.no_count = treasury_account.no_count.checked_add(1).unwrap();
        }

        // This will start the market automatically when max players reached
        if market_account.status == Status::NotStarted
            && treasury_account.yes_count == treasury_account.no_count
            && treasury_account.yes_count == market_account.max_player_count
        {
            market_account.status = Status::Ongoing;
            treasury_account.status = Status::Ongoing;
        }

        Ok(())
    }
}

pub fn handle_place_bet(ctx: Context<PlaceBet>, amount: u64, choice: bool, bump: u8) -> Result<()> {
    ctx.accounts.place_bet(amount, choice, bump)
}
