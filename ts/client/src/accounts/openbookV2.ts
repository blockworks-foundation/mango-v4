import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import {
  OpenBookV2Client,
  BookSideAccount,
  MarketAccount,
  baseLotsToUi,
  priceLotsToUi,
} from '@openbook-dex/openbook-v2';
import { Cluster, Keypair, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { MangoClient } from '../client';
import { OPENBOOK_V2_PROGRAM_ID } from '../constants';
import { MAX_I80F48, ONE_I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { As, EmptyWallet } from '../utils';
import { TokenIndex } from './bank';
import { Group } from './group';
import { AnchorProvider, Wallet } from '@coral-xyz/anchor';

export type OpenbookV2MarketIndex = number & As<'market-index'>;

export class OpenbookV2Market {
  public name: string;
  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      baseTokenIndex: number;
      quoteTokenIndex: number;
      name: number[];
      openbookV2Program: PublicKey;
      openbookV2MarketExternal: PublicKey;
      marketIndex: number;
      registrationTime: BN;
      reduceOnly: number;
      forceClose: number;
    },
  ): OpenbookV2Market {
    return new OpenbookV2Market(
      publicKey,
      obj.group,
      obj.baseTokenIndex as TokenIndex,
      obj.quoteTokenIndex as TokenIndex,
      obj.name,
      obj.openbookV2Program,
      obj.openbookV2MarketExternal,
      obj.marketIndex as OpenbookV2MarketIndex,
      obj.registrationTime,
      obj.reduceOnly == 1,
      obj.forceClose == 1,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public baseTokenIndex: TokenIndex,
    public quoteTokenIndex: TokenIndex,
    name: number[],
    public openbookProgram: PublicKey,
    public openbookMarketExternal: PublicKey,
    public marketIndex: OpenbookV2MarketIndex,
    public registrationTime: BN,
    public reduceOnly: boolean,
    public forceClose: boolean,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
  }

  public findOoIndexerPda(
    programId: PublicKey,
    mangoAccount: PublicKey,
  ): PublicKey {
    const [openOrderPublicKey] = PublicKey.findProgramAddressSync(
      [Buffer.from('OpenOrdersIndexer'), mangoAccount.toBuffer()],
      programId,
    );

    return openOrderPublicKey;
  }

  public findOoPda(
    programId: PublicKey,
    mangoAccount: PublicKey,
    index: number,
  ): PublicKey {
    const indexBuf = Buffer.alloc(4);
    indexBuf.writeUInt32LE(index);
    const [openOrderPublicKey] = PublicKey.findProgramAddressSync(
      [Buffer.from('OpenOrders'), mangoAccount.toBuffer(), indexBuf],
      programId,
    );

    return openOrderPublicKey;
  }

  public async getNextOoPda(
    client: MangoClient,
    programId: PublicKey,
    mangoAccount: PublicKey,
  ): Promise<PublicKey> {
    const openbookClient = new OpenBookV2Client(
      new AnchorProvider(
        client.connection,
        new EmptyWallet(Keypair.generate()),
        {
          commitment: client.connection.commitment,
        },
      ),
    );
    const indexer =
      await openbookClient.program.account.openOrdersIndexer.fetchNullable(
        this.findOoIndexerPda(programId, mangoAccount),
      );
    const nextIndex = indexer ? indexer.createdCounter + 1 : 1;
    const indexBuf = Buffer.alloc(4);
    indexBuf.writeUInt32LE(nextIndex);
    const [openOrderPublicKey] = PublicKey.findProgramAddressSync(
      [Buffer.from('OpenOrders'), mangoAccount.toBuffer(), indexBuf],
      programId,
    );
    console.log('nextoo', nextIndex, openOrderPublicKey.toBase58());
    return openOrderPublicKey;
  }

  public getFeeRates(taker = true): number {
    // todo-pan: fees are no longer hardcoded!!
    // See https://github.com/openbook-dex/program/blob/master/dex/src/fees.rs#L81
    const ratesBps =
      this.name === 'USDT/USDC'
        ? { maker: -0.5, taker: 1 }
        : { maker: -2, taker: 4 };
    return taker ? ratesBps.taker * 0.0001 : ratesBps.maker * 0.0001;
  }

  /**
   *
   * @param group
   * @returns maximum leverage one can bid on this market, this is only for display purposes,
   *  also see getMaxQuoteForOpenbookV2BidUi and getMaxBaseForOpenbookV2AskUi
   */
  maxBidLeverage(group: Group): number {
    const baseBank = group.getFirstBankByTokenIndex(this.baseTokenIndex);
    const quoteBank = group.getFirstBankByTokenIndex(this.quoteTokenIndex);
    if (
      quoteBank.initLiabWeight.sub(baseBank.initAssetWeight).lte(ZERO_I80F48())
    ) {
      return MAX_I80F48().toNumber();
    }

    return ONE_I80F48()
      .div(quoteBank.initLiabWeight.sub(baseBank.initAssetWeight))
      .toNumber();
  }

  /**
   *
   * @param group
   * @returns maximum leverage one can ask on this market, this is only for display purposes,
   *  also see getMaxQuoteForOpenbookV2BidUi and getMaxBaseForOpenbookV2AskUi
   */
  maxAskLeverage(group: Group): number {
    const baseBank = group.getFirstBankByTokenIndex(this.baseTokenIndex);
    const quoteBank = group.getFirstBankByTokenIndex(this.quoteTokenIndex);

    if (
      baseBank.initLiabWeight.sub(quoteBank.initAssetWeight).lte(ZERO_I80F48())
    ) {
      return MAX_I80F48().toNumber();
    }

    return ONE_I80F48()
      .div(baseBank.initLiabWeight.sub(quoteBank.initAssetWeight))
      .toNumber();
  }

  public async loadBids(
    client: MangoClient,
    group: Group,
  ): Promise<BookSideAccount> {
    const openbookClient = new OpenBookV2Client(
      new AnchorProvider(
        client.connection,
        new EmptyWallet(Keypair.generate()),
        {
          commitment: client.connection.commitment,
        },
      ),
    ); // readonly client for deserializing accounts
    const openbookMarketExternal = group.getOpenbookV2ExternalMarket(
      this.openbookMarketExternal,
    );

    return await openbookClient.program.account.bookSide.fetch(
      openbookMarketExternal.bids,
    );
  }

  public async loadAsks(
    client: MangoClient,
    group: Group,
  ): Promise<BookSideAccount> {
    const openbookClient = new OpenBookV2Client(
      new AnchorProvider(
        client.connection,
        new EmptyWallet(Keypair.generate()),
        {
          commitment: client.connection.commitment,
        },
      ),
    ); // readonly client for deserializing accounts
    const openbookMarketExternal = group.getOpenbookV2ExternalMarket(
      this.openbookMarketExternal,
    );

    return await openbookClient.program.account.bookSide.fetch(
      openbookMarketExternal.asks,
    );
  }

  public async computePriceForMarketOrderOfSize(
    client: MangoClient,
    group: Group,
    size: number,
    side: 'buy' | 'sell',
  ): Promise<number> {
    const ob =
      side == 'buy'
        ? await this.loadBids(client, group)
        : await this.loadAsks(client, group);
    let acc = 0;
    let selectedOrder;
    const orderSize = size;

    const openbookMarketExternal = group.getOpenbookV2ExternalMarket(
      this.openbookMarketExternal,
    );

    for (const order of this.getL2(client, openbookMarketExternal, ob)) {
      acc += order[1];
      if (acc >= orderSize) {
        selectedOrder = order;
        break;
      }
    }

    if (!selectedOrder) {
      throw new Error(
        'Unable to place market order for this order size. Please retry.',
      );
    }

    if (side === 'buy') {
      return selectedOrder[0] * 1.05 /* TODO Fix random constant */;
    } else {
      return selectedOrder[0] * 0.95 /* TODO Fix random constant */;
    }
  }

  public getL2(
    client: MangoClient,
    marketAccount: MarketAccount,
    bidsAccount?: BookSideAccount,
    asksAccount?: BookSideAccount,
  ): [number, number][] {
    const openbookClient = new OpenBookV2Client(
      new AnchorProvider(
        client.connection,
        new EmptyWallet(Keypair.generate()),
        {
          commitment: client.connection.commitment,
        },
      ),
    ); // readonly client for deserializing accounts
    const bidNodes = bidsAccount
      ? openbookClient.getLeafNodes(bidsAccount)
      : [];
    const askNodes = asksAccount
      ? openbookClient.getLeafNodes(asksAccount)
      : [];
    const levels: [number, number][] = [];

    for (const node of bidNodes.concat(askNodes)) {
      const priceLots = node.key.shrn(64);
      levels.push([
        priceLotsToUi(marketAccount, priceLots),
        baseLotsToUi(marketAccount, node.quantity),
      ]);
    }
    return levels;
  }

  public async logOb(client: MangoClient, group: Group): Promise<string> {
    // todo-pan
    const res = ``;
    // res += `  ${this.name} OrderBook`;
    // let orders = await this?.loadAsks(client, group);
    // for (const order of orders!.items(true)) {
    //   res += `\n  ${order.price.toString().padStart(10)}, ${order.size
    //     .toString()
    //     .padStart(10)}`;
    // }
    // res += `\n  --------------------------`;
    // orders = await this?.loadBids(client, group);
    // for (const order of orders!.items(true)) {
    //   res += `\n  ${order.price.toString().padStart(10)}, ${order.size
    //     .toString()
    //     .padStart(10)}`;
    // }
    return res;
  }
}

export type OpenbookV2OrderType =
  | { limit: Record<string, never> }
  | { immediateOrCancel: Record<string, never> }
  | { postOnly: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace OpenbookV2OrderType {
  export const limit = { limit: {} };
  export const immediateOrCancel = { immediateOrCancel: {} };
  export const postOnly = { postOnly: {} };
}

export type OpenbookV2SelfTradeBehavior =
  | { decrementTake: Record<string, never> }
  | { cancelProvide: Record<string, never> }
  | { abortTransaction: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace OpenbookV2SelfTradeBehavior {
  export const decrementTake = { decrementTake: {} };
  export const cancelProvide = { cancelProvide: {} };
  export const abortTransaction = { abortTransaction: {} };
}

export type OpenbookV2Side =
  | { bid: Record<string, never> }
  | { ask: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace OpenbookV2Side {
  export const bid = { bid: {} };
  export const ask = { ask: {} };
}

export function generateOpenbookV2MarketExternalVaultSignerAddress(
  openbookV2Market: OpenbookV2Market,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('Market'), openbookV2Market.openbookMarketExternal.toBuffer()],
    openbookV2Market.openbookProgram,
  )[0];
}

export function priceNumberToLots(price: number, market: MarketAccount): BN {
  return new BN(
    Math.round(
      (price *
        Math.pow(10, market.quoteDecimals) *
        market.baseLotSize.toNumber()) /
        (Math.pow(10, market.baseDecimals) * market.quoteLotSize.toNumber()),
    ),
  );
}

export function baseSizeNumberToLots(size: number, market: MarketAccount): BN {
  const native = new BN(Math.round(size * Math.pow(10, market.baseDecimals)));
  // rounds down to the nearest lot size
  return native.div(market.baseLotSize);
}
