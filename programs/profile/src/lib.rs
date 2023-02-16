use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use keychain::account::{CurrentKeyChain, KeyChainState};
use keychain::program::Keychain;

use {
    anchor_lang::{
        prelude::*,
        solana_program::{instruction::Instruction, system_program}, InstructionData,
    },
    clockwork_sdk::{
        ID as thread_program_ID,
        self,
        state::{Thread, Trigger, ThreadAccount, ThreadResponse},
        ThreadProgram,
    },
};

declare_id!("ProoXffuU4NJWstjWgFYqauFFatDMG9xHETuRMjKMLt");

const PROFILE: &str = "profile";
pub const SEED_THREAD: &str = "thread";

#[program]
pub mod profile {
    use super::*;

    use anchor_lang::solana_program::{
        program::{invoke},
        system_instruction,
    };

    use anchor_lang::solana_program::program::invoke_signed;

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
        profile.last_automation_execution = 0;

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

    pub fn create_automation(ctx: Context<CreateAutomation>, automation_id: u8) -> Result<()> {

        msg!("creating automation...");

        let hello_ix = Instruction {
            program_id: crate::ID,
            accounts: vec![
                // AccountMeta::new_readonly(authority.key(), false),
                AccountMeta::new(ctx.accounts.profile.key(), false),
                AccountMeta::new(ctx.accounts.thread.key(), true),
            ],
            data: profile::instruction::Hello {}.data()
            // data: clockwork_sdk::utils::anchor_sighash("hello").into(),
        };

        let profile = &ctx.accounts.profile;

        // todo: bump would normally be stored in the profile
        // let thread = Thread::pubkey(ctx.accounts.keychain.key(), "pfp".into());
        let (_, profile_bump) = Pubkey::find_program_address(&[profile.keychain.as_ref(), PROFILE.as_bytes().as_ref()], &crate::ID);

        let seeds = &[
            profile.keychain.as_ref(),
            PROFILE.as_bytes().as_ref(),
            &[profile_bump]
        ];

        // can't get the bump this way since we have to use the clockwork init; so we pass it in as a param
        // let thread_bump = *ctx.bumps.get("thread").unwrap();

        // figure out the bump ourselves, or could pass it in
        // let (_, thread_bump) = Pubkey::find_program_address(&[SEED_THREAD.as_bytes().as_ref(), ctx.accounts.profile.key().as_ref(), automation_id.to_string().as_bytes()],
        //                                                     &ThreadProgram::program_id());

        // let trigger =
        //     Trigger::Account {
        //         address: event.key(),
        //         offset: 0,
        //         size: 8,
        //     };

        // let trigger = Trigger::Cron {
        //     schedule: "*/60 * * * * * *".into(),
        //     skippable: true,
        // };

        let trigger = Trigger::Immediate;

        let clockwork = &ctx.accounts.clockwork;
        let system_program = &ctx.accounts.system_program;

        msg!("creating spi for call to thread program: {}", clockwork.key());

        clockwork_sdk::cpi::thread_create(
            CpiContext::new_with_signer(
                clockwork.to_account_info(),
                clockwork_sdk::cpi::ThreadCreate {
                    authority: ctx.accounts.profile.to_account_info(),
                    payer: ctx.accounts.signer.to_account_info(),
                    system_program: system_program.to_account_info(),
                    thread: ctx.accounts.thread.to_account_info(),
                },
                &[seeds],
                // &[&[SEED_AUTHORITY, &[bump]]],
            ),
            // thread id
            automation_id.to_string().into(),
            // instruction
            hello_ix.into(),
            // trigger
            trigger
        )?;

        // fund the thread a bit
        invoke(
            &system_instruction::transfer(
                ctx.accounts.signer.key,
                ctx.accounts.thread.key,
                10000000 as u64
            ),
            &[
                ctx.accounts.signer.to_account_info().clone(),
                ctx.accounts.thread.to_account_info().clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        Ok(())
    }


    pub fn hello(ctx: Context<HelloWorld>) -> Result<ThreadResponse> {
        let execution_time = Clock::get().unwrap().unix_timestamp;
        msg!(
            "Hello there! The current time is: {}",
            execution_time
        );

        let profile = &mut ctx.accounts.profile;
        profile.last_automation_execution =  execution_time;

        Ok(ThreadResponse::default())
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

#[derive(Accounts)]
pub struct HelloWorld<'info> {

    #[account(mut)]
    pub profile: Account<'info, Profile>,

    #[account(address = hello_thread.pubkey(), signer)]
    pub hello_thread: Account<'info, Thread>,
}

// todo: we can create an automation to trigger when the token account holding the pfp changes, which then sets/unsets the profile.pfp_token_account
//       if the user no longer owns the pfp


#[derive(Accounts)]
#[instruction(automation_id: u8)]
pub struct CreateAutomation<'info> {

    // #[account()]
    // keychain: Account<'info, CurrentKeyChain>,

    #[account(mut)]
    profile: Account<'info, Profile>,

    #[account(mut)]
    pub signer: Signer<'info>,

    // the clockwork thread account, we can use this but we'll use the one below so we can access the bump
    #[account(mut, address = Thread::pubkey(profile.key(), automation_id.to_string().into()))]
    pub thread: SystemAccount<'info>,

    // #[account(address = clockwork_sdk::ID)]
    #[account(address = thread_program_ID)]
    pub clockwork: Program<'info, ThreadProgram>,

    // #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,

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
    last_automation_execution: i64,
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
