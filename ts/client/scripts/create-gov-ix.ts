import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { BN } from '@project-serum/anchor';
import { serializeInstructionToBase64 } from '@solana/spl-governance';
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';
import fs from 'fs';
import { PerpMarketIndex } from '../src/accounts/perp';
import { Builder } from '../src/builder';
import { MangoClient } from '../src/client';
import {
  NullPerpEditParams,
  TrueIxGateParams,
  buildIxGate,
} from '../src/clientIxParamBuilder';
import { MANGO_V4_ID } from '../src/constants';
import { bpsToDecimal, percentageToDecimal, toNative } from '../src/utils';

const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR } = process.env;

const CLIENT_USER = MB_PAYER_KEYPAIR;
const GROUP_PK = '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';

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

async function buildClient(): Promise<MangoClient> {
  const clientKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(CLIENT_USER!, 'utf-8'))),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const clientWallet = new Wallet(clientKeypair);
  const clientProvider = new AnchorProvider(connection, clientWallet, options);

  return await MangoClient.connect(
    clientProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'get-program-accounts',
    },
  );
}

async function groupEdit(): Promise<void> {
  const client = await buildClient();
  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const ix = await client.program.methods
    .groupEdit(
      null, // admin
      null, // fastListingAdmin
      null, // securityAdmin
      null, // testing
      null, // version
      null, // depositLimitQuote
      null, // feesPayWithMngo
      null, // feesMngoBonusRate
      null, // feesSwapMangoAccount
      6, // feesMngoTokenIndex
      null, // feesExpiryInterval
      5, // allowedFastListingsPerInterval
    )
    .accounts({
      group: group.publicKey,
      admin: group.admin,
    })
    .instruction();
  console.log(serializeInstructionToBase64(ix));
}

// async function tokenRegister(): Promise<void> {
//   const client = await buildClient();

//   const group = await client.getGroup(new PublicKey(GROUP_PK));

//   const ix = await client.program.methods
//     .tokenRegister(
//       8 as TokenIndex,
//       'wBTC (Portal)',
//       defaultOracleConfig,
//       defaultInterestRate,
//       bpsToDecimal(50),
//       bpsToDecimal(5),
//       0.9,
//       0.8,
//       1.1,
//       1.2,
//       percentageToDecimal(5),
//       60 * 60,
//       0.06,
//       0.0003,
//       percentageToDecimal(20),
//       new BN(24 * 60 * 60),
//       new BN(toNative(50000, 6).toNumber()),
//       5_000_000_000,
//       5_000_000_000,
//       0,
//       0.0005,
//       0.0005,
//       0.0005,
//     )
//     .accounts({
//       group: group.publicKey,
//       admin: group.admin,
//       mint: new PublicKey('3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh'),
//       oracle: new PublicKey('GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'),
//       payer: (client.program.provider as AnchorProvider).wallet.publicKey,
//       rent: SYSVAR_RENT_PUBKEY,
//     })
//     .instruction();

//   // const coder = new BorshCoder(IDL);
//   // console.log(coder.instruction.decode(ix.data));

//   console.log(await serializeInstructionToBase64(ix));
// }

// async function tokenEdit(): Promise<void> {
//   const client = await buildClient();

//   const group = await client.getGroup(new PublicKey(GROUP_PK));

