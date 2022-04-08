import { AccountMeta, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
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
