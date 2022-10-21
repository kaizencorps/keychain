# Overview

Created at the Solana Hacker House in Bogota, Keychain is a way to link multiple wallets to a single on-chain account. 

Keychain was created to solve [Domination's](https://domination.gg) delegated login problem: our players want to play
Domination with their mobile phones/wallets, but keep their NFTs safely on their Ledger/desktop wallets. However, there
are several more potential use cases that we've identified (and also plan to utilize for Domination), such as:

- proving write-access to multiple wallets (for example, for Metaplex's proposed royalties solution: https://metaplex.notion.site/Royalties-and-the-Future-of-Creator-Monetization-Public-5fdcd1b163084b9b87b2e90090129d22)
- player profiles
- wallet-agnostic reputation system
- custodial (and transferable) wallet/account management

Additionally, we plan on Keychain being a key component in the Kaizen Corps ecosystem, and working with
other components that we develop such as [Bazaar](https://github.com/kaizencorps/bazaar).

### Demo: https://youtu.be/gRxQT4asCzY?t=352 (it works but we had technical issues lol)

## How it Works

Keychain currently works as a 3-step process:

- A user creates a Keychain account with a given wallet. This can be tied to a username or a "primary wallet," either 
of which is used to derive the PDA for the account. The given wallet is then added to the keychain as a verified address/wallet.
- The user can then add a new wallet (a key) to the Keychain account, which is initially unverified.
- The user can then confirm their ownership of the added address by calling the 'confirm key' method with the added wallet.

At this point, the Keychain account has 2 "keys," both of which are now verified, and the user has proven ownership of
both wallets. Any verified key on the keychain can add a new key, as well as remove itself from the keychain.

## Domains

Keychain is being built to be accessible by any app/project (in addition to being available to individuals). Domains are 
a way to segregate Keychain accounts (e.g. "domination" might be one domain and "kaizen-corps" might be another. We'll 
be introducing "admin" functions for apps/projects to create their own domains for administration. An open global domain 
will be created for anyone to use.

# Code

Usage can be deduced from the tests located in keychain.ts

# Status

Keychain is not yet deployed on mainnet, as it's currently under heavy development, though we plan on deploying a version 
within days (of this writing). A working prototype (without domains implementation) is already deployed on devnet: KeyNfJK4cXSjBof8Tg1aEDChUMea4A7wCzLweYFRAoN