//   const params = Builder(NullTokenEditParams)
//     .borrowWeightScaleStartQuote(new BN(toNative(100000, 6)).toNumber())
//     .depositWeightScaleStartQuote(new BN(toNative(100000, 6)).toNumber())
//     .build();
//   const ix = await client.program.methods
//     .tokenEdit(
//       params.oracle,
//       params.oracleConfig,
//       params.groupInsuranceFund,
//       params.interestRateParams,
//       params.loanFeeRate,
//       params.loanOriginationFeeRate,
//       params.maintAssetWeight,
//       params.initAssetWeight,
//       params.maintLiabWeight,
//       params.initLiabWeight,
//       params.liquidationFee,
//       params.stablePriceDelayIntervalSeconds,
//       params.stablePriceDelayGrowthLimit,
//       params.stablePriceGrowthLimit,
//       params.minVaultToDepositsRatio,
//       params.netBorrowLimitPerWindowQuote !== null
//         ? new BN(params.netBorrowLimitPerWindowQuote)
//         : null,
//       params.netBorrowLimitWindowSizeTs !== null
//         ? new BN(params.netBorrowLimitWindowSizeTs)
//         : null,
//       params.borrowWeightScaleStartQuote,
//       params.depositWeightScaleStartQuote,
//       params.resetStablePrice ?? false,
//       params.resetNetBorrowLimit ?? false,
//       params.reduceOnly,
//       params.name,
//       params.forceClose,
//       params.tokenConditionalSwapTakerFeeRate,
//       params.tokenConditionalSwapMakerFeeRate,
//       params.flashLoanSwapFeeRate,
//     )
//     .accounts({
//       group: group.publicKey,
//       oracle: new PublicKey('GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'),
//       admin: group.admin,
//       mintInfo: new PublicKey('59rgC1pa45EziDPyFgJgE7gbv7Dd7VaGmd2D93i1dtFk'),
//     })
//     .remainingAccounts([
//       {
//         pubkey: new PublicKey('8gabXzwdPn5TvtuQvysh3CxVbjfNY3TZd5XEG5qnueUm'),
//         isWritable: true,
//         isSigner: false,
//       } as AccountMeta,
//     ])
//     .instruction();
//   console.log(serializeInstructionToBase64(ix));
// }

// async function serum3Register(): Promise<void> {
//   const client = await buildClient();

//   const group = await client.getGroup(new PublicKey(GROUP_PK));

//   const ix = await client.program.methods
//     .serum3RegisterMarket(3, 'ETH (Portal)/USDC',)
//     .accounts({
//       group: group.publicKey,
//       admin: group.admin,
//       serumProgram: OPENBOOK_PROGRAM_ID['mainnet-beta'],
//       serumMarketExternal: new PublicKey(
//         'BbJgE7HZMaDp5NTYvRh5jZSkQPVDTU8ubPFtpogUkEj4',
//       ),
//       baseBank: group.getFirstBankByMint(
//         new PublicKey('7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs'),
//       ).publicKey,
//       quoteBank: group.getFirstBankByMint(
//         new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
//       ).publicKey,
//       payer: (client.program.provider as AnchorProvider).wallet.publicKey,
//     })
//     .instruction();

//   console.log(serializeInstructionToBase64(ix));
// }

