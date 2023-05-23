use anchor_lang::prelude::*;

// keychain v1
// declare_id!("KeyNfJK4cXSjBof8Tg1aEDChUMea4A7wCzLweYFRAoN");

// keychain v2 - grizzlython
// declare_id!("Key3oJGUxKaddvMRAKbyYVbE8Pf3ycrH8hyZxa7tVCo");

declare_id!("keyKitXGRWbPhF7RkMhNk47CqcFWPqAhMuMjVwapS3K");

// declare_id!("6uZ6f5k49o76Dtczsc9neRMk5foNzJ3CuwSm6QKCVp6T");

pub mod constant;
pub mod error;
pub mod context;
pub mod account;
mod util;

// todo: use realloc to enable any number of keys instead of limiting to 5

use constant::*;
use error::*;
use context::*;
use account::*;
use util::*;

#[program]
pub mod keychain {
    use anchor_lang::{AccountsClose};
    use super::*;

    use anchor_lang::solana_program::{
        program::{invoke},
        system_instruction,
    };

    pub fn create_domain(ctx: Context<CreateDomain>, name: String, key_cost: u64) -> Result <()> {

        // check name length <= 32
        require!(name.as_bytes().len() <= 32, KeychainError::NameTooLong);

        let is_valid_name = is_valid_name(&name);
        require!(is_valid_name, KeychainError::InvalidName);

        let domain_state = &mut ctx.accounts.domain_state;
        domain_state.version = CURRENT_DOMAIN_VERSION;
        domain_state.domain = ctx.accounts.domain.key();

        let domain = &mut ctx.accounts.domain;
        domain.name = name;
        domain.authority = *ctx.accounts.authority.key;
        domain.key_cost = key_cost;
        domain.treasury = *ctx.accounts.treasury.key;
        domain.bump = *ctx.bumps.get("domain").unwrap();
        domain.keychain_action_threshold = DEFAULT_DOMAIN_KEYCHAIN_ACTION_THRESHOLD;

        msg!("created domain account: {}", ctx.accounts.domain.key());
        Ok(())
    }

    // todo: make this callable by the domain admin. currently this is a super-admin function

    // just for closing the domain account
    pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {

        let remaining_lamports = ctx.accounts.account.lamports();

        msg!("transferring all lamports out of account (to close) {}: {}", ctx.accounts.account.key(), remaining_lamports);

        // transfer sol: https://solanacookbook.com/references/programs.html#how-to-transfer-sol-in-a-program

        // Debit from_account and credit to_account
        **ctx.accounts.account.try_borrow_mut_lamports()? -= remaining_lamports;
        **ctx.accounts.authority.try_borrow_mut_lamports()? += remaining_lamports;

        Ok(())
    }

    // if done by admin, then the authority needs to be the Domain's authority
    pub fn create_keychain(ctx: Context<CreateKeychain>, keychain_name: String) -> Result <()> {

        // we only reserve 32 bytes for the name
        require!(keychain_name.as_bytes().len() <= 32, KeychainError::NameTooLong);
        require!(keychain_name.len() >= 3, KeychainError::NameTooShort);

        let is_valid_name = is_valid_name(&keychain_name);
        require!(is_valid_name, KeychainError::InvalidName);

        // if the signer is the same as the domain authority, then this is a domain admin
        // for now, don't allow this. this is for when a project wants to pre-allocate or create keychains on behalf of the user
        /*
        let mut admin = false;
        if ctx.accounts.authority.key() == ctx.accounts.domain.authority.key() {
            admin = true;
        }
        */

        // the wallet being added needs to be the signer (ignore admin scenario)
        require!(ctx.accounts.authority.key() == *ctx.accounts.wallet.key, KeychainError::NotSigner);

        let keychain_state = &mut ctx.accounts.keychain_state;
        keychain_state.keychain_version = CURRENT_KEYCHAIN_VERSION;
        keychain_state.keychain = ctx.accounts.keychain.key();
        keychain_state.action_threshold = ctx.accounts.domain.keychain_action_threshold;

        let key = UserKey {
            key: *ctx.accounts.wallet.to_account_info().key,
        };

        let keychain = &mut ctx.accounts.keychain;
        keychain.name = keychain_name;
        keychain.num_keys = 1;
        keychain.domain = ctx.accounts.domain.name.clone();
        keychain.bump = *ctx.bumps.get("keychain").unwrap();
        keychain.keys = vec![key];

        // now set up the pointer/map account
        let keychain_key = &mut ctx.accounts.keychain_key;
        keychain_key.key = ctx.accounts.wallet.key();
        keychain_key.keychain = ctx.accounts.keychain.key();

        msg!("created keychain account: {}", ctx.accounts.keychain.key());
        msg!("created key account: {}", ctx.accounts.keychain_key.key());

        Ok(())
    }

