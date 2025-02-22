use anchor_lang::prelude::*;

#[error_code]
pub enum KeychainError {
    #[msg("You are not authorized to perform that action")]
    NotAuthorized,
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
    InvalidVerifier,
    #[msg("That key doesn't exist on this keychain")]
    KeyNotFound,
    #[msg("Signer is not a domain admin")]
    NotDomainAdmin,
    #[msg("Can only add wallet of signer")]
    NotSigner,
    #[msg("Invalid name. Must be lowercase + no spaces.")]
    InvalidName,
    #[msg("Name too long. Max 32 characters")]
    NameTooLong,
    #[msg("Name too short. Min 2 characters")]
    NameTooShort,
    #[msg("Invalid treasury")]
    InvalidTreasury,
    #[msg("Wrong keychain version")]
    InvalidKeychainVersion,
    #[msg("Missing required key account")]
    MissingKeyAccount,
    #[msg("Invalid Key account")]
    InvalidKeyAccount,
    #[msg("A pending action already exists")]
    PendingActionExists,
    #[msg("A pending action doesn't exist")]
    NoPendingAction,
    #[msg("Key not verified")]
    KeyNotVerified


}
