use crate::errors::CustomError;
use crate::state::*;
use anchor_lang::prelude::*;
// use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(
        mut,
        has_one = owner,
        seeds = [b"market", owner.key().as_ref(), &market_account.market_id.to_le_bytes()],
        bump = market_account.bump,
    )]
    pub market_account: Account<'info, MarketAccount>,

    #[account(
        mut,
        has_one = market_account,
        seeds = [b"treasury", market_account.key().as_ref()],
        bump = treasury_account.bump,
    )]
    pub treasury_account: Account<'info, TreasuryAccount>,

    /// CHECK: Only this PDA can resolve
    #[account(seeds = [b"resolver_authority"], bump)]
    pub resolver_authority: UncheckedAccount<'info>,

    /// CHECK: Market owner (used for validation)
    pub owner: UncheckedAccount<'info>,

    // pub price_update: Account<'info, PriceUpdateV2>,

    pub system_program: Program<'info, System>,
}

impl<'info> ResolveMarket<'info> {
    pub fn resolve_market(&mut self, resolve_value: u64, program_id: &Pubkey) -> Result<()> {
        let (expected_resolver, _) =
            Pubkey::find_program_address(&[b"resolver_authority"], program_id);

        require_keys_eq!(
            expected_resolver,
            self.resolver_authority.key(),
            CustomError::UnauthorizedResolver
        );
        // let price_update = &mut self.price_update;
        // // get_price_no_older_than will fail if the price update is more than 30 seconds old
        // let maximum_age: u64 = 30;
        // // get_price_no_older_than will fail if the price update is for a different price feed.
        // // This string is the id of the BTC/USD feed. See https://docs.pyth.network/price-feeds/price-feeds for all available IDs.
        // let feed_id: [u8; 32] = get_feed_id_from_hex(
        //     "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
        // )?;
        // let price = price_update.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;
        // // Sample output:
        // // The price is (7160106530699 ± 5129162301) * 10^-8
        // msg!(
        //     "The price is ({} ± {}) * 10^{}",
        //     price.price,
        //     price.conf,
        //     price.exponent
        // );

        let market_account = &mut self.market_account;
        let treasury_account = &mut self.treasury_account;

        require!(
            market_account.status == Status::Ongoing,
            CustomError::InvalidMarketStatus
        );

        let outcome_yes = match market_account.relational_value.as_str() {
            ">=" => resolve_value >= market_account.target_value,
            "<=" => resolve_value <= market_account.target_value,
            ">" => resolve_value > market_account.target_value,
            "<" => resolve_value < market_account.target_value,
            "==" => resolve_value == market_account.target_value,
            _ => return Err(CustomError::InvalidRelationalOp.into()),
        };

        market_account.status = if outcome_yes {
            Status::ResolvedYes
        } else {
            Status::ResolvedNo
        };

        market_account.resolve_value = resolve_value;
        market_account.updated_at = Clock::get()?.unix_timestamp;
        treasury_account.status = market_account.status;

        Ok(())
    }
}

pub fn handle_resolve_market(ctx: Context<ResolveMarket>, resolve_value: u64) -> Result<()> {
    ctx.accounts.resolve_market(resolve_value, ctx.program_id)
}
