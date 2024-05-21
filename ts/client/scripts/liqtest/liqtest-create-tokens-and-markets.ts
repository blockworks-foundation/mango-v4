import { BN, AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  Transaction,
  SystemProgram,
  AddressLookupTableProgram,
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  TransactionInstruction,
  Signer,
} from '@solana/web3.js';
import * as splToken from '@solana/spl-token';
import * as serum from '@project-serum/serum';
import * as obv2 from '@openbook-dex/openbook-v2';
import fs from 'fs';
import { MangoClient } from '../../src/client';
import {
  MANGO_V4_ID,
  OPENBOOK_PROGRAM_ID,
  OPENBOOK_V2_PROGRAM_ID,
} from '../../src/constants';
import { connect } from 'http2';
import { generateSerum3MarketExternalVaultSignerAddress } from '../../src/accounts/serum3';
import { OracleConfigParams } from '../../src';

//
// Script which creates three mints and two serum3 markets relating them
//

const MINT_COUNT = 5;
const SERUM_MARKET_COUNT = 4;
const OBV2_MARKET_COUNT = 4;

function getVaultOwnerAndNonce(
  market: PublicKey,
  programId: PublicKey,
): [PublicKey, BN] {
  const nonce = new BN(0);
  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      const vaultOwner = PublicKey.createProgramAddressSync(
        [market.toBuffer(), nonce.toArrayLike(Buffer, 'le', 8)],
        programId,
      );
      return [vaultOwner, nonce];
    } catch (e) {
      nonce.iaddn(1);
    }
  }
}

