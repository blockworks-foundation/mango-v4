import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

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