    // for testing upgrade mechanism
    pub fn create_keychain_v1(ctx: Context<CreateKeychainV1>, keychain_name: String) -> Result <()> {
        require!(keychain_name.as_bytes().len() <= 32, KeychainError::NameTooLong);

        let keychain_state = &mut ctx.accounts.keychain_state;

        // set to older version
        keychain_state.keychain_version = CURRENT_KEYCHAIN_VERSION - 1;
        keychain_state.keychain = ctx.accounts.keychain.key();

        let key = UserKey {
            key: *ctx.accounts.wallet.to_account_info().key,
        };

        let keychain = &mut ctx.accounts.keychain;
        keychain.num_keys = 1;
        keychain.domain = ctx.accounts.domain.name.clone();
        keychain.keys = vec![key];

        // now set up the pointer/map account
        let keychain_key = &mut ctx.accounts.key;
        keychain_key.key = ctx.accounts.wallet.key();
        keychain_key.keychain = ctx.accounts.keychain.key();

        msg!("created keychain account: {}", ctx.accounts.keychain.key());
        msg!("created key account: {}", ctx.accounts.key.key());
        Ok(())
    }

    // user w/existing keychain (and verified key), adds a new (unverified) key
    pub fn add_key(ctx: Context<AddKey>, key: Pubkey) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;

        let signer = *ctx.accounts.authority.to_account_info().key;

        require!(!keychain.has_key(&key), KeychainError::KeyAlreadyExists);
        require!(usize::from(keychain.num_keys) < MAX_KEYS, KeychainError::MaxKeys);

        // check that there isn't already a pending action
        let keychain_state = &mut ctx.accounts.keychain_state;
        require!(keychain_state.pending_action.is_none(), KeychainError::PendingActionExists);

        // signer automatically casts vote to approve
        let mut pending_action = PendingKeyChainAction::new(KeyChainActionType::AddKey, key);
        let authority_index = keychain.index_of(&signer).unwrap() as u8;
        pending_action.votes.set_index(authority_index);

        keychain_state.pending_action = Some(pending_action);

        // don't even bother checking the threshold cause let's not ever allow just 1 vote to add a key

        // todo: MIGHT wanna add the key account as an optional to mae sure it doesn't exist yet: https://solana.stackexchange.com/questions/3745/anchors-init-if-constraint-for-the-optional-initialization-of-accounts

