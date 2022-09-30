use anchor_lang::prelude::*;


declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod keychain {
    use anchor_lang::AccountsClose;
    use super::*;

    pub fn create_keychain(ctx: Context<CreateKeychain>, username: String, appname: String) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;
        let key = UserKey {
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
    pub fn add_user_key(ctx: Context<AddUserKey>, key: Pubkey) -> Result <()> {
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
        let key = UserKey {
            key: key,
            verified: false,
        };

        // Add it to the keychain.
        keychain.keys.push(key);
        keychain.num_keys += 1;

        Ok(())
    }

    // user confirms a new (unverified) key on a keychain (which then becomes verified)
    pub fn confirm_user_key(ctx: Context<ConfirmUserKey>) -> Result <()> {
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
    pub fn remove_user_key(ctx: Context<RemoveUserKey>, key: Pubkey) -> Result <()> {
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

    // The function now accepts a gif_link param from the user. We also reference the user from the Context
    /*
    pub fn add_gif(ctx: Context<AddGif>, gif_link: String) -> Result <()> {
        let base_account = &mut ctx.accounts.base_account;
        let user = &mut ctx.accounts.user;

        // Build the struct.
        let item = ItemStruct {
            gif_link: gif_link.to_string(),
            user_address: *user.to_account_info().key,
        };

        // Add it to the gif_list vector.
        base_account.gif_list.push(item);
        base_account.total_gifs += 1;
        Ok(())
    }

     */
}

#[derive(Accounts)]
#[instruction(username: String, appname: String)]
pub struct CreateKeychain<'info> {
    #[account(
        init,
        payer = user,
        seeds = [username.as_bytes().as_ref(), appname.as_bytes().as_ref(), "keychain".as_bytes().as_ref()],
        bump,
        space = 8 + 2 + 3 * 32
    )]
    pub keychain: Account<'info, KeyChain>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program <'info, System>,
}

// Create a custom struct for us to work with.
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UserKey {
    pub key: Pubkey,
    pub verified: bool
}

#[account]
pub struct KeyChain {
    pub num_keys: u16,
    // Attach a Vector of type ItemStruct to the account.
    pub keys: Vec<UserKey>,
}

// Add the signer who calls the AddGif method to the struct so that we can save it
#[derive(Accounts)]
pub struct AddUserKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    pub user: Signer<'info>,
}

// Confirm the signer who calls the AddGif method to the struct so that we can save it
#[derive(Accounts)]
pub struct ConfirmUserKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // this needs to be a UserKey on the keychain
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct RemoveUserKey<'info> {
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



