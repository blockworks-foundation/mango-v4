import { I80F48, I80F48Dto } from './I80F48';
import { Keypair, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { MangoClient } from '../../client';

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

export async function createStubOracle(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  tokenMintPk: PublicKey,
  payer: Keypair,
  staticPrice: number,
): Promise<void> {
  return await client.program.methods
    .createStubOracle({ val: I80F48.fromNumber(staticPrice).getData() })
    .accounts({
      group: groupPk,
      admin: adminPk,
      tokenMint: tokenMintPk,
      payer: payer.publicKey,
    })
    .signers([payer])
    .rpc();
}

export async function setStubOracle(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  tokenMintPk: PublicKey,
  payer: Keypair,
  staticPrice: number,
): Promise<void> {
  return await client.program.methods
    .setStubOracle({ val: new BN(staticPrice) })
    .accounts({
      group: groupPk,
      admin: adminPk,
      tokenMint: tokenMintPk,
      payer: payer.publicKey,
    })
    .signers([payer])
    .rpc();
}

export async function getStubOracleForGroupAndMint(
  client: MangoClient,
  groupPk: PublicKey,
  mintPk: PublicKey,
): Promise<StubOracle[]> {
  return (
    await client.program.account.stubOracle.all([
      {
        memcmp: {
          bytes: groupPk.toBase58(),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: mintPk.toBase58(),
          offset: 40,
        },
      },
    ])
  ).map((pa) => StubOracle.from(pa.publicKey, pa.account));
}
