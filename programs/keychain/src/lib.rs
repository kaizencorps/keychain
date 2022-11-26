use anchor_lang::prelude::*;

declare_id!("KeyNfJK4cXSjBof8Tg1aEDChUMea4A7wCzLweYFRAoN");

const KEYCHAIN: &str = "keychain";

#[program]
pub mod keychain {
    use anchor_lang::AccountsClose;
    use super::*;

    pub fn create_domain(ctx: Context<CreateDomain>, name: String) -> Result <()> {
        ctx.accounts.domain.authority = *ctx.accounts.authority.key;
        ctx.accounts.domain.bump = *ctx.bumps.get("domain").unwrap();
        // let domain_name = name.as_bytes();
        // let mut name = [0u8; 32];
        // name[..domain_name.len()].copy_from_slice(domain_name);

        // todo: check name length <= 32

        ctx.accounts.domain.name = name;
        msg!("created domain account: {}", ctx.accounts.domain.key());
        Ok(())
    }

    // if done by admin, then the authority needs to be the Domain's authority
    pub fn create_keychain(ctx: Context<CreateKeychain>, _playername: String) -> Result <()> {
        msg!("creating keychain...");
        let mut admin = false;

        // if the signer is the same as the domain authority, then this is a domain admin
        if ctx.accounts.authority.key() == ctx.accounts.domain.authority.key() {
            admin = true;
        } else {
            // otherwise, the wallet being added needs to be the signer
            require!(ctx.accounts.authority.key() == *ctx.accounts.wallet.key, ErrorCode::NotSigner);
        }

        let keychain = &mut ctx.accounts.keychain;
        let key = PlayerKey {
            key: *ctx.accounts.wallet.to_account_info().key,
            verified: true
        };

        // keychain.domain = ctx.accounts.domain.key();
        // add to the keychain vector
        keychain.keys.push(key);
        keychain.num_keys = 1;

        // first, verify the keychain_key derivation matches the given one (since they need to be unique)
        /*
        let (keychain_key_address, _keychain_key_bump) =
            Pubkey::find_program_address(&[ctx.accounts.keychain_key.key().as_ref(), ctx.accounts.domain.name.as_bytes(), KEYCHAIN.as_bytes()], ctx.program_id);

        if ctx.accounts.keychain_key.key() != keychain_key_address {
            msg!("derived keychain key account (pointer) doesn't match: {}", keychain_key_address);
            return Err(ErrorCode::IncorrectKeyAddress.into());
        }
         */

        // now set up the pointer account
        let keychain_key = &mut ctx.accounts.keychain_key;
        keychain_key.key = ctx.accounts.wallet.key();
        keychain_key.keychain = ctx.accounts.keychain.key();

        msg!("created keychain account: {}", ctx.accounts.keychain.key());
        msg!("created keychain key account: {}", ctx.accounts.keychain_key.key());

        Ok(())
    }

    // user w/existing keychain (and verified key), adds a new (unverified) key
    pub fn add_key(ctx: Context<AddKey>, key: Pubkey) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;

        let mut found_signer = false;
        let mut found_existing = false;
        let signer = *ctx.accounts.user.to_account_info().key;

        // todo: check that the signer is in the keychain & whether this key already exists
        for user_key in &keychain.keys {
            if user_key.verified && user_key.key == signer {
                found_signer = true;
            }
            if key == user_key.key {
                found_existing = true;
            }
        }

        require!(found_signer, ErrorCode::SignerNotInKeychain);
        require!(!found_existing, ErrorCode::KeyAlreadyExists);

        // todo: check keychain size limit ..?

        // Build the struct.
        let key = PlayerKey {
            key: key,
            verified: false,
        };

        // Add it to the keychain.
        keychain.keys.push(key);
        keychain.num_keys += 1;

