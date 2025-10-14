use crate::errors::CustomError;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct ClaimFees<'info> {
    #[account(
        mut,
        has_one = market_account,
        seeds = [b"treasury", market_account.key().as_ref()],
        bump = treasury_account.bump,
    )]
    pub treasury_account: Account<'info, TreasuryAccount>,

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

impl<'info> ClaimFees<'info> {
    fn claim_fees(&mut self, vault_bump: u8) -> Result<()> {
        let treasury_account = &mut self.treasury_account;
        let market_account = &self.market_account;
        let treasury_vault = &mut self.treasury_vault;

        require!(
            market_account.status == Status::ResolvedNo
                || market_account.status == Status::ResolvedYes,
            CustomError::InvalidMarketStatus
        );
        require!(
            treasury_account.creator == self.user.key(),
            CustomError::Unauthorized
        );
        require!(treasury_account.fee_amount > 0, CustomError::NoFeesToClaim);

        let creator_fee = treasury_account
            .creator_fee_amount
            .checked_add(0)
            .ok_or(CustomError::MathOverflow)?;

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

        transfer(cpi_ctx, creator_fee)?;

        // TODO: Implement platform fee withdrawal
        // Currently set to 0 for simplicity
        // let platform_fee = treasury_account
        //     .fee_amount
        //     .checked_sub(0)
        //     .ok_or(CustomError::MathOverflow)?;

        // **treasury_account
        //     .to_account_info()
        //     .try_borrow_mut_lamports()? -= platform_fee;
        // **self
        //     .system_program
        //     .to_account_info()
        //     .try_borrow_mut_lamports()? += platform_fee;

        //treasury_account.fee_amount = 0;
        treasury_account.creator_fee_amount = 0;

        Ok(())
    }
}

pub fn handle_claim_fees(ctx: Context<ClaimFees>) -> Result<()> {
    let vault_bump = ctx.bumps.treasury_vault;
    ctx.accounts.claim_fees(vault_bump)
}
