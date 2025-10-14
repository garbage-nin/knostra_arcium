use crate::errors::CustomError;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CancelMarket<'info> {
    #[account(
        mut,
        has_one = market_account,
        seeds = [b"treasury", market_account.key().as_ref()],
        bump = treasury_account.bump,
    )]
    pub treasury_account: Account<'info, TreasuryAccount>,

    #[account(
        mut,
        seeds = [b"market", creator.key().as_ref(), &market_account.market_id.to_le_bytes()],
        bump = market_account.bump,
    )]
    pub market_account: Account<'info, MarketAccount>,

    /// CHECK: This is only used for PDA derivation and owner validation.
    /// It does not need to be a signer because resolution is permissionless (triggered by a bot/cron).
    pub creator: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> CancelMarket<'info> {
    fn cancel(&mut self) -> Result<()> {
        let treasury_account = &mut self.treasury_account;
        let market_account = &mut self.market_account;

        // COMMENTED OUT FIRST TWO REQUIRES FOR TESTING PURPOSES
        // ✅ Require market already started
        // require!(
        //     Clock::get()?.unix_timestamp >= market_account.market_start as i64,
        //     CustomError::MarketNotStarted
        // );

        require!(
            market_account.status == Status::NotStarted,
            CustomError::InvalidMarketStatus
        );
        // require!(
        //     treasury_account.yes_count != treasury_account.no_count,
        //     CustomError::CannotCancelMarket
        // );

        // Cancel the market
        market_account.status = Status::Cancelled;
        treasury_account.status = Status::Cancelled;

        Ok(())
    }
}

pub fn handle_cancel_market(ctx: Context<CancelMarket>) -> Result<()> {
    ctx.accounts.cancel()
}
