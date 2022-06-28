import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { I80F48, I80F48Dto } from './I80F48';

export class StubOracle {
  public price: I80F48;
  public lastUpdated: number;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      mint: PublicKey;
      price: I80F48Dto;
      lastUpdated: BN;
      reserved: unknown;
    },
  ): StubOracle {
    console.log(publicKey);
    console.log(publicKey);

    return new StubOracle(
      publicKey,
      obj.group,
      obj.mint,
      obj.price,
      obj.lastUpdated,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public mint: PublicKey,
    price: I80F48Dto,
    lastUpdated: BN,
  ) {
    this.price = I80F48.from(price);
    this.lastUpdated = lastUpdated.toNumber();
  }
}
