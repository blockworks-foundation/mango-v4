import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { Market, Orderbook } from '@project-serum/serum/lib/market';
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
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
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
