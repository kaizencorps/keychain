import * as anchor from "@project-serum/anchor";
import {findListingDomainPda} from "./utils";
const { assert } = require("chai");
const { PublicKey } = anchor.web3;

function randomName() {
  return Math.random().toString(36).substring(2, 5) + Math.random().toString(36).substring(2, 5);
}

const listingDomainName = randomName();
let txid: string;
let programDataAccount;

describe("bazaar", () => {

  // Configure the client to use the local cluster.
  let provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Program client handle.
  const bazaarProg = anchor.workspace.Bazaar;
  const connection = provider.connection;

  const [listingDomainPda] = findListingDomainPda(listingDomainName, bazaarProg.programId);


  it("Creates a listing domain", async () => {

    // find the program's data account
    console.log('bazaar progid: ', bazaarProg.programId.toBase58());
    const bazaarProgramAccount = await connection.getParsedAccountInfo(bazaarProg.programId);
    // @ts-ignore
    programDataAccount = new PublicKey(bazaarProgramAccount.value.data.parsed.info.programData);
    console.log('program data account: ', programDataAccount);


    // now we can create the listing domain as an admin
    txid = await bazaarProg.methods.createListingDomain({name: listingDomainName})
        .accounts({
          upgradeAuthority: bazaarProg.provider.wallet.publicKey,
          program: bazaarProg.programId,
          programData: programDataAccount,
          systemProgram: anchor.web3.SystemProgram.programId,
          listingDomain: listingDomainPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

    console.log(`created listing domain: ${listingDomainName}, txid `, txid);

    let listingDomain = await bazaarProg.account.listingDomain.fetch(listingDomainPda);
    const decodedName = new TextDecoder("utf-8").decode(new Uint8Array(listingDomain.name.filter(char => char !== 0)));

    console.log('decoded listing domain name: ', decodedName);

    assert.equal(decodedName, listingDomainName);

    /*
    const chat = await bazaarProg.account.chatRoom.fetch(chatRoom.publicKey);
    const name = new TextDecoder("utf-8").decode(new Uint8Array(chat.name));

    assert.isTrue(name.startsWith("Test Chat")); // [u8; 280] => trailing zeros.
    assert.lengthOf(chat.messages, 33607);
    assert.strictEqual(chat.head.toNumber(), 0);
    assert.strictEqual(chat.tail.toNumber(), 0);
     */

  });


});
