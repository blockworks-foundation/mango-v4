import { PublicKey } from '@solana/web3.js';

export class Group {
  static from(publicKey: PublicKey, obj: { admin: PublicKey }): Group {
    return new Group(publicKey, obj.admin);
  }

  constructor(public publicKey: PublicKey, public admin: PublicKey) {}
}
