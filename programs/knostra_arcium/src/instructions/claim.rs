use crate::errors::CustomError;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(
        mut,
        has_one = market_account,
        seeds = [b"treasury", market_account.key().as_ref()],
        bump = treasury_account.bump,
    )]
    pub treasury_account: Account<'info, TreasuryAccount>,

    #[account(
        mut,
        has_one = market_account,
        has_one = user,
        seeds = [b"bet", market_account.key().as_ref(), user.key().as_ref()],
        bump = bet_account.bump,
    )]
    pub bet_account: Account<'info, BetAccount>,

    #[account(
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

impl<'info> Claim<'info> {
    fn claim(&mut self, vault_bump: u8) -> Result<()> {
        let bet_account = &mut self.bet_account;
        let treasury_account = &mut self.treasury_account;
        let market_account = &mut self.market_account;
        let treasury_vault = &mut self.treasury_vault;

        require!(
            market_account.status == Status::ResolvedYes
                || market_account.status == Status::ResolvedNo
                || market_account.status == Status::Cancelled,
            CustomError::InvalidMarketStatus
        );

        require!(!bet_account.claimed, CustomError::AlreadyClaimed);

        if market_account.status == Status::Cancelled {
            require!(
                treasury_account.total_amount >= bet_account.bet_amount,
                CustomError::InsufficientTreasury
            );

            let market_key = market_account.key();
            let treasury_seeds = &[b"treasury_vault", market_key.as_ref(), &[vault_bump]];
            let signer_seeds = &[&treasury_seeds[..]];

            let cpi_accounts = Transfer {
                from: treasury_vault.to_account_info(),
                to: self.user.to_account_info(),
            };

            let cpi_ctx = CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );

            transfer(cpi_ctx, bet_account.bet_amount)?;

            bet_account.claimed = true;

            return Ok(());
        }

        let is_winner = (market_account.status == Status::ResolvedYes && bet_account.choice)
            || (market_account.status == Status::ResolvedNo && !bet_account.choice);

        if is_winner {
            let mut payout = bet_account
                .bet_amount
                .checked_mul(2)
                .ok_or(CustomError::MathOverflow)?;

            // TODO: Make fee configurable
            // Deduct platform fees
            let total_fee_bps: u64 = 200; // 2%
            let creator_fee_bps: u64 = 100; // 1%
            let protocol_fee_bps: u64 = 100; // 1%

            let total_fee = payout
                .checked_mul(total_fee_bps)
                .ok_or(CustomError::MathOverflow)?
                .checked_div(10_000)
                .ok_or(CustomError::MathOverflow)?;

            let creator_fee = payout
                .checked_mul(creator_fee_bps)
                .ok_or(CustomError::MathOverflow)?
                .checked_div(10_000)
                .ok_or(CustomError::MathOverflow)?;

            let protocol_fee = total_fee
                .checked_sub(protocol_fee_bps)
                .ok_or(CustomError::MathOverflow)?;

            payout = payout
                .checked_sub(total_fee)
                .ok_or(CustomError::MathOverflow)?;

            require!(
                treasury_account.total_amount >= payout + total_fee,
                CustomError::InsufficientTreasury
            );

            let market_key = market_account.key();
            let treasury_seeds = &[b"treasury_vault", market_key.as_ref(), &[vault_bump]];
            let signer_seeds = &[&treasury_seeds[..]];

            let cpi_accounts = Transfer {
                from: treasury_vault.to_account_info(),
                to: self.user.to_account_info(),
            };

            let cpi_ctx = CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );

            transfer(cpi_ctx, payout)?;

            treasury_account.fee_amount = treasury_account
                .fee_amount
                .checked_add(protocol_fee)
                .ok_or(CustomError::MathOverflow)?;

            treasury_account.creator_fee_amount = treasury_account
                .creator_fee_amount
                .checked_add(creator_fee)
                .ok_or(CustomError::MathOverflow)?;

            treasury_account.total_amount = treasury_account
                .total_amount
                .checked_sub(payout + total_fee)
                .ok_or(CustomError::MathOverflow)?;

            bet_account.claimed = true;
        } else {
            return err!(CustomError::NotAWinner);
        }

        Ok(())
    }
}

pub fn handle_claim(ctx: Context<Claim>) -> Result<()> {
    let vault_bump = ctx.bumps.treasury_vault;
    ctx.accounts.claim(vault_bump)
}