async function perpCreate(): Promise<void> {
  const client = await buildClient();

  const bids = new Keypair();
  const asks = new Keypair();
  const eventQueue = new Keypair();

  const bookSideSize = (client.program as any)._coder.accounts.size(
    (client.program.account.bookSide as any)._idlAccount,
  );
  const eventQueueSize = (client.program as any)._coder.accounts.size(
    (client.program.account.eventQueue as any)._idlAccount,
  );

  const group = await client.getGroup(new PublicKey(GROUP_PK));

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

async function perpEdit(): Promise<void> {
  const client = await buildClient();
  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const perpMarket = group.getPerpMarketByMarketIndex(0 as PerpMarketIndex);
  const params = Builder(NullPerpEditParams)
    .positivePnlLiquidationFee(bpsToDecimal(250))
    .build();
  const ix = await client.program.methods
    .perpEditMarket(
      params.oracle,
      params.oracleConfig,
      params.baseDecimals,
      params.maintBaseAssetWeight,
      params.initBaseAssetWeight,
      params.maintBaseLiabWeight,
      params.initBaseLiabWeight,
      params.maintOverallAssetWeight,
      params.initOverallAssetWeight,
      params.baseLiquidationFee,
      params.makerFee,
      params.takerFee,
      params.minFunding,
      params.maxFunding,
      params.impactQuantity !== null ? new BN(params.impactQuantity) : null,
      params.groupInsuranceFund,
      params.feePenalty,
      params.settleFeeFlat,
      params.settleFeeAmountThreshold,
      params.settleFeeFractionLowHealth,
      params.stablePriceDelayIntervalSeconds,
      params.stablePriceDelayGrowthLimit,
      params.stablePriceGrowthLimit,
      params.settlePnlLimitFactor,
      params.settlePnlLimitWindowSize !== null
        ? new BN(params.settlePnlLimitWindowSize)
        : null,
      params.reduceOnly,
      params.resetStablePrice ?? false,
      params.positivePnlLiquidationFee,
      params.name,
      params.forceClose,
    )
    .accounts({
      group: group.publicKey,
      oracle: params.oracle ?? perpMarket.oracle,
      admin: group.admin,
      perpMarket: perpMarket.publicKey,
    })
    .instruction();
  console.log(serializeInstructionToBase64(ix));
}

async function ixDisable(): Promise<void> {
  const client = await buildClient();

  const group = await client.getGroup(new PublicKey(GROUP_PK));

  const ixGateParams = TrueIxGateParams;
  ixGateParams.HealthRegion = false;
  const ix = await client.program.methods
    .ixGateSet(buildIxGate(ixGateParams))
    .accounts({
      group: group.publicKey,
      admin: group.securityAdmin,
    })
    .instruction();

  console.log(await serializeInstructionToBase64(ix));
}

async function createMangoAccount(): Promise<void> {
  const client = await buildClient();

  const group = await client.getGroup(new PublicKey(GROUP_PK));

  const ix = await client.program.methods
    .accountCreate(0, 8, 8, 4, 32, 'Mango DAO 0')
    .accounts({
      group: group.publicKey,
      owner: new PublicKey('5tgfd6XgwiXB9otEnzFpXK11m7Q7yZUaAJzWK4oT5UGF'),
      payer: new PublicKey('5tgfd6XgwiXB9otEnzFpXK11m7Q7yZUaAJzWK4oT5UGF'),
    })
    .instruction();

  console.log(await serializeInstructionToBase64(ix));
}

async function idlResize(): Promise<void> {
  // anchor constant for all idl-specific instructions
  const idlIxBytes = [0x40, 0xf4, 0xbc, 0x78, 0xa7, 0xe9, 0x69, 0x0a];
  const idlIxNum = 6; // resize
  const newSize = new BN(19000);
  const ix = new TransactionInstruction({
    keys: [
      {
        pubkey: new PublicKey('3foqXduY5PabCn6LjNrLo3waNf3Hy6vQgqavoVUCsUN9'), // idl account
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: new PublicKey('FP4PxqHTVzeG2c6eZd7974F9WvKUSdBeduUK3rjYyvBw'), // authority
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: new PublicKey('11111111111111111111111111111111'), // system program
        isSigner: false,
        isWritable: false,
      },
    ],
    programId: MANGO_V4_ID['mainnet-beta'],
    data: Buffer.from(idlIxBytes.concat([idlIxNum], newSize.toArray('le', 8))),
  });

  console.log(await serializeInstructionToBase64(ix));
}

async function idlSetAuthority(): Promise<void> {
  // anchor constant for all idl-specific instructions
  const idlIxBytes = [0x40, 0xf4, 0xbc, 0x78, 0xa7, 0xe9, 0x69, 0x0a];
  const idlIxNum = 4; // setAuthority
  const newAuthority = new PublicKey(
    '8SSLjXBEVk9nesbhi9UMCA32uijbVBUqWoKPPQPTekzt',
  );
  const ix = new TransactionInstruction({
    keys: [
      {
        pubkey: new PublicKey('3foqXduY5PabCn6LjNrLo3waNf3Hy6vQgqavoVUCsUN9'), // idl account
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: new PublicKey('FP4PxqHTVzeG2c6eZd7974F9WvKUSdBeduUK3rjYyvBw'), // authority
        isSigner: true,
        isWritable: true,
      },
    ],
    programId: MANGO_V4_ID['mainnet-beta'],
    data: Buffer.concat([
      Buffer.from(idlIxBytes.concat([idlIxNum])),
      newAuthority.toBuffer(),
    ]),
  });

  console.log(await serializeInstructionToBase64(ix));
}

async function main(): Promise<void> {
  try {
    await groupEdit();
    // await tokenRegister();
    // await tokenEdit();
    // await perpCreate();
    // await perpEdit();
    // await serum3Register();
    // await ixDisable();
    // await createMangoAccount();
    // await idlResize();
    // await idlSetAuthority();
  } catch (error) {
    console.log(error);
  }
}

try {
  main();
} catch (error) {
  console.log(error);
}
