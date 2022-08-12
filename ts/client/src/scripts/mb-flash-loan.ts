import { Jupiter } from '@jup-ag/core';
import { AnchorProvider, Wallet } from '@project-serum/anchor';
import {
  AccountMeta,
  Connection,
  Keypair,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  TransactionInstruction,
} from '@solana/web3.js';
import BN from 'bn.js';
import fs from 'fs';
import { QUOTE_DECIMALS } from '../accounts/bank';
import { MangoClient } from '../index';
import { getAssociatedTokenAddress } from '../utils';

const MB_CLUSTER_URL =
  process.env.MB_CLUSTER_URL ||
  'https://mango.rpcpool.com/946ef7337da3f5b8d3e4a34e7f88';
const MB_PAYER_KEYPAIR =
  process.env.MB_PAYER_KEYPAIR ||
  '/Users/tylershipe/.config/solana/deploy.json';

//
// example script which shows usage of flash loan ix using a jupiter swap
//
// NOTE: we assume that ATA for source and target already exist for wallet
async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL, options);

  // load user key
  const user = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connectForGroupName(
    userProvider,
    'mainnet-beta.microwavedcola',
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // load admin key
  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, 0);
  console.log(`Found group ${group.publicKey.toBase58()}`);
  console.log(`start btc bank ${group.banksMap.get('BTC').toString()}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(`start balance \n${mangoAccount.toString(group)}`);

  //
  // flash loan 3
  //
  if (true) {
    // source of swap
    const sourceBank = group.banksMap.get('USDC');
    // target of swap
    const targetBank = group.banksMap.get('BTC');
    // 0.2$, at 1BTC=20,000$, 0.2$=0.00001BTC
    const sourceAmount = 2 * Math.pow(10, QUOTE_DECIMALS - 1);

    console.log(`Flash loaning ${sourceBank.name} to ${targetBank.name}`);

    // jupiter route
    const jupiter = await Jupiter.load({
      connection: client.program.provider.connection,
      cluster: 'mainnet-beta',
      user: mangoAccount.owner, // or public key
      // platformFeeAndAccounts:  NO_PLATFORM_FEE,
      routeCacheDuration: 10_000, // Will not refetch data on computeRoutes for up to 10 seconds
    });
    const routes = await jupiter.computeRoutes({
      inputMint: sourceBank.mint, // Mint address of the input token
      outputMint: targetBank.mint, // Mint address of the output token
      inputAmount: sourceAmount, // raw input amount of tokens
      slippage: 5, // The slippage in % terms
      forceFetch: false, // false is the default value => will use cache if not older than routeCacheDuration
    });
    const routesInfosWithoutRaydium = routes.routesInfos.filter((r) => {
      if (r.marketInfos.length > 1) {
        for (const mkt of r.marketInfos) {
          if (mkt.amm.label === 'Raydium' || mkt.amm.label === 'Serum')
            return false;
        }
      }
      return true;
    });

    // loop until we manage first successful swap
    let res;
    let i = 0;
    while (true) {
      const instructions: TransactionInstruction[] = [];

      // select a route and fetch+build its tx
      const selectedRoute = routesInfosWithoutRaydium[i];
      const { transactions } = await jupiter.exchange({
        routeInfo: selectedRoute,
      });

      const { setupTransaction, swapTransaction } = transactions;
      for (const ix of swapTransaction.instructions) {
        if (
          ix.programId.toBase58() ===
          'JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo'
        ) {
          instructions.push(ix);
        }
      }

      // run jup setup in a separate tx, ideally this should be packed before flashLoanBegin in same tx,
      // but it increases chance of flash loan tx to exceed tx size limit
      if (setupTransaction) {
        await this.program.provider.sendAndConfirm(setupTransaction);
      }

      // flash loan start ix - takes a loan for source token,
      // flash loan end ix - returns increase in all token account's amounts to respective vaults,
      const healthRemainingAccounts =
        client.buildFixedAccountRetrieverHealthAccounts(
          group,
          mangoAccount,
          [sourceBank, targetBank], // we would be taking a sol loan potentially
        );
      // 1. build flash loan end ix
      const flashLoadnEndIx = await client.program.methods
        .flashLoanEnd(true)
        .accounts({
          account: mangoAccount.publicKey,
          owner: (client.program.provider as AnchorProvider).wallet.publicKey,
        })
        .remainingAccounts([
          ...healthRemainingAccounts.map(
            (pk) =>
              ({
                pubkey: pk,
                isWritable: false,
                isSigner: false,
              } as AccountMeta),
          ),
          {
            pubkey: sourceBank.vault,
            isWritable: true,
            isSigner: false,
          } as AccountMeta,
          {
            pubkey: targetBank.vault,
            isWritable: true,
            isSigner: false,
          } as AccountMeta,
          {
            pubkey: await getAssociatedTokenAddress(
              sourceBank.mint,
              mangoAccount.owner,
            ),
            isWritable: true, // increase in this address amount is transferred back to the sourceBank.vault above in this case whatever is residual of source bank loan
            isSigner: false,
          } as AccountMeta,
          {
            pubkey: await getAssociatedTokenAddress(
              targetBank.mint,
              mangoAccount.owner,
            ),
            isWritable: true, // increase in this address amount is transferred back to the targetBank.vault above in this case whatever is result of swap
            isSigner: false,
          } as AccountMeta,
          {
            pubkey: group.publicKey,
            isWritable: false,
            isSigner: false,
          } as AccountMeta,
        ])
        .instruction();
      instructions.push(flashLoadnEndIx);
      // 2. build flash loan start ix, add end ix as a post ix
      try {
        res = await client.program.methods
          .flashLoanBegin([
            new BN(sourceAmount),
            new BN(
              0,
            ) /* we don't care about borrowing the target amount, this is just a dummy */,
          ])
          .accounts({
            // for observing ixs in the entire tx,
            // e.g. apart from flash loan start and end no other ix should target mango v4 program
            // e.g. forbid FlashLoanBegin been called from CPI
            instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          })
          .remainingAccounts([
            {
              pubkey: sourceBank.publicKey,
              isWritable: true, // metadata for flash loan is updated
              isSigner: false,
            } as AccountMeta,
            {
              pubkey: targetBank.publicKey,
              isWritable: true, // this is a dummy, its just done so that we match flash loan start and end ix
              isSigner: false,
            } as AccountMeta,
            {
              pubkey: sourceBank.vault,
              isWritable: true,
              isSigner: false,
            } as AccountMeta,
            {
              pubkey: targetBank.vault,
              isWritable: true, // this is a dummy, its just done so that we match flash loan start and end ix
              isSigner: false,
            } as AccountMeta,
            {
              pubkey: await getAssociatedTokenAddress(
                sourceBank.mint,
                mangoAccount.owner,
              ),
              isWritable: true, // token transfer i.e. loan to a desired token account e.g. user's ATA when using a route made for a specific user
              isSigner: false,
            } as AccountMeta,
            {
              pubkey: await getAssociatedTokenAddress(
                targetBank.mint,
                mangoAccount.owner,
              ),
              isWritable: false, // this is a dummy, its just done so that we match flash loan start and end ix
              isSigner: false,
            } as AccountMeta,
            {
              pubkey: group.publicKey,
              isWritable: false,
              isSigner: false,
            } as AccountMeta,
          ])
          .postInstructions(instructions)
          .rpc();

        // break when success
        break;
      } catch (error) {
        console.log(error);
        if (
          (error.toString() as string).includes('Transaction too large:') ||
          (error.toString() as string).includes(
            'encoding overruns Uint8Array',
          ) ||
          (error.toString() as string).includes(
            'The value of "offset" is out of range. It must be >= 0 and <= 1231. Received 1232',
          ) ||
          (error.toString() as string).includes(
            'The value of "value" is out of range. It must be >= 0 and <= 255. Received',
          ) ||
          i > 10
        ) {
          console.log(`route ${i} was bad, trying next one...`);
          i++;
        } else {
          throw error; // let others bubble up
        }
      }
    }

    console.log(`success tx - https://explorer.solana.com/tx/${res}`);

    group.reloadBanks(client);
    console.log(`end btc bank ${group.banksMap.get('BTC').toString()}`);

    await mangoAccount.reload(client, group);
    console.log(`end balance \n${mangoAccount.toString(group)}`);
  }
}

main();
