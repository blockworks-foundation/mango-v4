import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { Market, Orderbook } from '@project-serum/serum/lib/market';
import { Cluster, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { MangoClient } from '../client';
import { SERUM3_PROGRAM_ID } from '../constants';
import { Group } from './group';
import { MAX_I80F48, ONE_I80F48, ZERO_I80F48 } from './I80F48';

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
      bump: number;
      registrationTime: BN;
    },
  ): Serum3Market {
    return new Serum3Market(
      publicKey,
      obj.group,
      obj.baseTokenIndex,
      obj.quoteTokenIndex,
      obj.name,
      obj.serumProgram,
      obj.serumMarketExternal,
      obj.marketIndex,
      obj.registrationTime,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public baseTokenIndex: number,
    public quoteTokenIndex: number,
    name: number[],
    public serumProgram: PublicKey,
    public serumMarketExternal: PublicKey,
    public marketIndex: number,
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
    if (!baseBank) {
      throw new Error(
        `bank for base token with index ${this.baseTokenIndex} not found`,
      );
    }

    const quoteBank = group.getFirstBankByTokenIndex(this.quoteTokenIndex);
    if (!quoteBank) {
      throw new Error(
        `bank for quote token with index ${this.quoteTokenIndex} not found`,
      );
    }

    if (
      quoteBank.initLiabWeight.sub(baseBank.initAssetWeight).lte(ZERO_I80F48)
    ) {
      return MAX_I80F48.toNumber();
    }

    return ONE_I80F48.div(
      quoteBank.initLiabWeight.sub(baseBank.initAssetWeight),
    ).toNumber();
  }

  /**
   *
   * @param group
   * @returns maximum leverage one can ask on this market, this is only for display purposes,
   *  also see getMaxQuoteForSerum3BidUi and getMaxBaseForSerum3AskUi
   */
  maxAskLeverage(group: Group): number {
    const baseBank = group.getFirstBankByTokenIndex(this.baseTokenIndex);
    if (!baseBank) {
      throw new Error(
        `bank for base token with index ${this.baseTokenIndex} not found`,
      );
    }

    const quoteBank = group.getFirstBankByTokenIndex(this.quoteTokenIndex);
    if (!quoteBank) {
      throw new Error(
        `bank for quote token with index ${this.quoteTokenIndex} not found`,
      );
    }

    if (
      baseBank.initLiabWeight.sub(quoteBank.initAssetWeight).lte(ZERO_I80F48)
    ) {
      return MAX_I80F48.toNumber();
    }

    return ONE_I80F48.div(
      baseBank.initLiabWeight.sub(quoteBank.initAssetWeight),
    ).toNumber();
  }

  public async loadBids(client: MangoClient, group: Group): Promise<Orderbook> {
    const serum3MarketExternal = group.serum3MarketExternalsMap.get(
      this.serumMarketExternal.toBase58(),
    );
    if (!serum3MarketExternal) {
      throw new Error(
        `Unable to find serum3MarketExternal for ${this.serumMarketExternal.toBase58()}`,
      );
    }
    return await serum3MarketExternal.loadBids(
      client.program.provider.connection,
    );
  }

  public async loadAsks(client: MangoClient, group: Group): Promise<Orderbook> {
    const serum3MarketExternal = group.serum3MarketExternalsMap.get(
      this.serumMarketExternal.toBase58(),
    );
    if (!serum3MarketExternal) {
      throw new Error(
        `Unable to find serum3MarketExternal for ${this.serumMarketExternal.toBase58()}`,
      );
    }
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
    SERUM3_PROGRAM_ID[cluster],
  );
}
