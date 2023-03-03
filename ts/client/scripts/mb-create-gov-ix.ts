import { AnchorProvider } from '@coral-xyz/anchor';
import { BN, Wallet } from '@project-serum/anchor';
import { serializeInstructionToBase64 } from '@solana/spl-governance';
import {
  AccountMeta,
  Connection,
  Keypair,
  PublicKey,
  SYSVAR_RENT_PUBKEY,
  SystemProgram,
} from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../src/accounts/bank';
import { Builder } from '../src/builder';
import { MangoClient } from '../src/client';
import { NullTokenEditParams } from '../src/clientIxParamBuilder';
import { MANGO_V4_ID, OPENBOOK_PROGRAM_ID } from '../src/constants';
import { bpsToDecimal, percentageToDecimal, toNative } from '../src/utils';

const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR, MB_PAYER3_KEYPAIR } = process.env;

const defaultOracleConfig = {
  confFilter: 0.1,
  maxStalenessSlots: 120,
};

const defaultInterestRate = {
  adjustmentFactor: 0.004,
  util0: 0.7,
  rate0: 0.1,
  util1: 0.85,
  rate1: 0.2,
  maxRate: 2.0,
};

async function buildAdminClient(): Promise<[MangoClient, Keypair, Keypair]> {
  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER3_KEYPAIR!, 'utf-8'))),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);

  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'get-program-accounts',
    },
  );

  const creator = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );

  return [client, admin, creator];
}

async function tokenRegister(): Promise<void> {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  const ix = await client.program.methods
    .tokenRegister(
      8 as TokenIndex,
      'wBTC (Portal)',
      defaultOracleConfig,
      defaultInterestRate,
      bpsToDecimal(50),
      bpsToDecimal(5),
      0.9,
      0.8,
      1.1,
      1.2,
      percentageToDecimal(5),
      percentageToDecimal(20),
      new BN(24 * 60 * 60),
      new BN(toNative(50000, 6).toNumber()),
    )
    .accounts({
      group: group.publicKey,
      admin: group.admin,
      mint: new PublicKey('3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh'),
      oracle: new PublicKey('GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'),
      payer: (client.program.provider as AnchorProvider).wallet.publicKey,
      rent: SYSVAR_RENT_PUBKEY,
    })
    .instruction();

  // const coder = new BorshCoder(IDL);
  // console.log(coder.instruction.decode(ix.data));

  console.log(await serializeInstructionToBase64(ix));
}

async function tokenEdit(): Promise<void> {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  const params = Builder(NullTokenEditParams)
    .oracle(new PublicKey('GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'))
    .borrowWeightScaleStartQuote(new BN(toNative(100000, 6)).toNumber())
    .depositWeightScaleStartQuote(new BN(toNative(100000, 6)).toNumber())
    .build();
  const ix = await client.program.methods
    .tokenEdit(
      params.oracle,
      params.oracleConfig,
      params.groupInsuranceFund,
      params.interestRateParams,
      params.loanFeeRate,
      params.loanOriginationFeeRate,
      params.maintAssetWeight,
      params.initAssetWeight,
      params.maintLiabWeight,
      params.initLiabWeight,
      params.liquidationFee,
      params.stablePriceDelayIntervalSeconds,
      params.stablePriceDelayGrowthLimit,
      params.stablePriceGrowthLimit,
      params.minVaultToDepositsRatio,
      params.netBorrowLimitPerWindowQuote !== null
        ? new BN(params.netBorrowLimitPerWindowQuote)
        : null,
      params.netBorrowLimitWindowSizeTs !== null
        ? new BN(params.netBorrowLimitWindowSizeTs)
        : null,
      params.borrowWeightScaleStartQuote,
      params.depositWeightScaleStartQuote,
      params.resetStablePrice ?? false,
      params.resetNetBorrowLimit ?? false,
      params.reduceOnly,
    )
    .accounts({
      group: group.publicKey,
      oracle: new PublicKey('GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'),
      admin: group.admin,
      mintInfo: new PublicKey('59rgC1pa45EziDPyFgJgE7gbv7Dd7VaGmd2D93i1dtFk'),
    })
    .remainingAccounts([
      {
        pubkey: new PublicKey('8gabXzwdPn5TvtuQvysh3CxVbjfNY3TZd5XEG5qnueUm'),
        isWritable: true,
        isSigner: false,
      } as AccountMeta,
    ])
    .instruction();
  console.log(serializeInstructionToBase64(ix));
}

