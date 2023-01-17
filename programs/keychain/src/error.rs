use anchor_lang::prelude::*;

#[error_code]
pub enum KeychainError {
    #[msg("You don't have enough SOL")]
    NotEnoughSol,
    #[msg("The given key account is not the correct PDA for the given address")]
    IncorrectKeyAddress,
    #[msg("That key already exists")]
    KeyAlreadyExists,
    #[msg("You cannot add any more keys on your keychain. Remove one first")]
    MaxKeys,
    #[msg("You are not a valid signer for this keychain")]
    SignerNotInKeychain,
    #[msg("Verifier must be the same as the key being verified")]
    SignerNotKey,
    #[msg("That key doesn't exist on this keychain")]
    KeyNotFound,
    #[msg("Signer is not a domain admin")]
    NotDomainAdmin,
    #[msg("Can only add wallet of signer")]
    NotSigner,
    #[msg("Name too long. Max 32 characters")]
    NameTooLong,
    #[msg("Wrong treasury account")]
    WrongTreasury,
    #[msg("Wrong keychain version")]
    InvalidKeychainVersion

}
