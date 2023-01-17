use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use keychain::account::{CurrentKeyChain, KeyChainState};
use keychain::program::Keychain;

declare_id!("ProoXffuU4NJWstjWgFYqauFFatDMG9xHETuRMjKMLt");

const PROFILE: &str = "profile";

#[program]
pub mod profile {
    use super::*;

    // create the profile
    pub fn create_profile(ctx: Context<CreateProfile>, username: String) -> Result <()> {

        require!(username.as_bytes().len() <= 32, ErrorCode::NameTooLong);

        let keychain = &ctx.accounts.keychain;
        let user = *ctx.accounts.user.to_account_info().key;

        // check that the signer is on the keychain
        let is_keychain_owner = check_key(keychain, user);
        require!(is_keychain_owner, ErrorCode::NotOnKeychain);

        let profile = &mut ctx.accounts.profile;
        profile.username = username;
        profile.keychain = ctx.accounts.keychain.key();

        msg!("created profile account: {}", ctx.accounts.profile.key());
        Ok(())
    }

    // sets the token account in the profile (but first checks if the owner is on the keychain)
    pub fn set_pfp(ctx: Context<SetPfp>) -> Result <()> {

        let keychain = &ctx.accounts.keychain;
        let user = *ctx.accounts.user.to_account_info().key;

        // first: check that the user is on the keychain
        let is_keychain_owner = check_key(keychain, user);
        require!(is_keychain_owner, ErrorCode::NotOnKeychain);

        // next: check that the owner of the token account is on the keychain and the token account isn't empty
        let is_pfp_owner = check_key(keychain, ctx.accounts.pfp_token_account.owner);
        require!(is_pfp_owner && ctx.accounts.pfp_token_account.amount == 1, ErrorCode::OwnerNotOnKeychain);

        // for more robust nft verification: https://medium.com/@Arrivant_/how-to-verify-nfts-in-an-anchor-program-a051299acde8

        // ok, now set the token account
        let profile = &mut ctx.accounts.profile;
        profile.pfp_token_account = ctx.accounts.pfp_token_account.key();

        Ok(())
    }

}

#[derive(Accounts)]
pub struct CreateProfile<'info> {
    // space: 8 discriminator + size(Domain) = 40 +
    #[account(
    init,
    payer = user,
    seeds = [keychain.key().as_ref(), PROFILE.as_bytes().as_ref()],
    bump,
    space = 8 + Profile::MAX_SIZE,
    )]
    profile: Account<'info, Profile>,
    #[account(mut)]
    user: Signer<'info>,
    system_program: Program <'info, System>,
    keychain_program: Program <'info, Keychain>,

    #[account(owner = keychain_program.key())]
    keychain: Account<'info, CurrentKeyChain>,
}

#[derive(Accounts)]
pub struct SetPfp<'info> {
    // the token account of the pfp nft
    #[account()]
    pfp_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    profile: Account<'info, Profile>,
    #[account(mut)]
    user: Signer<'info>,
    keychain_program: Program <'info, Keychain>,

    #[account(owner = keychain_program.key())]
    keychain: Account<'info, CurrentKeyChain>,

}


// can store any sort of profile info, for now just store the selected pfp nft
#[account]
pub struct Profile {
    // allow for a different username from the one used for the keychain
    username: String,
    // the pfp to use, needs to be in a key on the keychain when set
    pfp_token_account: Pubkey,
    // the keychain this profile is tied to
    keychain: Pubkey,
}

impl Profile {
    // allow for 64 character name
    pub const MAX_SIZE: usize = 64 + 32 + 32;
}

#[error_code]
pub enum ErrorCode {
    #[msg("Username too long. Max 32 characters")]
    NameTooLong,
    #[msg("Signer is not on the keychain")]
    NotOnKeychain,
    #[msg("The owner of this NFT is not on the keychain")]
    OwnerNotOnKeychain,
}

// utility func to check if a particular key is on the given keychain and that it's verified
pub fn check_key(keychain: &Account<CurrentKeyChain>, userkey: Pubkey) -> bool {
    let mut found_key = false;
    for key in &keychain.keys {
        if key.verified && key.key == userkey {
            found_key = true;
        }
    }
    return found_key;
}
