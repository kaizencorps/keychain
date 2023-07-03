use anchor_lang::error_code;

#[error_code]
pub enum BazaarError {

    #[msg("No items specified")]
    NoItemsSpecified,
    #[msg("Invalid item accounts")]
    InvalidItemAccounts,
    #[msg("Missing item quantities")]
    MissingItemQuantities,
    #[msg("Too many item quantities")]
    TooManyItemQuantities,
    #[msg("Missing item account")]
    MissingItemAccount,
    #[msg("Item quantities must be whole numbers")]
    InvalidItemQuantity,
    #[msg("Attempted to list more items than owned")]
    NotEnoughItems,
    #[msg("Too many items")]
    TooManyItems,

    // ported over from yardsale

    #[msg("Not authorized")]
    NotAuthorized,
    #[msg("Name too long")]
    NameTooLong,
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

    // pNFT shit
    #[msg("Bad Metadata")]
    BadMetadata,
    #[msg("Bad Ruleset")]
    BadRuleset,
    #[msg("TransferBuilder failed")]
    TransferBuilderFailed
}