        Ok(())
    }

    // user verifies a new (unverified) key on a keychain (which then becomes verified)
    pub fn verify_key(ctx: Context<VerifyKey>) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;

        let mut found_signer = false;

        let signer = *ctx.accounts.user.to_account_info().key;

        for user_key in &mut *keychain.keys {
            if user_key.key == signer {
                found_signer = true;
                if user_key.verified {
                    msg!("key already verified: {}", user_key.key);
                } else {
                    msg!("key now verified: {}", user_key.key);
                    user_key.verified = true;
                }
            }
        }

        require!(found_signer, ErrorCode::SignerNotInKeychain);

        Ok(())
    }

    // remove a key from a keychain
    pub fn remove_key(ctx: Context<RemoveKey>, key: Pubkey) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;

        let mut found_signer = false;
        let mut remove_index = usize::MAX;

        let signer = *ctx.accounts.user.to_account_info().key;

        for (index, user_key) in keychain.keys.iter().enumerate() {
        // for &mut &user_key in &keychain.keys {
            if user_key.key == signer {
                found_signer = true;
            }
            if key == user_key.key {
                remove_index = index;
            }
        }

        require!(found_signer, ErrorCode::SignerNotInKeychain);
        require!(remove_index != usize::MAX, ErrorCode::KeyNotFound);

        keychain.keys.swap_remove(remove_index);

        // decrement
        keychain.num_keys -= 1;

        if keychain.num_keys == 0 {
            // close the keychain account if this is the last key
            msg!("No more keys. Destroying keychain");
            keychain.close(ctx.accounts.user.to_account_info());
        }

        Ok(())
    }
}

// create a domain account for admin usage

#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateDomain<'info> {
    // space: 8 discriminator + size(Domain) = 40
    #[account(
        init,
        payer = authority,
        seeds = [name.as_bytes().as_ref(), "keychain".as_bytes().as_ref()],
        bump,
        space = 8 + Domain::MAX_SIZE,
    )]
    domain: Account<'info, Domain>,
    #[account(mut)]
    authority: Signer<'info>,
    system_program: Program <'info, System>,
}


#[derive(Accounts)]
#[instruction(playername: String)]
pub struct CreateKeychain<'info> {
    // space: 8 discriminator + KeyChain::MAX_SIZE
    #[account(
        init,
        payer = authority,
        seeds = [playername.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
        bump,
        space = 8 + KeyChain::MAX_SIZE
    )]
    keychain: Account<'info, KeyChain>,
    #[account(
        init,
        payer = authority,
        seeds = [wallet.key().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
        bump,
        space = 8 + (32 * 2)
    )]
    // the first key on this keychain
    keychain_key: Account<'info, KeyChainKey>,
    #[account()]
    domain: Account<'info, Domain>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    wallet: AccountInfo<'info>,
    #[account(mut)]
    authority: Signer<'info>,
    system_program: Program <'info, System>,
}

// Create a custom struct for us to work with.
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PlayerKey {
    // size = 1 + 32 = 33
    pub key: Pubkey,
    pub verified: bool                  // initially false after existing key adds a new one, until the added key verifies
}

// todo: might wanna store the "display" version of the playername since the account should be derived from a "normalized" version of the playername
#[account]
pub struct KeyChain {
    num_keys: u16,
    domain: Pubkey,
    // Attach a Vector of type ItemStruct to the account.
    keys: Vec<PlayerKey>,
}

impl KeyChain {
    // allow up to 3 wallets for now - 2 num_keys + 4 vector + (space(T) * amount)
    pub const MAX_SIZE: usize = 2 + 32 + (4 + 5 * 33);
}

// a "pointer" account which points to the keychain it's attached to. this is to prevent keys from being added ot multiple keychains
#[account]
pub struct KeyChainKey {
    // pointer to the keychain this key is attached to
    keychain: Pubkey,
    // the key/wallet this key holds - matches the one in the keychain
    key: Pubkey,
}

// domains are needed for admin functions
#[account]
pub struct Domain {
    // max size = 32
    name: String,
    authority: Pubkey,
    bump: u8
}

impl Domain {
    // allow up to 3 wallets for now - 2 num_keys + 4 vector + (space(T) * amount)
    pub const MAX_SIZE: usize = 32 + 32 + 1;
}

#[derive(Accounts)]
pub struct AddKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    pub user: Signer<'info>,
}

// Confirm the signer who calls the AddGif method to the struct so that we can save it
#[derive(Accounts)]
pub struct VerifyKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // this needs to be a UserKey on the keychain
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct RemoveKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    pub user: Signer<'info>,
}


#[error_code]
pub enum ErrorCode {
    #[msg("The given key account is not the correct PDA for the given address")]
    IncorrectKeyAddress,
    #[msg("That key is already on your keychain")]
    KeyAlreadyExists,
    #[msg("You are not a valid signer for this keychain")]
    SignerNotInKeychain,
    #[msg("That key doesn't exist on this keychain")]
    KeyNotFound,
    #[msg("Signer is not a domain admin")]
    NotDomainAdmin,
    #[msg("Can only add wallet of signer")]
    NotSigner,

}



