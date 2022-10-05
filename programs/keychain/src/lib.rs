use anchor_lang::prelude::*;

declare_id!("KeyNfJK4cXSjBof8Tg1aEDChUMea4A7wCzLweYFRAoN");

#[program]
pub mod keychain {
    use anchor_lang::AccountsClose;
    use super::*;

    pub fn create_keychain(ctx: Context<CreateKeychain>, username: String, appname: String) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;
        let key = PlayerKey {
            key: *ctx.accounts.user.to_account_info().key,
            verified: true
        };
        // add to the keychain vector
        keychain.keys.push(key);
        keychain.num_keys = 1;

        msg!("created keychain account: {}", ctx.accounts.keychain.key());

        Ok(())
    }

    // user w/existing keychain (and verified key), adds a new (unverified) key
    pub fn add_player_key(ctx: Context<AddPlayerKey>, key: Pubkey) -> Result <()> {
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

    // user confirms a new (unverified) key on a keychain (which then becomes verified)
    pub fn confirm_player_key(ctx: Context<ConfirmPlayerKey>) -> Result <()> {
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

    // remove a key from the keychain
    pub fn remove_player_key(ctx: Context<RemovePlayerKey>, key: Pubkey) -> Result <()> {
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

#[derive(Accounts)]
#[instruction(username: String, appname: String)]
pub struct CreateKeychain<'info> {
    // space: 8 discriminator + 4 (vec) + size(PlayerKey) = 40
    #[account(
        init,
        payer = user,
        seeds = [username.as_bytes().as_ref(), appname.as_bytes().as_ref(), "keychain".as_bytes().as_ref()],
        bump,
        space = 8 + KeyChain::MAX_SIZE
    )]
    pub keychain: Account<'info, KeyChain>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program <'info, System>,
}

// Create a custom struct for us to work with.
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PlayerKey {
    // size = 8 + 32
    pub key: Pubkey,
    pub verified: bool                  // initially false after existing key adds a new one, until the added key verifies
}

#[account]
pub struct KeyChain {
    pub num_keys: u16,
    // Attach a Vector of type ItemStruct to the account.
    pub keys: Vec<PlayerKey>,
}

impl KeyChain {
    // allow up to 3 wallets for now - 2 num_keys + 4 vector + (space(T) * amount)
    pub const MAX_SIZE: usize = 2 + (4 + 40 * 3);
}

// Add the signer who calls the AddGif method to the struct so that we can save it
#[derive(Accounts)]
pub struct AddPlayerKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    pub user: Signer<'info>,
}

// Confirm the signer who calls the AddGif method to the struct so that we can save it
#[derive(Accounts)]
pub struct ConfirmPlayerKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // this needs to be a UserKey on the keychain
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct RemovePlayerKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    pub user: Signer<'info>,
}



#[error_code]
pub enum ErrorCode {
    #[msg("That key is already on your keychain")]
    KeyAlreadyExists,
    #[msg("You are not a valid signer for this keychain")]
    SignerNotInKeychain,
    #[msg("That key doesn't exist on this keychain")]
    KeyNotFound,

}



