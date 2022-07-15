import * as anchor from '@project-serum/anchor';
import {
  Program,
  Spl,
  SplToken,
  BN,
  AnchorProvider,
} from '@project-serum/anchor';
import { MangoV4 } from '../target/types/mango_v4';
import * as spl from '@solana/spl-token';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import * as assert from 'assert';
import { PublicKey, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import { MangoClient, Group, I80F48 } from '../ts/client/src/index';


import {
  // ASSOCIATED_TOKEN_PROGRAM_ID,
  // Token,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import { Id } from '../ts/client/src/ids';

enum MINTS {
  USDC = 'USDC',
  BTC = 'BTC',
}

async function createMints(
  program: anchor.Program<MangoV4>,
  payer: anchor.web3.Keypair,
  admin,
) {
  let mintsMap: Partial<Record<keyof typeof MINTS, spl.Token>>;
  let mints: spl.Token[] = [];
  for (let i = 0; i < 2; i++) {
    mints.push(
      await spl.Token.createMint(
        program.provider.connection,
        payer,
        admin.publicKey,
        admin.publicKey,
        6,
        spl.TOKEN_PROGRAM_ID,
      ),
    );
  }
  mintsMap = {
    USDC: mints[0],
    BTC: mints[1],
  };

  return mintsMap;
}

async function createUsers(
  mintsMap: Partial<Record<keyof typeof MINTS, spl.Token>>,
  payer: anchor.web3.Keypair,
) {
  let users: { key: anchor.web3.Keypair; tokenAccounts: spl.AccountInfo[] }[] =
    [];
  for (let i = 0; i < 4; i++) {
    let user = anchor.web3.Keypair.generate();

    let tokenAccounts: spl.AccountInfo[] = [];
    for (let mintKey in mintsMap) {
      let mint: spl.Token = mintsMap[mintKey];
      let tokenAccount = await mint.getOrCreateAssociatedAccountInfo(
        user.publicKey,
      );
      await mint.mintTo(tokenAccount.address, payer, [], 1_000_000_000_000_000);
      tokenAccounts.push(tokenAccount);
    }
    console.log('created user ' + i);
    users.push({ key: user, tokenAccounts: tokenAccounts });
  }

  return users;
}

describe('mango-v4', () => {

  let programId = new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD');
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  let providerWallet = provider.wallet;
  let providerPayer = (providerWallet as NodeWallet).payer;

  const program = anchor.workspace.MangoV4 as Program<MangoV4>;
  let users: { key: anchor.web3.Keypair; tokenAccounts: spl.AccountInfo[] }[] =
    [];

  let mintsMap: Partial<Record<keyof typeof MINTS, spl.Token>>;

  it('Is initialized!', async () => {
    console.log(providerWallet.publicKey.toString());

    mintsMap = await createMints(program, providerPayer, providerWallet);
    users = await createUsers(mintsMap, providerPayer);

    console.log(users.map((e) => e.key.publicKey.toString()));
  });

  it('test_basic', async () => {


    let groupNum = 0;
    let testing = 0;
    let insuranceMintPk = mintsMap['USDC']!.publicKey;
    const adminPk = providerWallet.publicKey;

    await program.methods
      .groupCreate(groupNum, testing ? 1 : 0)
      .accounts({
        admin: adminPk,
        payer: adminPk,
        insuranceMint: insuranceMintPk,
      })
      .rpc();

    let groups = await program.account.group.all();
    // let group = await client.getGroupForAdmin(adminPk, groupNum);
    let group = groups[0];
    
    let mint = 'BTC'
    let mintPk = mintsMap[mint]!.publicKey;
    let oraclePk = anchor.web3.Keypair.generate().publicKey;
    let oracleConfFilter = 0.1;
    let tokenIndex = 1;
    let name = mint;
    let adjustmentFactor = 0.01;
    let util0 = 0.4;
    let rate0 =0.07;
    let util1 = 0.8;
    let rate1 = 0.9;
    let maxRate = 0.88;
    let loanFeeRate = 0.0005;
    let loanOriginationFeeRate = 1.5;
    let maintAssetWeight = 0.8;
    let initAssetWeight = 0.6;
    let maintLiabWeight = 1.2;
    let initLiabWeight = 1.4;
    let liquidationFee = 0.02;

    await program.methods
    .tokenRegister(
      tokenIndex,
      new BN(0),
      name,
      {
        confFilter: {
          val: I80F48.fromNumber(oracleConfFilter).getData(),
        },
      } as any, // future: nested custom types dont typecheck, fix if possible?
      { adjustmentFactor, util0, rate0, util1, rate1, maxRate },
      loanFeeRate,
      loanOriginationFeeRate,
      maintAssetWeight,
      initAssetWeight,
      maintLiabWeight,
      initLiabWeight,
      liquidationFee,
    )
    .accounts({
      group: group.publicKey,
      admin: providerWallet.publicKey,
      mint: mintPk,
      oracle: oraclePk,
      payer: providerWallet.publicKey,
      rent: SYSVAR_RENT_PUBKEY,
    })
    .rpc();

    mint = 'USDC'
    mintPk = mintsMap[mint]!.publicKey;
    oraclePk = anchor.web3.Keypair.generate().publicKey;
    tokenIndex = 0;
    name = mint;

    await program.methods
    .tokenRegister(
      tokenIndex,
      new BN(0),
      name,
      {
        confFilter: {
          val: I80F48.fromNumber(oracleConfFilter).getData(),
        },
      } as any, // future: nested custom types dont typecheck, fix if possible?
      { adjustmentFactor, util0, rate0, util1, rate1, maxRate },
      loanFeeRate,
      loanOriginationFeeRate,
      maintAssetWeight,
      initAssetWeight,
      maintLiabWeight,
      initLiabWeight,
      liquidationFee,
    )
    .accounts({
      group: group.publicKey,
      admin: providerWallet.publicKey,
      mint: mintPk,
      oracle: oraclePk,
      payer: providerWallet.publicKey,
      rent: SYSVAR_RENT_PUBKEY,
    })
    .rpc();

    let mintInfos: { name: string; publicKey: string }[] = [];
    let banks: { name: string; publicKey: string }[] = [];
    let stubOracles: { name: string; publicKey: string }[] = [];
    for (let mint in mintsMap) {
      let mintPk = await mintsMap[mint].publicKey!;
      let [mintInfoPk] = await PublicKey.findProgramAddress(
        [group.publicKey.toBuffer(), Buffer.from('MintInfo', 'utf-8'), mintPk.toBuffer()],
        programId,
      );

      let mintInfo = await program.account.mintInfo.fetch(mintInfoPk);
      mintInfos.push({ 'name': mint, 'publicKey': mintInfoPk.toString() })
      banks.push({ 'name': mint, 'publicKey': mintInfo.banks[0].toString() })
      stubOracles.push({ 'name': mint, 'publicKey': mintInfo.oracle.toString() })
    }


    let ids = new Id('localnet', 'localnet', group.publicKey.toString(), '', programId.toString(), banks, stubOracles, mintInfos, [], []);

    console.log(ids)

    // Doesn't make sense to pass 'devnet' here - but it's only used when interacting with Serum I believe
    const client = await MangoClient.connect(
      provider,
      'localnet',
      programId,
      ids
    );


    // let group = await client.getGroupForAdmin(adminPk, groupNum);

    //   const mangoAccount = await client.getOrCreateMangoAccount(
    //     group,
    //     users[0].key.publicKey,
    //     0,
    //     'my_mango_account',
    //   );

    //   await client.tokenDeposit(group, mangoAccount, 'USDC', 50);

    //   // // The mango client alwaysb creates a mango account with the owner set up the admin?
    //   // let accountNum = 0;
    //   // await program.methods
    //   // .accountCreate(accountNum, 'my_mango_account')
    //   // .accounts({
    //   //   group: group.publicKey,
    //   //   owner: users[0].key.publicKey,
    //   //   payer: providerPayer.publicKey,
    //   // })
    //   // .rpc();

    //   // const mangoAccount: MangoAccount = await client.getMangoAccountForOwner(group, users[0].key.publicKey)[0]




    //   let amount = 50;
    //   let account = mangoAccount;
    //   // let token_account = users[0].tokenAccounts[0];
    //   // let token_authority = providerPayer;
    //   let bankIndex = 0;

    //   // let tokenAccount = program.account.
    //   // let token_account: TokenAccount = account_loader.load(&self.token_account).await.unwrap();


    //   let mintPk = await mintsMap['BTC']!.publicKey!;
    //   const [mintInfoPk] = await PublicKey.findProgramAddress(
    //     [group.publicKey.toBuffer(), Buffer.from('MintInfo', 'utf-8'), mintPk.toBuffer()],
    //     programId,
    //   );

    //   let mintInfo = await program.account.mintInfo.fetch(mintInfoPk);

    //   // const [address] = await PublicKey.findProgramAddress(
    //   //   [owner.toBuffer(), programId.toBuffer(), mint.toBuffer()],
    //   //   associatedTokenProgramId,
    //   // );

    //   // group: group.publicKey,
    //   // account: mangoAccount.publicKey,
    //   // owner: users[0].key.publicKey,
    //   // bank: mintInfo.banks[bankIndex] mint_info.banks[self.bank_index],
    //   // vault: mint_info.vaults[bankIndex],
    //   // token_account: users[0].tokenAccounts['BTC'],
    //   // token_program: new PublicKey(TOKEN_PROGRAM_ID),



    //   console.log(mintInfo);

    //   await program.methods
    //   // .tokenDeposit(toNativeDecimals(amount, bank.mintDecimals))
    //   // Math.trunc(amount * Math.pow(10, decimals)))
    //   .tokenDeposit(new BN(amount))
    //   .accounts({
    //     group: group.publicKey,
    //     account: mangoAccount.publicKey,
    //     bank: mintInfo.banks[bankIndex],
    //     vault: mintInfo.vaults[bankIndex],
    //     tokenAccount: users[0].tokenAccounts['BTC'],
    //     tokenAuthority: providerWallet.publicKey
    //   })
    //   .remainingAccounts(
    //     healthRemainingAccounts.map(
    //       (pk) =>
    //         ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
    //     ),
    //   )
    //   .rpc();
    //   // .preInstructions(preInstructions)
    //   // .postInstructions(postInstructions)
    //   // .signers(additionalSigners)



    // //   const [referrerMemoryPk] = await PublicKey.findProgramAddress(
    // //     [mangoAccountPk.toBytes(), ,
    // //     this.programId,
    // //   );


    // //   let mint_info = Pubkey::find_program_address(
    // //     &[
    // //         account.group.as_ref(),
    // //         b"MintInfo".as_ref(),
    // //         token_account.mint.as_ref(),
    // //     ],
    // //     &program_id,
    // // )
    // // .0;


    // //   await program.methods
    // //   .tokenDeposit(amount)
    // //   .accounts({
    // //     group: group.publicKey,
    // //     account: mangoAccount.publicKey,
    // //     bank: bank.publicKey,
    // //     vault: bank.vault,
    // //     tokenAccount: wrappedSolAccount?.publicKey ?? tokenAccountPk,
    // //     tokenAuthority: (this.program.provider as AnchorProvider).wallet
    // //       .publicKey,
    // //   })

    // //   // console.log(mangoAccount)

    //   // console.log(`Depositing...50 USDC`);
    //   await client.tokenDeposit(group, mangoAccount, 'USDC', 50);
    //   // await mangoAccount.reload(client, group);

    // //   console.log(`Depositing...0.0005 BTC`);
    // //   await client.tokenDeposit(group, mangoAccount, 'BTC', 0.0005);
    // //   // await mangoAccount.reload(client, group);

    // //   // await client.tokenDeposit(group, mangoAccount, 'USDC', 50);
    // //   // await mangoAccount.reload(client, group);
  });
});
