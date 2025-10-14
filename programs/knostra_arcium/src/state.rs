use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct MarketAccount {
    pub bump: u8,
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    #[max_len(256)]
    pub description: String,
    #[max_len(10)]
    pub token: String,
    pub market_start: u64,
    pub market_end: u64,
    #[max_len(5)]
    pub relational_value: String,
    pub target_value: u64,
    pub resolve_value: u64,
    pub status: Status,
    pub required_bet_amount: u64,
    pub max_player_count: u64,
    pub market_id: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(InitSpace)]
#[account]
pub struct TreasuryAccount {
    pub market_account: Pubkey,
    pub creator: Pubkey,
    pub bump: u8,
    pub total_amount: u64,
    pub fee_amount: u64,
    pub creator_fee_amount: u64,
    pub yes_count: u64,
    pub no_count: u64,
    pub status: Status,
}

#[derive(InitSpace)]
#[account]
pub struct BetAccount {
    pub market_account: Pubkey,
    pub user: Pubkey,
    pub bump: u8,
    pub bet_amount: u64,
    pub choice: bool,
    pub claimed: bool,
}

#[derive(InitSpace, AnchorSerialize, AnchorDeserialize, Copy, Clone, PartialEq)]
pub enum Status {
    NotStarted,
    Ongoing,
    ResolvedYes,
    ResolvedNo,
    Completed,
    Cancelled,
}

#[derive(InitSpace, AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub struct CreateMarketParams {
    #[max_len(32)]
    pub name: String,
    #[max_len(256)]
    pub description: String,
    #[max_len(10)]
    pub token: String,
    pub market_start: u64,
    pub market_end: u64,
    #[max_len(5)]
    pub relational_value: String,
    pub target_value: u64,
    pub required_bet_amount: u64,
    pub max_player_count: u64,
}
