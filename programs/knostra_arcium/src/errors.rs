use anchor_lang::prelude::*;
#[error_code]
pub enum CustomError {
    #[msg("The relational operator is invalid")]
    InvalidRelationalOp,

    #[msg("Market is not in the correct status")]
    InvalidMarketStatus,

    #[msg("Market has not ended yet")]
    MarketNotEnded,

    #[msg("Market has not started")]
    MarketNotStarted,

    #[msg("Market cannot be cancelled")]
    CannotCancelMarket,

    #[msg("You are not a winner")]
    NotAWinner,

    #[msg("Bet already claimed")]
    AlreadyClaimed,

    #[msg("Bet amount is invalid")]
    InvalidBetAmount,

    #[msg("Math operation overflowed")]
    MathOverflow,

    #[msg("Insufficient funds to place bet")]
    InsufficientTreasury,

    #[msg("No fees to claim")]
    NoFeesToClaim,

    #[msg("Unauthorized user to claim fees")]
    Unauthorized,

    #[msg("Maximum number of players reached")]
    MaxPlayersReached,

    #[msg("Unauthorized resolver authority")]
    UnauthorizedResolver,

    #[msg("Deck is full")]
    DeckFull,

    #[msg("You do not own the specified NFT")]
    NotNftOwner,

    #[msg("Invalid NFT balance in associated token account")]
    InvalidNftBalance,

    #[msg("Required account is missing")]
    MissingAccount,

    #[msg("Invalid token account")]
    InvalidTokenAccount,

    #[msg("Invalid mint for the NFT")]
    InvalidMint,

    #[msg("Player has already joined the game")]
    PlayerAlreadyJoined,

    #[msg("Invalid Player")]
    InvalidPayer
}