async function main(): Promise<void> {
  Error.stackTraceLimit = 10000;

  const options = AnchorProvider.defaultOptions();
  options.commitment = 'processed';
  options.preflightCommitment = 'finalized';
  const connection = new Connection(process.env.CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  // Make mints
  const mints = await Promise.all(
    Array(MINT_COUNT)
      .fill(null)
      .map(() =>
        splToken.createMint(connection, admin, admin.publicKey, null, 6),
      ),
  );

  // Mint some tokens to the admin
  for (const mint of mints) {
    const tokenAccount = await splToken.createAssociatedTokenAccountIdempotent(
      connection,
      admin,
      mint,
      admin.publicKey,
    );
    await splToken.mintTo(connection, admin, mint, tokenAccount, admin, 1e15);
  }
  //const mints = [new PublicKey('5aMD1uEcWnXnptwmyfxmTWHzx3KeMsZ7jmiJAZ3eiAdH'), new PublicKey('FijXcDUkgTiMsghQVpjRDBdUPtkrJfQdfRZkr6zLkdkW'), new PublicKey('3tVDfiFQAAT3rqLNMXUaH2p5X5R4fjz8LYEvFEQ9fDYB')]

  // make markets
  const quoteMint = mints[0];

  // Make serum markets
  const serumMarkets: PublicKey[] = [];
  for (const baseMint of mints.slice(1, 1 + SERUM_MARKET_COUNT)) {
    const feeRateBps = 0.25; // don't think this does anything
    const quoteDustThreshold = 100;
    const baseLotSize = 1000;
    const quoteLotSize = 1; // makes prices be in 1000ths

    const openbookProgramId = OPENBOOK_PROGRAM_ID.devnet;
    const market = Keypair.generate();
    const requestQueue = Keypair.generate();
    const eventQueue = Keypair.generate();
    const bids = Keypair.generate();
    const asks = Keypair.generate();
    const baseVault = Keypair.generate();
    const quoteVault = Keypair.generate();

    const [vaultOwner, vaultSignerNonce] = getVaultOwnerAndNonce(
      market.publicKey,
      openbookProgramId,
    );

    await splToken.createAccount(
      connection,
      admin,
      baseMint,
      vaultOwner,
      baseVault,
    );
    await splToken.createAccount(
      connection,
      admin,
      quoteMint,
      vaultOwner,
      quoteVault,
    );

    const tx = new Transaction();
    tx.add(
      SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: market.publicKey,
        lamports: await connection.getMinimumBalanceForRentExemption(
          serum.Market.getLayout(openbookProgramId).span,
        ),
        space: serum.Market.getLayout(openbookProgramId).span,
        programId: openbookProgramId,
      }),
      SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: requestQueue.publicKey,
        lamports: await connection.getMinimumBalanceForRentExemption(5120 + 12),
        space: 5120 + 12,
        programId: openbookProgramId,
      }),
      SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: eventQueue.publicKey,
        lamports: await connection.getMinimumBalanceForRentExemption(
          262144 + 12,
        ),
        space: 262144 + 12,
        programId: openbookProgramId,
      }),
      SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: bids.publicKey,
        lamports: await connection.getMinimumBalanceForRentExemption(
          65536 + 12,
        ),
        space: 65536 + 12,
        programId: openbookProgramId,
      }),
      SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: asks.publicKey,
        lamports: await connection.getMinimumBalanceForRentExemption(
          65536 + 12,
        ),
        space: 65536 + 12,
        programId: openbookProgramId,
      }),
      serum.DexInstructions.initializeMarket({
        market: market.publicKey,
        requestQueue: requestQueue.publicKey,
        eventQueue: eventQueue.publicKey,
        bids: bids.publicKey,
        asks: asks.publicKey,
        baseVault: baseVault.publicKey,
        quoteVault: quoteVault.publicKey,
        baseMint,
        quoteMint,
        baseLotSize: new BN(baseLotSize),
        quoteLotSize: new BN(quoteLotSize),
        feeRateBps,
        vaultSignerNonce,
        quoteDustThreshold: new BN(quoteDustThreshold),
        programId: openbookProgramId,
        authority: undefined,
      }),
    );

    await sendAndConfirmTransaction(connection, tx, [
      admin,
      market,
      requestQueue,
      eventQueue,
      bids,
      asks,
    ]);

    serumMarkets.push(market.publicKey);
  }

  // Make openbook-v2 markets
  const obv2Markets: PublicKey[] = [];
  for (const baseMint of mints.slice(1, 1 + OBV2_MARKET_COUNT)) {
    const baseLotSize = 1000;
    const quoteLotSize = 1; // makes prices be in 1000ths

    const openbookv2ProgramId = OPENBOOK_V2_PROGRAM_ID.devnet;
    const market = Keypair.generate();
    const requestQueue = Keypair.generate();
    const eventQueue = Keypair.generate();
    const bids = Keypair.generate();
    const asks = Keypair.generate();
    const baseVault = Keypair.generate();
    const quoteVault = Keypair.generate();

    const [vaultOwner, vaultSignerNonce] = getVaultOwnerAndNonce(
      market.publicKey,
      openbookv2ProgramId,
    );

    await splToken.createAccount(
      connection,
      admin,
      baseMint,
      vaultOwner,
      baseVault,
    );
    await splToken.createAccount(
      connection,
      admin,
      quoteMint,
      vaultOwner,
      quoteVault,
    );
    const provider = new AnchorProvider(
      connection,
      new Wallet(admin),
      AnchorProvider.defaultOptions(),
    );
    const obv2client = new obv2.OpenBookV2Client(provider, openbookv2ProgramId);
    const createMarket = await obv2client.createMarketIx(
      admin.publicKey,
      '',
      quoteMint,
      baseMint,
      new BN(quoteLotSize),
      new BN(baseLotSize),
      new BN(0),
      new BN(0),
      new BN(0),
      null,
      null,
      null,
      null,
      null,
      new OracleConfigParams(),
      market,
      admin.publicKey,
    );

    // const tx = new Transaction();
    // tx.add(
    //   SystemProgram.createAccount({
    //     fromPubkey: admin.publicKey,
    //     newAccountPubkey: market.publicKey,
    //     lamports: await connection.getMinimumBalanceForRentExemption(
    //       serum.Market.getLayout(openbookv2ProgramId).span,
    //     ),
    //     space: serum.Market.getLayout(openbookv2ProgramId).span,
    //     programId: openbookv2ProgramId,
    //   }),
    //   SystemProgram.createAccount({
    //     fromPubkey: admin.publicKey,
    //     newAccountPubkey: requestQueue.publicKey,
    //     lamports: await connection.getMinimumBalanceForRentExemption(5120 + 12),
    //     space: 5120 + 12,
    //     programId: openbookv2ProgramId,
    //   }),
    //   SystemProgram.createAccount({
    //     fromPubkey: admin.publicKey,
    //     newAccountPubkey: eventQueue.publicKey,
    //     lamports: await connection.getMinimumBalanceForRentExemption(
    //       262144 + 12,
    //     ),
    //     space: 262144 + 12,
    //     programId: openbookv2ProgramId,
    //   }),
    //   SystemProgram.createAccount({
    //     fromPubkey: admin.publicKey,
    //     newAccountPubkey: bids.publicKey,
    //     lamports: await connection.getMinimumBalanceForRentExemption(
    //       65536 + 12,
    //     ),
    //     space: 65536 + 12,
    //     programId: openbookv2ProgramId,
    //   }),
    //   SystemProgram.createAccount({
    //     fromPubkey: admin.publicKey,
    //     newAccountPubkey: asks.publicKey,
    //     lamports: await connection.getMinimumBalanceForRentExemption(
    //       65536 + 12,
    //     ),
    //     space: 65536 + 12,
    //     programId: openbookv2ProgramId,
    //   }),
    // );
    // await sendAndConfirmTransaction(connection, tx, [
    //   admin,
    //   market,
    //   requestQueue,
    //   eventQueue,
    //   bids,
    //   asks,
    // ]);
    //
    const tx2 = new Transaction();
    const signers: Signer[] = [admin];
    for (const x of createMarket[0]) {
      tx2.add(x);
    }
    for (const x of createMarket[1]) {
      signers.push(x);
    }

    await sendAndConfirmTransaction(connection, tx2, signers);
    obv2Markets.push(market.publicKey);
  }

  console.log(
    "MINTS='[" + mints.map((pk) => '"' + pk.toBase58() + '"').join(',') + "]'",
  );
  console.log(
    "SERUM_MARKETS='[" +
      serumMarkets.map((pk) => '"' + pk.toBase58() + '"').join(',') +
      "]'",
  );
  console.log(
    "OBV2_MARKETS='[" +
      obv2Markets.map((pk) => '"' + pk.toBase58() + '"').join(',') +
      "]'",
  );

  process.exit();
}

main();