async function serum3Register(): Promise<void> {
  const result = await buildAdminClient();
  const client = result[0];

  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  const ix = await client.program.methods
    .serum3RegisterMarket(3, 'ETH (Portal)/USDC')
    .accounts({
      group: group.publicKey,
      admin: group.admin,
      serumProgram: OPENBOOK_PROGRAM_ID['mainnet-beta'],
      serumMarketExternal: new PublicKey(
        'BbJgE7HZMaDp5NTYvRh5jZSkQPVDTU8ubPFtpogUkEj4',
      ),
      baseBank: group.getFirstBankByMint(
        new PublicKey('7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs'),
      ).publicKey,
      quoteBank: group.getFirstBankByMint(
        new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
      ).publicKey,
      payer: (client.program.provider as AnchorProvider).wallet.publicKey,
    })
    .instruction();

  console.log(serializeInstructionToBase64(ix));
}

async function perpCreate(): Promise<void> {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const bids = new Keypair();
  const asks = new Keypair();
  const eventQueue = new Keypair();

  const bookSideSize = (client.program as any)._coder.accounts.size(
    (client.program.account.bookSide as any)._idlAccount,
  );
  const eventQueueSize = (client.program as any)._coder.accounts.size(
    (client.program.account.eventQueue as any)._idlAccount,
  );

  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  const ix = await client.program.methods
    .perpCreateMarket(
      2,
      'SOL-PERP',
      defaultOracleConfig,
      9,
      new BN(100),
      new BN(10000000),
      0.9,
      0.8,
      1.1,
      1.2,
      0,
      0,
      percentageToDecimal(5),
      bpsToDecimal(-1),
      bpsToDecimal(4),
      percentageToDecimal(-5),
      percentageToDecimal(5),
      new BN(100),
      false,
      bpsToDecimal(0.5),
      toNative(0.0001, 6).toNumber(),
      toNative(100, 6).toNumber(),
      percentageToDecimal(1),
      0,
      1,
      new BN(60 * 60),
      percentageToDecimal(10),
    )
    .accounts({
      group: group.publicKey,
      admin: group.admin,
      oracle: new PublicKey('H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG'),
      bids: bids.publicKey,
      asks: asks.publicKey,
      eventQueue: eventQueue.publicKey,
      payer: (client.program.provider as AnchorProvider).wallet.publicKey,
    })
    .preInstructions([
      // book sides
      SystemProgram.createAccount({
        programId: client.program.programId,
        space: bookSideSize,
        lamports:
          await client.program.provider.connection.getMinimumBalanceForRentExemption(
            bookSideSize,
          ),
        fromPubkey: (client.program.provider as AnchorProvider).wallet
          .publicKey,
        newAccountPubkey: bids.publicKey,
      }),
      SystemProgram.createAccount({
        programId: client.program.programId,
        space: bookSideSize,
        lamports:
          await client.program.provider.connection.getMinimumBalanceForRentExemption(
            bookSideSize,
          ),
        fromPubkey: (client.program.provider as AnchorProvider).wallet
          .publicKey,
        newAccountPubkey: asks.publicKey,
      }),
      // event queue
      SystemProgram.createAccount({
        programId: client.program.programId,
        space: eventQueueSize,
        lamports:
          await client.program.provider.connection.getMinimumBalanceForRentExemption(
            eventQueueSize,
          ),
        fromPubkey: (client.program.provider as AnchorProvider).wallet
          .publicKey,
        newAccountPubkey: eventQueue.publicKey,
      }),
    ])
    .signers([bids, asks, eventQueue])
    .instruction();
  console.log(serializeInstructionToBase64(ix));
}

async function main(): Promise<void> {
  try {
    // await tokenRegister();
    // await tokenEdit();
    // await perpCreate();
    await serum3Register();
  } catch (error) {
    console.log(error);
  }
}

try {
  main();
} catch (error) {
  console.log(error);
}
