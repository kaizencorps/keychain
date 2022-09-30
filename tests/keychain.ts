import * as anchor from "@project-serum/anchor";
import {AnchorProvider, Program, Wallet} from "@project-serum/anchor";
import { Keychain } from "../target/types/keychain";
import * as assert from "assert";
const { SystemProgram } = anchor.web3;


describe("keychain", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.Keychain as Program<Keychain>;
  // const program = anchor.workspace.Keychain;

    const appName = 'domination';
    const username = 'silostack';

    // our keychain account
    const [keychainPda, keychainPdaBump] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            Buffer.from(anchor.utils.bytes.utf8.encode(username)),
            Buffer.from(anchor.utils.bytes.utf8.encode(appName)),
            Buffer.from(anchor.utils.bytes.utf8.encode("keychain")),
        ],
        program.programId
    );

    console.log(`keychain pda: ${keychainPda.toBase58()}`);

    // the 2nd key to put on the keychain
    const key2 = anchor.web3.Keypair.generate();

    // wallet 2 provider (simulate separate connection)
    const wallet2Provider = new AnchorProvider(
        provider.connection,
        new Wallet(key2),
        {}
    );
    const wallet2Program = new Program(program.idl, program.programId, wallet2Provider);

    it('sets up the test', async () => {
        // airdrop some sol to the 2nd key's wallet
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(key2.publicKey, anchor.web3.LAMPORTS_PER_SOL * 0.5),
            "confirmed"
        );
    });

    it("Creates the keychain", async () => {
      let tx = await program.rpc.createKeychain(username, appName, {
              accounts: {
                  keychain: keychainPda,
                  user: provider.wallet.publicKey,
                  systemProgram: SystemProgram.programId,
              }
          }
      );

      console.log(`created keychain tx: ${tx}`);

      // now let's fetch that fuckin thing
      let keychain = await program.account.keyChain.fetch(keychainPda);
      console.log('keychain: ', keychain);
      console.log('-- num keys: ', keychain.numKeys);
      console.log('-- keys: ', keychain.keys);
      console.log('-- key 1: ', keychain.keys[0].key.toBase58());

  });

  it("Adds a key to the keychain and verifies it", async () => {
      await program.rpc.addUserKey(key2.publicKey, {
          accounts: {
              keychain: keychainPda,
              user: provider.wallet.publicKey,
          }
      });

      let keychain = await program.account.keyChain.fetch(keychainPda);

      // not verified yet
      assert.ok(!keychain.keys[1].verified, 'added key should be verified');

      // try to add again and we fail (already there)
      program.rpc.addUserKey(key2.publicKey, {
          accounts: {
              keychain: keychainPda,
              user: provider.wallet.publicKey,
          }
      }).then(() => {
          assert.fail("shoudln't be able to add same key again");
      }).catch(() => {
          // expected
      });

      await wallet2Program.rpc.confirmUserKey({
          accounts: {
              keychain: keychainPda,
              user: key2.publicKey,
          }
      });

      // now the 2nd key is verified
      keychain = await program.account.keyChain.fetch(keychainPda);

      // verified!
      assert.ok(keychain.keys[1].verified, 'added key should be verified');

  });

  it("removes a key from the keychain", async () => {
      // we'll remove the original key (simulate it potentially being a custodial key)
      await wallet2Program.rpc.removeUserKey(provider.wallet.publicKey, {
          accounts: {
              keychain: keychainPda,
              user: key2.publicKey,
          }
      });

      let keychain = await program.account.keyChain.fetch(keychainPda);
      assert.ok(keychain.numKeys == 1, 'should only be 1 key on the keychain');
      assert.ok(keychain.keys.length == 1, 'should only be 1 key on the keychain');
  });

  it("closes an empty keychain account", async () => {
     // now wallet2 will remove itself (key2) from the keychain (the only key)
     await wallet2Program.rpc.removeUserKey(key2.publicKey, {
         accounts: {
            keychain: keychainPda,
            user: key2.publicKey,
         }
      });

      // account shouldn't exist now
      program.account.keyChain.fetch(keychainPda).then(() => {
          assert.fail("account shouldn't exist");
      }).catch(() => {
          // expected
      });
  });

});
