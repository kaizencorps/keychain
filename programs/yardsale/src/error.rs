use anchor_lang::prelude::*;

#[error_code]
pub enum YardsaleError {
    #[msg("Not authorized")]
    NotAuthorized,
    #[msg("Invalid Keychain")]
    InvalidKeychain,
    #[msg("Invalid item")]
    InvalidItem,
    #[msg("Invalid price")]
    InvalidPrice,
    #[msg("Sale proceeds token account not specified")]
    ProceedsTokenAccountNotSpecified,
    #[msg("Sale proceeds account not specified")]
    ProceedsAccountNotSpecified,
    #[msg("Funding account not specified")]
    FundingAccountNotSpecified,
    #[msg("Insufficient funds")]
    InsufficientFunds,
}
