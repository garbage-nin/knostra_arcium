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

#[derive(InitSpace)]
#[account]
pub struct DeckAccount {
    pub owner: Pubkey,
    #[max_len(20)]
    pub nfts: Vec<Pubkey>,
    pub bump: u8,
}

#[derive(InitSpace)]
#[account]
pub struct GameAccount {
    pub market_account: Pubkey,
    pub player_yes: Pubkey,
    pub player_yes_deck: Pubkey,
    pub player_no: Pubkey,
    pub player_no_deck: Pubkey,
    pub yes_cards1: [u8; 32],
    pub yes_cards2: [u8; 32],
    pub yes_cards3: [u8; 32],
    pub no_cards1: [u8; 32],
    pub no_cards2: [u8; 32],
    pub no_cards3: [u8; 32],
    pub current_turn: u8,
    pub result: u8,
    pub bump: u8,
    pub nonce: u128,
    pub game_id: u64,
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
