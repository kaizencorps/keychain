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
    #[msg("Token unit listings not allowed")]
    TokenUnitListingsNotAllowed,

    #[msg("Missing a buyer's item token account")]
    MissingBuyerItemToken,
    #[msg("Missing a listing's item token account")]
    MissingListingItemToken,



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
    #[msg("Insufficient units")]
    InsufficientUnits,

    #[msg("The number of provided item quantities does not match the number of items in the listing")]
    ItemQuantitiesMismatch,

}