        Ok(())
    }

    // vote = true means confirm & vote = false means reject
    pub fn vote_pending_action(ctx: Context<VotePendingAction>, vote: bool) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;
        let signer = *ctx.accounts.authority.to_account_info().key;

        // a single rejection will cancel the pending action
        if !vote {
            // clear the pending action
            ctx.accounts.keychain_state.pending_action = None;
        } else {
            set_vote(keychain, &mut ctx.accounts.keychain_state, &signer, vote);

            let action_threshold = ctx.accounts.keychain_state.action_threshold;
            let pending_action = ctx.accounts.keychain_state.pending_action.as_mut().unwrap();

            if (action_threshold > 0 && pending_action.votes.count_set() >= action_threshold) ||
                (u16::from(pending_action.votes.count_set()) == keychain.num_keys) {

                // perform the pending action
                match pending_action.action_type {
                    KeyChainActionType::AddKey => {
                        // make sure the key has been verified - this makes sure the keychain_key account exists
                        require!(pending_action.verified, KeychainError::KeyNotVerified);
                        // add the key
                        keychain.add_key(pending_action.key);
                    },
                    KeyChainActionType::RemoveKey => {
                        // remove the key - in this case we need to have been passed in the keychain_key account
                        require!(ctx.accounts.keychain_key.is_some(), KeychainError::MissingKeyAccount);
                        keychain.remove_key(pending_action.key);

                        // close the keychain_key account - send lamports back to the signer
                        let keychain_key = ctx.accounts.keychain_key.as_mut().unwrap();
                        keychain_key.close(ctx.accounts.authority.to_account_info())?;
                    },
                }

                // clear the pending action
                ctx.accounts.keychain_state.pending_action = None;
            }
        }

        Ok(())
    }

    // only called when pending action = addkey
    // user verifies a new (unverified) key on a keychain - potentially becomes linked but based on votes
    pub fn verify_key(ctx: Context<VerifyKey>) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;
        let domain = &ctx.accounts.domain;
        let signer = *ctx.accounts.authority.to_account_info().key;

        // check that the payer can pay for this
        if ctx.accounts.authority.lamports() < domain.key_cost {
            return Err(KeychainError::NotEnoughSol.into());
        }

        // pay for this key - transfer sol to treasury
        invoke(
            &system_instruction::transfer(
                ctx.accounts.authority.key,
                &domain.treasury,
                domain.key_cost,
            ),
            &[
                ctx.accounts.authority.to_account_info().clone(),
                ctx.accounts.treasury.clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        // set up the pointer/map account
        let keychain_key = &mut ctx.accounts.keychain_key;
        keychain_key.key = ctx.accounts.authority.key();
        keychain_key.keychain = keychain.key();

        let action_threshold = ctx.accounts.keychain_state.action_threshold;
        let pending_action = ctx.accounts.keychain_state.pending_action.as_mut().unwrap();

        // either we've hit the threshold or all keys have voted
        if (action_threshold > 0 && pending_action.count_votes() >= action_threshold) ||
            (u16::from(pending_action.count_votes()) == keychain.num_keys) {

            // we've reached the threshold - remove the pending action
            let keychain_state = &mut ctx.accounts.keychain_state;
            // clear pending action
            keychain_state.pending_action = None;

            // Add it to the keychain.
            keychain.add_key(signer);
        } else {
            // then we haven't reached the threshold yet - but make sure we've set the verified
            pending_action.verified = true;
        }

        Ok(())
    }

    // remove a key from a keychain
    pub fn remove_key(ctx: Context<RemoveKey>, key: Pubkey) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;
        let keychain_state = &mut ctx.accounts.keychain_state;
        let signer = *ctx.accounts.authority.to_account_info().key;

        // if this is the only linked key, then we close the whole keychain
        if keychain.num_keys == 1 {
            msg!("Closing keychain: {}", keychain.key());
            // close the keychain
            keychain.close(ctx.accounts.authority.to_account_info())?;
            keychain_state.close(ctx.accounts.authority.to_account_info())?;
            let keychain_key = &mut ctx.accounts.keychain_key;
            keychain_key.close(ctx.accounts.authority.to_account_info())?;

        } else {
            // votes
            let mut pending_action = PendingKeyChainAction::new(KeyChainActionType::RemoveKey, key);
            let authority_index = keychain.index_of(&signer).unwrap() as u8;
            pending_action.vote(authority_index, true);
            keychain_state.pending_action = Some(pending_action);
        }

        Ok(())
    }
}





