import { BN } from '@project-serum/anchor';

export class TokenIndex {
  public val: BN;

  constructor(val: BN) {
    this.val = val;
  }

  static from(from: { val: BN }): TokenIndex {
    return new TokenIndex(from.val);
  }

  static fromValue(value: BN) {
    return new TokenIndex(value);
  }

  public toString = (): string => {
    let fract = (
      this.val.uand(new BN(0xffffffffffff)).toNumber() / Math.pow(2.0, 48)
    )
      .toString()
      .slice(1);
    return `${this.val.ushrn(48).toNumber()}${fract}`;
  };
}
