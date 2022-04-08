import { PublicKey } from '@solana/web3.js';

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
