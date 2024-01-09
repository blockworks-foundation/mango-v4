import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { Market, Orderbook } from '@project-serum/serum';
import { Cluster, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { MangoClient } from '../client';
import { OPENBOOK_PROGRAM_ID } from '../constants';
import { MAX_I80F48, ONE_I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { As } from '../utils';
import { TokenIndex } from './bank';
import { Group } from './group';

export type MarketIndex = number & As<'market-index'>;

export class Serum3Market {
  public name: string;
  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      baseTokenIndex: number;
      quoteTokenIndex: number;
      name: number[];
      serumProgram: PublicKey;
      serumMarketExternal: PublicKey;
      marketIndex: number;
      registrationTime: BN;
      reduceOnly: number;
      forceClose: number;
      oraclePriceBand: number;
    },
  ): Serum3Market {
    return new Serum3Market(
      publicKey,
      obj.group,
      obj.baseTokenIndex as TokenIndex,
      obj.quoteTokenIndex as TokenIndex,
      obj.name,
      obj.serumProgram,
      obj.serumMarketExternal,
      obj.marketIndex as MarketIndex,
      obj.registrationTime,
      obj.reduceOnly == 1,
      obj.forceClose == 1,
      obj.oraclePriceBand,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public baseTokenIndex: TokenIndex,
    public quoteTokenIndex: TokenIndex,
    name: number[],
    public serumProgram: PublicKey,
    public serumMarketExternal: PublicKey,
    public marketIndex: MarketIndex,
    public registrationTime: BN,
    public reduceOnly: boolean,
    public forceClose: boolean,
    public oraclePriceBand: number,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
  }

  public async findOoPda(
    programId: PublicKey,
    mangoAccount: PublicKey,
  ): Promise<PublicKey> {
    const [openOrderPublicKey] = await PublicKey.findProgramAddress(
      [
        Buffer.from('Serum3OO'),
        mangoAccount.toBuffer(),
        this.publicKey.toBuffer(),
      ],
      programId,
    );

    return openOrderPublicKey;
  }

  public getFeeRates(taker = true): number {
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
   *  also see getMaxQuoteForSerum3BidUi and getMaxBaseForSerum3AskUi
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
   *  also see getMaxQuoteForSerum3BidUi and getMaxBaseForSerum3AskUi
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

  public async loadBids(client: MangoClient, group: Group): Promise<Orderbook> {
    const serum3MarketExternal = group.getSerum3ExternalMarket(
      this.serumMarketExternal,
    );
    return await serum3MarketExternal.loadBids(
      client.program.provider.connection,
    );
  }

  public async loadAsks(client: MangoClient, group: Group): Promise<Orderbook> {
    const serum3MarketExternal = group.getSerum3ExternalMarket(
      this.serumMarketExternal,
    );
    return await serum3MarketExternal.loadAsks(
      client.program.provider.connection,
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
    for (const order of ob.getL2(size * 2 /* TODO Fix random constant */)) {
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

  public async logOb(client: MangoClient, group: Group): Promise<string> {
    let res = ``;
    res += `  ${this.name} OrderBook`;
    let orders = await this?.loadAsks(client, group);
    for (const order of orders!.items(true)) {
      res += `\n  ${order.price.toString().padStart(10)}, ${order.size
        .toString()
        .padStart(10)}`;
    }
    res += `\n  --------------------------`;
    orders = await this?.loadBids(client, group);
    for (const order of orders!.items(true)) {
      res += `\n  ${order.price.toString().padStart(10)}, ${order.size
        .toString()
        .padStart(10)}`;
    }
    return res;
  }
}

export type Serum3OrderType =
  | { limit: Record<string, never> }
  | { immediateOrCancel: Record<string, never> }
  | { postOnly: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace Serum3OrderType {
  export const limit = { limit: {} };
  export const immediateOrCancel = { immediateOrCancel: {} };
  export const postOnly = { postOnly: {} };
}

export type Serum3SelfTradeBehavior =
  | { decrementTake: Record<string, never> }
  | { cancelProvide: Record<string, never> }
  | { abortTransaction: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace Serum3SelfTradeBehavior {
  export const decrementTake = { decrementTake: {} };
  export const cancelProvide = { cancelProvide: {} };
  export const abortTransaction = { abortTransaction: {} };
}

export type Serum3Side =
  | { bid: Record<string, never> }
  | { ask: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace Serum3Side {
  export const bid = { bid: {} };
  export const ask = { ask: {} };
}

export async function generateSerum3MarketExternalVaultSignerAddress(
  cluster: Cluster,
  serum3Market: Serum3Market,
  serum3MarketExternal: Market,
): Promise<PublicKey> {
  return await PublicKey.createProgramAddress(
    [
      serum3Market.serumMarketExternal.toBuffer(),
      serum3MarketExternal.decoded.vaultSignerNonce.toArrayLike(
        Buffer,
        'le',
        8,
      ),
    ],
    OPENBOOK_PROGRAM_ID[cluster],
  );
}
