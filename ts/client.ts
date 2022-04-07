import { BN, Program, Provider } from '@project-serum/anchor';
import * as spl from '@solana/spl-token';
import {
  AccountMeta,
  PublicKey,
  SYSVAR_RENT_PUBKEY,
  TransactionSignature,
} from '@solana/web3.js';
import { Bank, getMintInfoForTokenIndex } from './accounts/types/bank';
import { Group } from './accounts/types/group';
import { I80F48 } from './accounts/types/I80F48';
import { MangoAccount } from './accounts/types/mangoAccount';
import { StubOracle } from './accounts/types/oracle';
import { IDL, MangoV4 } from './mango_v4';

export const MANGO_V4_ID = new PublicKey(
  'm43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD',
);

export class MangoClient {
  constructor(public program: Program<MangoV4>, public devnet?: boolean) {}

  /// public

  // Group

  public async createGroup(): Promise<TransactionSignature> {
    const adminPk = this.program.provider.wallet.publicKey;
    return await this.program.methods
      .createGroup()
      .accounts({
        admin: adminPk,
        payer: adminPk,
      })
      .rpc();
  }

  public async getGroup(adminPk: PublicKey): Promise<Group> {
    const groups = (
      await this.program.account.group.all([
        {
          memcmp: {
            bytes: adminPk.toBase58(),
            offset: 8,
          },
        },
      ])
    ).map((tuple) => Group.from(tuple.publicKey, tuple.account));
    return groups[0];
  }

  // Tokens/Banks

  public async registerToken(
    group: Group,
    mintPk: PublicKey,
    oraclePk: PublicKey,
    tokenIndex: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .registerToken(
        tokenIndex,
        0.8,
        0.6,
        1.2,
        1.4,
        0.02 /*TODO expose as args*/,
      )
      .accounts({
        group: group.publicKey,
        admin: this.program.provider.wallet.publicKey,
        mint: mintPk,
        oracle: oraclePk,
        payer: this.program.provider.wallet.publicKey,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();
  }

  public async getBanksForGroup(group: Group): Promise<Bank[]> {
    return (
      await this.program.account.bank.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 8,
          },
        },
      ])
    ).map((tuple) => Bank.from(tuple.publicKey, tuple.account));
  }

  // Stub Oracle

  public async createStubOracle(
    group: Group,
    mintPk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .createStubOracle({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: this.program.provider.wallet.publicKey,
        tokenMint: mintPk,
        payer: this.program.provider.wallet.publicKey,
      })
      .rpc();
  }

  public async setStubOracle(
    group: Group,
    mintPk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .setStubOracle({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: this.program.provider.wallet.publicKey,
        tokenMint: mintPk,
        payer: this.program.provider.wallet.publicKey,
      })
      .rpc();
  }

  public async getStubOracle(
    group: Group,
    mintPk: PublicKey,
  ): Promise<StubOracle> {
    const stubOracles = (
      await this.program.account.stubOracle.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
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
    return stubOracles[0];
  }

  // MangoAccount

  public async createMangoAccount(
    group: Group,
    accountNumber: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .createAccount(accountNumber)
      .accounts({
        group: group.publicKey,
        owner: this.program.provider.wallet.publicKey,
        payer: this.program.provider.wallet.publicKey,
      })
      .rpc();
  }

  public async getMangoAccount(
    group: Group,
    ownerPk: PublicKey,
  ): Promise<MangoAccount[]> {
    return (
      await this.program.account.mangoAccount.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 8,
          },
        },
        {
          memcmp: {
            bytes: ownerPk.toBase58(),
            offset: 40,
          },
        },
      ])
    ).map((pa) => {
      return MangoAccount.from(pa.publicKey, pa.account);
    });
  }

  public async deposit(
    group: Group,
    mangoAccount: MangoAccount,
    bank: Bank,
    amount: number,
  ) {
    const tokenAccountPk = await spl.getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
    );

    const healthRemainingAccounts: PublicKey[] =
      await this.buildHealthRemainingAccounts(group, mangoAccount, bank);

    return await this.program.methods
      .deposit(new BN(amount))
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        tokenAccount: tokenAccountPk,
        tokenAuthority: this.program.provider.wallet.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .rpc();
  }

  public async withdraw(
    group: Group,
    mangoAccount: MangoAccount,
    bank: Bank,
    amount: number,
    allowBorrow: boolean,
  ) {
    const tokenAccountPk = await spl.getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
    );

    const healthRemainingAccounts: PublicKey[] =
      await this.buildHealthRemainingAccounts(group, mangoAccount, bank);

    return await this.program.methods
      .withdraw(new BN(amount), allowBorrow)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        tokenAccount: tokenAccountPk,
        tokenAuthority: this.program.provider.wallet.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .rpc();
  }

  /// static

  static async connect(
    provider: Provider,
    devnet?: boolean,
  ): Promise<MangoClient> {
    // TODO: use IDL on chain or in repository? decide...
    // Alternatively we could fetch IDL from chain.
    // const idl = await Program.fetchIdl(MANGO_V4_ID, provider);
    let idl = IDL;

    // TODO: remove...
    // Temporarily add missing (dummy) type definitions, so we can do new Program(...) below
    // without anchor throwing errors. These types come from part of the code we don't yet care about
    // in the client.
    function addDummyType(idl: MangoV4, typeName: string) {
      if (idl.types.find((type) => type.name === typeName)) {
        return;
      }
      (idl.types as any).push({
        name: typeName,
        type: {
          kind: 'struct',
          fields: [],
        },
      });
    }
    addDummyType(idl, 'usize');
    addDummyType(idl, 'AnyNode');
    addDummyType(idl, 'EventQueueHeader');
    addDummyType(idl, 'AnyEvent');
    addDummyType(idl, 'H');
    addDummyType(idl, 'H::Item');
    addDummyType(idl, 'NodeHandle');

    return new MangoClient(
      new Program<MangoV4>(idl as MangoV4, MANGO_V4_ID, provider),
      devnet,
    );
  }

  /// private

  private async buildHealthRemainingAccounts(
    group: Group,
    mangoAccount: MangoAccount,
    bank: Bank,
  ) {
    const healthRemainingAccounts: PublicKey[] = [];
    {
      const tokenIndices = mangoAccount.tokens
        .filter((token) => token.tokenIndex !== 65535)
        .map((token) => token.tokenIndex);
      tokenIndices.push(bank.tokenIndex);

      const mintInfos = await Promise.all(
        [...new Set(tokenIndices)].map(async (tokenIndex) =>
          getMintInfoForTokenIndex(this, group.publicKey, tokenIndex),
        ),
      );
      healthRemainingAccounts.push(
        ...mintInfos.flatMap((mintinfos) => {
          return mintinfos.flatMap((mintinfo) => {
            return mintinfo.bank;
          });
        }),
      );
      healthRemainingAccounts.push(
        ...mintInfos.flatMap((mintinfos) => {
          return mintinfos.flatMap((mintinfo) => {
            return mintinfo.oracle;
          });
        }),
      );
      healthRemainingAccounts.push(
        ...mangoAccount.serum3
          .filter((serum3Account) => serum3Account.marketIndex !== 65535)
          .map((serum3Account) => serum3Account.openOrders),
      );
    }
    return healthRemainingAccounts;
  }
}
