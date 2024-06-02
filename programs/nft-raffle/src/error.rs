use anchor_lang::prelude::*;

#[error_code]
pub enum RaffleError {
    #[msg("Invalid Metadata Address")]
    InvalidMetadata,
    #[msg("Invalid Collections")]
    InvalidCollection,
    #[msg("Can't Parse The NFT's Creators")]
    MetadataCreatorParseError,
    #[msg("The number of max entrants is too large")]
    MaxEntrantsTooLarge,
    #[msg("End time is too early")]
    EndTimeTooEarly,
    #[msg("This raffle has ended")]
    RaffleEnded,
    #[msg("There aren't enough tickets left")]
    NotEnoughTicketLeft,
    #[msg("You don't have enough SOL")]
    NotEnoughSOL,
    #[msg("This raffle is not over")]
    RaffleNotEnded,
    #[msg("There's no entrants in this raffle")]
    RaffleHasNoEntrants,
    #[msg("There are no rewards to claim")]
    NoRewards,
    #[msg("You are not the winner")]
    NotWinner,
    #[msg("You are not the creator of this raffle")]
    NotCreator,
    #[msg("This raffle has some entrants")]
    HasEntrants,
}