import {
  AccountMeta,
  PublicKey,
  Transaction,
  TransactionInstruction,
  TransactionSignature,
} from '@solana/web3.js';
import BN from 'bn.js';
import * as bs58 from 'bs58';
import { MangoClient } from '../../client';

export class Serum3Market {
  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      serumProgram: PublicKey;
      serumMarketExternal: PublicKey;
      marketIndex: number;
      baseTokenIndex: number;
      quoteTokenIndex: number;
      bump: number;
      reserved: unknown;
    },
  ): Serum3Market {
    return new Serum3Market(
      publicKey,
      obj.group,
      obj.serumProgram,
      obj.serumMarketExternal,
      obj.marketIndex,
      obj.baseTokenIndex,
      obj.quoteTokenIndex,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public serumProgram: PublicKey,
    public serumMarketExternal: PublicKey,
    public marketIndex: number,
    public baseTokenIndex: number,
    public quoteTokenIndex: number,
  ) {}
}

export async function serum3RegisterMarket(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  serumProgramPk: PublicKey,
  serumMarketExternalPk: PublicKey,
  quoteBankPk: PublicKey,
  baseBankPk: PublicKey,
  marketIndex: number,
): Promise<TransactionSignature> {
  const tx = new Transaction();
  const ix = await serum3RegisterMarketIx(
    client,
    groupPk,
    adminPk,
    serumProgramPk,
    serumMarketExternalPk,
    quoteBankPk,
    baseBankPk,
    marketIndex,
  );
  tx.add(ix);
  return await client.program.provider.send(tx);
}

export async function serum3RegisterMarketIx(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  serumProgramPk: PublicKey,
  serumMarketExternalPk: PublicKey,
  quoteBankPk: PublicKey,
  baseBankPk: PublicKey,
  marketIndex: number,
): Promise<TransactionInstruction> {
  return await client.program.methods
    .serum3RegisterMarket(marketIndex)
    .accounts({
      group: groupPk,
      admin: adminPk,
      serumProgram: serumProgramPk,
      serumMarketExternal: serumMarketExternalPk,
      quoteBank: quoteBankPk,
      baseBank: baseBankPk,
      payer: adminPk,
    })
    .instruction();
}

export async function getSerum3MarketForBaseAndQuote(
  client: MangoClient,
  groupPk: PublicKey,
  baseTokenIndex: number,
  quoteTokenIndex: number,
): Promise<Serum3Market[]> {
  const bbuf = Buffer.alloc(2);
  bbuf.writeUInt16LE(baseTokenIndex);

  const qbuf = Buffer.alloc(2);
  qbuf.writeUInt16LE(quoteTokenIndex);

  const bumpfbuf = Buffer.alloc(1);
  bumpfbuf.writeUInt8(255);

  return (
    await client.program.account.serum3Market.all([
      {
        memcmp: {
          bytes: groupPk.toBase58(),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: bs58.encode(bbuf),
          offset: 106,
        },
      },
      {
        memcmp: {
          bytes: bs58.encode(qbuf),
          offset: 108,
        },
      },
    ])
  ).map((tuple) => Serum3Market.from(tuple.publicKey, tuple.account));
}

export async function serum3CreateOpenOrders(
  client: MangoClient,
  groupPk: PublicKey,
  accountPk: PublicKey,
  serumMarketPk: PublicKey,
  serumProgramPk: PublicKey,
  serumMarketExternalPk: PublicKey,
  ownerPk: PublicKey,
): Promise<void> {
  return await client.program.methods
    .serum3CreateOpenOrders()
    .accounts({
      group: groupPk,
      account: accountPk,
      serumMarket: serumMarketPk,
      serumProgram: serumProgramPk,
      serumMarketExternal: serumMarketExternalPk,
      owner: ownerPk,
      payer: ownerPk,
    })
    .rpc();
}
export class Serum3SelfTradeBehavior {
  static decrementTake = { decrementTake: {} };
  static cancelProvide = { cancelProvide: {} };
  static abortTransaction = { abortTransaction: {} };
}

export class Serum3OrderType {
  static limit = { limit: {} };
  static immediateOrCancel = { immediateOrCancel: {} };
  static postOnly = { postOnly: {} };
}

export class Serum3Side {
  static bid = { bid: {} };
  static ask = { ask: {} };
}

export async function serum3PlaceOrder(
  client: MangoClient,
  groupPk: PublicKey,
  accountPk: PublicKey,
  ownerPk: PublicKey,
  openOrdersPk: PublicKey,
  serumMarketPk: PublicKey,
  serumProgramPk: PublicKey,
  serumMarketExternalPk: PublicKey,
  marketBidsPk: PublicKey,
  marketAsksPk: PublicKey,
  marketEventQueuePk: PublicKey,
  marketRequestQueuePk: PublicKey,
  marketBaseVaultPk: PublicKey,
  marketQuoteVaultPk: PublicKey,
  marketVaultSignerPk: PublicKey,
  quoteBankPk: PublicKey,
  quoteVaultPk: PublicKey,
  baseBankPk: PublicKey,
  baseVaultPk: PublicKey,
  healthRemainingAccounts: PublicKey[],
  side: Serum3Side,
  limitPrice: number,
  maxBaseQty: number,
  maxNativeQuoteQtyIncludingFees: number,
  selfTradeBehavior: Serum3SelfTradeBehavior,
  orderType: Serum3OrderType,
  clientOrderId: number,
  limit: number,
): Promise<void> {
  return await client.program.methods
    .serum3PlaceOrder(
      side,
      new BN(limitPrice),
      new BN(maxBaseQty),
      new BN(maxNativeQuoteQtyIncludingFees),
      selfTradeBehavior,
      orderType,
      new BN(clientOrderId),
      limit,
    )
    .accounts({
      group: groupPk,
      account: accountPk,
      owner: ownerPk,
      openOrders: openOrdersPk,
      serumMarket: serumMarketPk,
      serumProgram: serumProgramPk,
      serumMarketExternal: serumMarketExternalPk,
      marketBids: marketBidsPk,
      marketAsks: marketAsksPk,
      marketEventQueue: marketEventQueuePk,
      marketRequestQueue: marketRequestQueuePk,
      marketBaseVault: marketBaseVaultPk,
      marketQuoteVault: marketQuoteVaultPk,
      marketVaultSigner: marketVaultSignerPk,
      quoteBank: quoteBankPk,
      quoteVault: quoteVaultPk,
      baseBank: baseBankPk,
      baseVault: baseVaultPk,
    })
    .remainingAccounts(
      healthRemainingAccounts.map(
        (pk) =>
          ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
      ),
    )

    .rpc();
}
