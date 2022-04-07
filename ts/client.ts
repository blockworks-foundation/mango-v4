import { Program, Provider } from '@project-serum/anchor';
import * as spl from '@solana/spl-token';
import { PublicKey, TransactionSignature } from '@solana/web3.js';
import {
  Bank,
  getBanksForGroup,
  getMintInfoForTokenIndex,
  registerToken,
} from './accounts/types/bank';
import { createGroup, getGroupForAdmin, Group } from './accounts/types/group';
import {
  createMangoAccount,
  deposit,
  getMangoAccountsForGroupAndOwner,
  MangoAccount,
  withdraw,
} from './accounts/types/mangoAccount';
import {
  createStubOracle,
  getStubOracleForGroupAndMint,
  setStubOracle,
  StubOracle,
} from './accounts/types/oracle';
import { IDL, MangoV4 } from './mango_v4';

export const MANGO_V4_ID = new PublicKey(
  'm43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD',
);

export class MangoClient {
  constructor(public program: Program<MangoV4>, public devnet?: boolean) {}

  /// public

  // Group

  public async createGroup() {
    return await createGroup(this, this.program.provider.wallet.publicKey);
  }

  public async getGroup(adminPk: PublicKey): Promise<Group> {
    return await getGroupForAdmin(this, adminPk);
  }

  // Tokens/Banks

  public async registerToken(
    group: Group,
    mintPk: PublicKey,
    oraclePk: PublicKey,
    tokenIndex: number,
  ): Promise<TransactionSignature> {
    return await registerToken(
      this,
      group.publicKey,
      this.program.provider.wallet.publicKey,
      mintPk,
      oraclePk,
      tokenIndex,
    );
  }

  public async getBanksForGroup(group: Group): Promise<Bank[]> {
    return await getBanksForGroup(this, group.publicKey);
  }

  // Stub Oracle

  public async createStubOracle(
    group: Group,
    mintPk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await createStubOracle(
      this,
      group.publicKey,
      this.program.provider.wallet.publicKey,
      mintPk,
      price,
    );
  }

  public async setStubOracle(
    group: Group,
    mintPk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await setStubOracle(
      this,
      group.publicKey,
      this.program.provider.wallet.publicKey,
      mintPk,
      price,
    );
  }

  public async getStubOracle(
    group: Group,
    mintPk: PublicKey,
  ): Promise<StubOracle> {
    return await getStubOracleForGroupAndMint(this, group.publicKey, mintPk);
  }

  // MangoAccount

  public async createMangoAccount(
    group: Group,
    accountNumber: number,
  ): Promise<TransactionSignature> {
    return createMangoAccount(
      this,
      group.publicKey,
      this.program.provider.wallet.publicKey,
      accountNumber,
    );
  }

  public async getMangoAccount(
    group: Group,
    ownerPk: PublicKey,
  ): Promise<MangoAccount[]> {
    return await getMangoAccountsForGroupAndOwner(
      this,
      group.publicKey,
      ownerPk,
    );
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

    return await deposit(
      this,
      group.publicKey,
      mangoAccount.publicKey,
      bank.publicKey,
      bank.vault,
      tokenAccountPk,
      mangoAccount.owner,
      healthRemainingAccounts,
      amount,
    );
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

    return await withdraw(
      this,
      group.publicKey,
      mangoAccount.publicKey,
      bank.publicKey,
      bank.vault,
      tokenAccountPk,
      mangoAccount.owner,
      healthRemainingAccounts,
      amount,
      allowBorrow,
    );
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
