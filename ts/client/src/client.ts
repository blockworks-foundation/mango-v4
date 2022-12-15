import { AnchorProvider, BN, Program, Provider } from '@project-serum/anchor';
import {
  closeAccount,
  initializeAccount,
  WRAPPED_SOL_MINT,
} from '@project-serum/serum/lib/token-instructions';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import {
  AccountMeta,
  Cluster,
  Keypair,
  MemcmpFilter,
  PublicKey,
  Signer,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
  TransactionSignature,
} from '@solana/web3.js';
import bs58 from 'bs58';
import { Bank, MintInfo, TokenIndex } from './accounts/bank';
import { Group } from './accounts/group';
import {
  MangoAccount,
  PerpPosition,
  TokenPosition,
} from './accounts/mangoAccount';
import { StubOracle } from './accounts/oracle';
import {
  FillEvent,
  OutEvent,
  PerpEventQueue,
  PerpMarket,
  PerpMarketIndex,
  PerpOrderSide,
  PerpOrderType,
} from './accounts/perp';
import {
  generateSerum3MarketExternalVaultSignerAddress,
  Serum3Market,
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from './accounts/serum3';
import { OPENBOOK_PROGRAM_ID } from './constants';
import { Id } from './ids';
import { IDL, MangoV4 } from './mango_v4';
import { I80F48 } from './numbers/I80F48';
import { FlashLoanType, InterestRateParams, OracleConfigParams } from './types';
import {
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddress,
  I64_MAX_BN,
  toNative,
} from './utils';
import { sendTransaction } from './utils/rpc';

enum AccountRetriever {
  Scanning,
  Fixed,
}

export type IdsSource = 'api' | 'static' | 'get-program-accounts';

export type MangoClientOptions = {
  idsSource?: IdsSource;
  postSendTxCallback?: ({ txid }: { txid: string }) => void;
  prioritizationFee?: number;
};

export class MangoClient {
  private idsSource: IdsSource;
  private postSendTxCallback?: ({ txid }) => void;
  private prioritizationFee: number;

  constructor(
    public program: Program<MangoV4>,
    public programId: PublicKey,
    public cluster: Cluster,
    public opts: MangoClientOptions = {},
  ) {
    this.idsSource = opts?.idsSource || 'get-program-accounts';
    this.prioritizationFee = opts?.prioritizationFee || 0;
    this.postSendTxCallback = opts?.postSendTxCallback;
    // TODO: evil side effect, but limited backtraces are a nightmare
    Error.stackTraceLimit = 1000;
  }

  /// public

  // Group

  public async groupCreate(
    groupNum: number,
    testing: boolean,
    version: number,
    insuranceMintPk: PublicKey,
  ): Promise<TransactionSignature> {
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;
    return await this.program.methods
      .groupCreate(groupNum, testing ? 1 : 0, version)
      .accounts({
        creator: adminPk,
        payer: adminPk,
        insuranceMint: insuranceMintPk,
      })
      .rpc();
  }

  public async groupEdit(
    group: Group,
    admin?: PublicKey,
    fastListingAdmin?: PublicKey,
    testing?: number,
    version?: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .groupEdit(
        admin ?? null,
        fastListingAdmin ?? null,
        testing ?? null,
        version ?? null,
      )
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async groupClose(group: Group): Promise<TransactionSignature> {
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;
    return await this.program.methods
      .groupClose()
      .accounts({
        group: group.publicKey,
        insuranceVault: group.insuranceVault,
        admin: adminPk,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async getGroup(groupPk: PublicKey): Promise<Group> {
    const groupAccount = await this.program.account.group.fetch(groupPk);
    const group = Group.from(groupPk, groupAccount);
    const ids: Id | undefined = await this.getIds(groupPk);
    await group.reloadAll(this, ids);
    return group;
  }

  public async getGroupsForCreator(creatorPk: PublicKey): Promise<Group[]> {
    const filters: MemcmpFilter[] = [
      {
        memcmp: {
          bytes: creatorPk.toBase58(),
          offset: 8,
        },
      },
    ];

    const groups = (await this.program.account.group.all(filters)).map(
      (tuple) => Group.from(tuple.publicKey, tuple.account),
    );
    groups.forEach((group) => group.reloadAll(this));
    return groups;
  }

  public async getGroupForCreator(
    creatorPk: PublicKey,
    groupNum: number,
  ): Promise<Group> {
    const bbuf = Buffer.alloc(4);
    bbuf.writeUInt32LE(groupNum);
    const filters: MemcmpFilter[] = [
      {
        memcmp: {
          bytes: creatorPk.toBase58(),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: bs58.encode(bbuf),
          offset: 40,
        },
      },
    ];
    const groups = (await this.program.account.group.all(filters)).map(
      (tuple) => Group.from(tuple.publicKey, tuple.account),
    );
    await groups[0].reloadAll(this);
    return groups[0];
  }

  public async getIds(groupPk: PublicKey): Promise<Id | undefined> {
    switch (this.idsSource) {
      case 'api':
        return await Id.fromApi(groupPk);
      case 'get-program-accounts':
        return undefined;
      case 'static':
        return Id.fromIdsByPk(groupPk);
    }
  }

  // Tokens/Banks

  public async tokenRegister(
    group: Group,
    mintPk: PublicKey,
    oraclePk: PublicKey,
    oracleConfig: OracleConfigParams,
    tokenIndex: number,
    name: string,
    interestRateParams: InterestRateParams,
    loanFeeRate: number,
    loanOriginationFeeRate: number,
    maintAssetWeight: number,
    initAssetWeight: number,
    maintLiabWeight: number,
    initLiabWeight: number,
    liquidationFee: number,
    minVaultToDepositsRatio: number,
    netBorrowLimitWindowSizeTs: number,
    netBorrowLimitPerWindowQuote: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .tokenRegister(
        tokenIndex,
        name,
        oracleConfig,
        interestRateParams,
        loanFeeRate,
        loanOriginationFeeRate,
        maintAssetWeight,
        initAssetWeight,
        maintLiabWeight,
        initLiabWeight,
        liquidationFee,
        minVaultToDepositsRatio,
        new BN(netBorrowLimitWindowSizeTs),
        new BN(netBorrowLimitPerWindowQuote),
      )
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        mint: mintPk,
        oracle: oraclePk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();
  }

  public async tokenRegisterTrustless(
    group: Group,
    mintPk: PublicKey,
    oraclePk: PublicKey,
    tokenIndex: number,
    name: string,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .tokenRegisterTrustless(tokenIndex, name)
      .accounts({
        group: group.publicKey,
        fastListingAdmin: (this.program.provider as AnchorProvider).wallet
          .publicKey,
        mint: mintPk,
        oracle: oraclePk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();
  }

  public async tokenEdit(
    group: Group,
    mintPk: PublicKey,
    oracle: PublicKey | null,
    oracleConfig: OracleConfigParams | null,
    groupInsuranceFund: boolean | null,
    interestRateParams: InterestRateParams | null,
    loanFeeRate: number | null,
    loanOriginationFeeRate: number | null,
    maintAssetWeight: number | null,
    initAssetWeight: number | null,
    maintLiabWeight: number | null,
    initLiabWeight: number | null,
    liquidationFee: number | null,
    stablePriceDelayIntervalSeconds: number | null,
    stablePriceDelayGrowthLimit: number | null,
    stablePriceGrowthLimit: number | null,
    minVaultToDepositsRatio: number | null,
    netBorrowLimitPerWindowQuote: number | null,
    netBorrowLimitWindowSizeTs: number | null,
    borrowWeightScaleStartQuote: number | null,
    depositWeightScaleStartQuote: number | null,
    resetStablePrice: boolean | null,
    resetNetBorrowLimit: boolean | null,
  ): Promise<TransactionSignature> {
    const bank = group.getFirstBankByMint(mintPk);
    const mintInfo = group.mintInfosMapByTokenIndex.get(bank.tokenIndex)!;

    return await this.program.methods
      .tokenEdit(
        oracle,
        oracleConfig,
        groupInsuranceFund,
        interestRateParams,
        loanFeeRate,
        loanOriginationFeeRate,
        maintAssetWeight,
        initAssetWeight,
        maintLiabWeight,
        initLiabWeight,
        liquidationFee,
        stablePriceDelayIntervalSeconds,
        stablePriceDelayGrowthLimit,
        stablePriceGrowthLimit,
        minVaultToDepositsRatio,
        netBorrowLimitPerWindowQuote !== null
          ? new BN(netBorrowLimitPerWindowQuote)
          : null,
        netBorrowLimitWindowSizeTs !== null
          ? new BN(netBorrowLimitWindowSizeTs)
          : null,
        borrowWeightScaleStartQuote,
        depositWeightScaleStartQuote,
        resetStablePrice ?? false,
        resetNetBorrowLimit ?? false,
      )
      .accounts({
        group: group.publicKey,
        oracle: oracle ?? bank.oracle,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        mintInfo: mintInfo.publicKey,
      })
      .remainingAccounts([
        {
          pubkey: bank.publicKey,
          isWritable: true,
          isSigner: false,
        } as AccountMeta,
      ])
      .rpc();
  }

  public async tokenDeregister(
    group: Group,
    mintPk: PublicKey,
  ): Promise<TransactionSignature> {
    const bank = group.getFirstBankByMint(mintPk);
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;

    const dustVaultPk = await getAssociatedTokenAddress(bank.mint, adminPk);
    const ai = await this.program.provider.connection.getAccountInfo(
      dustVaultPk,
    );
    const preInstructions: TransactionInstruction[] = [];
    if (!ai) {
      preInstructions.push(
        await createAssociatedTokenAccountIdempotentInstruction(
          adminPk,
          adminPk,
          bank.mint,
        ),
      );
    }

    const ix = await this.program.methods
      .tokenDeregister()
      .accounts({
        group: group.publicKey,
        admin: adminPk,
        mintInfo: group.mintInfosMapByTokenIndex.get(bank.tokenIndex)
          ?.publicKey,
        dustVault: dustVaultPk,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .remainingAccounts(
        [bank.publicKey, bank.vault].map(
          (pk) =>
            ({ pubkey: pk, isWritable: true, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [...preInstructions, ix],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
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

  public async getMintInfosForGroup(group: Group): Promise<MintInfo[]> {
    return (
      await this.program.account.mintInfo.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 8,
          },
        },
      ])
    ).map((tuple) => {
      return MintInfo.from(tuple.publicKey, tuple.account);
    });
  }

  public async getMintInfoForTokenIndex(
    group: Group,
    tokenIndex: TokenIndex,
  ): Promise<MintInfo[]> {
    const tokenIndexBuf = Buffer.alloc(2);
    tokenIndexBuf.writeUInt16LE(tokenIndex);
    return (
      await this.program.account.mintInfo.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 8,
          },
        },
        {
          memcmp: {
            bytes: bs58.encode(tokenIndexBuf),
            offset: 40,
          },
        },
      ])
    ).map((tuple) => {
      return MintInfo.from(tuple.publicKey, tuple.account);
    });
  }

  // Stub Oracle

  public async stubOracleCreate(
    group: Group,
    mintPk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .stubOracleCreate({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        mint: mintPk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async stubOracleClose(
    group: Group,
    oracle: PublicKey,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .stubOracleClose()
      .accounts({
        group: group.publicKey,
        oracle: oracle,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async stubOracleSet(
    group: Group,
    oraclePk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .stubOracleSet({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        oracle: oraclePk,
      })
      .rpc();
  }

  public async getStubOracle(
    group: Group,
    mintPk?: PublicKey,
  ): Promise<StubOracle[]> {
    const filters = [
      {
        memcmp: {
          bytes: group.publicKey.toBase58(),
          offset: 8,
        },
      },
    ];

    if (mintPk) {
      filters.push({
        memcmp: {
          bytes: mintPk.toBase58(),
          offset: 40,
        },
      });
    }

    return (await this.program.account.stubOracle.all(filters)).map((pa) =>
      StubOracle.from(pa.publicKey, pa.account),
    );
  }

  // MangoAccount

  public async getOrCreateMangoAccount(group: Group): Promise<MangoAccount> {
    const clientOwner = (this.program.provider as AnchorProvider).wallet
      .publicKey;
    let mangoAccounts = await this.getMangoAccountsForOwner(
      group,
      (this.program.provider as AnchorProvider).wallet.publicKey,
    );
    if (mangoAccounts.length === 0) {
      await this.createMangoAccount(group);
      mangoAccounts = await this.getMangoAccountsForOwner(group, clientOwner);
    }
    return mangoAccounts.sort((a, b) => a.accountNum - b.accountNum)[0];
  }

  public async createMangoAccount(
    group: Group,
    accountNumber?: number,
    name?: string,
    tokenCount?: number,
    serum3Count?: number,
    perpCount?: number,
    perpOoCount?: number,
  ): Promise<TransactionSignature> {
    const ix = await this.program.methods
      .accountCreate(
        accountNumber ?? 0,
        tokenCount ?? 8,
        serum3Count ?? 8,
        perpCount ?? 8,
        perpOoCount ?? 8,
        name ?? '',
      )
      .accounts({
        group: group.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      [],
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async createAndFetchMangoAccount(
    group: Group,
    accountNumber?: number,
    name?: string,
    tokenCount?: number,
    serum3Count?: number,
    perpCount?: number,
    perpOoCount?: number,
  ): Promise<MangoAccount | undefined> {
    const accNum = accountNumber ?? 0;
    await this.createMangoAccount(
      group,
      accNum,
      name,
      tokenCount,
      serum3Count,
      perpCount,
      perpOoCount,
    );
    return await this.getMangoAccountForOwner(
      group,
      (this.program.provider as AnchorProvider).wallet.publicKey,
      accNum,
    );
  }

  public async expandMangoAccount(
    group: Group,
    account: MangoAccount,
    tokenCount: number,
    serum3Count: number,
    perpCount: number,
    perpOoCount: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .accountExpand(tokenCount, serum3Count, perpCount, perpOoCount)
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async editMangoAccount(
    group: Group,
    mangoAccount: MangoAccount,
    name?: string,
    delegate?: PublicKey,
  ): Promise<TransactionSignature> {
    const ix = await this.program.methods
      .accountEdit(name ?? null, delegate ?? null)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      [],
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async getMangoAccount(
    mangoAccount: MangoAccount | PublicKey,
  ): Promise<MangoAccount> {
    const mangoAccountPk =
      mangoAccount instanceof MangoAccount
        ? mangoAccount.publicKey
        : mangoAccount;
    return MangoAccount.from(
      mangoAccountPk,
      await this.program.account.mangoAccount.fetch(mangoAccountPk),
    );
  }

  public async getMangoAccountForPublicKey(
    mangoAccountPk: PublicKey,
  ): Promise<MangoAccount> {
    return MangoAccount.from(
      mangoAccountPk,
      await this.program.account.mangoAccount.fetch(mangoAccountPk),
    );
  }

  public async getMangoAccountWithSlot(
    mangoAccountPk: PublicKey,
  ): Promise<{ slot: number; value: MangoAccount } | undefined> {
    const resp =
      await this.program.provider.connection.getAccountInfoAndContext(
        mangoAccountPk,
      );
    if (!resp?.value) return;
    const decodedMangoAccount = this.program.coder.accounts.decode(
      'mangoAccount',
      resp.value.data,
    );
    const mangoAccount = MangoAccount.from(mangoAccountPk, decodedMangoAccount);
    return { slot: resp.context.slot, value: mangoAccount };
  }

  public async getMangoAccountForOwner(
    group: Group,
    ownerPk: PublicKey,
    accountNumber: number,
  ): Promise<MangoAccount | undefined> {
    const mangoAccounts = await this.getMangoAccountsForOwner(group, ownerPk);
    const foundMangoAccount = mangoAccounts.find(
      (a) => a.accountNum == accountNumber,
    );

    return foundMangoAccount;
  }

  public async getMangoAccountsForOwner(
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

  public async getMangoAccountsForDelegate(
    group: Group,
    delegate: PublicKey,
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
            bytes: delegate.toBase58(),
            offset: 104,
          },
        },
      ])
    ).map((pa) => {
      return MangoAccount.from(pa.publicKey, pa.account);
    });
  }

  public async getAllMangoAccounts(group: Group): Promise<MangoAccount[]> {
    return (
      await this.program.account.mangoAccount.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 8,
          },
        },
      ])
    ).map((pa) => {
      return MangoAccount.from(pa.publicKey, pa.account);
    });
  }

  public async closeMangoAccount(
    group: Group,
    mangoAccount: MangoAccount,
  ): Promise<TransactionSignature> {
    const ix = await this.program.methods
      .accountClose()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        solDestination: mangoAccount.owner,
      })
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      [],
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async tokenDeposit(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    amount: number,
  ): Promise<TransactionSignature> {
    const decimals = group.getMintDecimals(mintPk);
    const nativeAmount = toNative(amount, decimals);
    return await this.tokenDepositNative(
      group,
      mangoAccount,
      mintPk,
      nativeAmount,
    );
  }

  public async tokenDepositNative(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    nativeAmount: BN,
  ): Promise<TransactionSignature> {
    const bank = group.getFirstBankByMint(mintPk);

    const tokenAccountPk = await getAssociatedTokenAddress(
      mintPk,
      mangoAccount.owner,
    );

    let wrappedSolAccount: Keypair | undefined;
    let preInstructions: TransactionInstruction[] = [];
    let postInstructions: TransactionInstruction[] = [];
    const additionalSigners: Signer[] = [];
    if (mintPk.equals(WRAPPED_SOL_MINT)) {
      wrappedSolAccount = new Keypair();
      const lamports = nativeAmount.add(new BN(1e7));

      preInstructions = [
        SystemProgram.createAccount({
          fromPubkey: mangoAccount.owner,
          newAccountPubkey: wrappedSolAccount.publicKey,
          lamports: lamports.toNumber(),
          space: 165,
          programId: TOKEN_PROGRAM_ID,
        }),
        initializeAccount({
          account: wrappedSolAccount.publicKey,
          mint: WRAPPED_SOL_MINT,
          owner: mangoAccount.owner,
        }),
      ];
      postInstructions = [
        closeAccount({
          source: wrappedSolAccount.publicKey,
          destination: mangoAccount.owner,
          owner: mangoAccount.owner,
        }),
      ];
      additionalSigners.push(wrappedSolAccount);
    }

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [mangoAccount],
        [bank],
        [],
      );

    const ix = await this.program.methods
      .tokenDeposit(new BN(nativeAmount))
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: mangoAccount.owner,
        bank: bank.publicKey,
        vault: bank.vault,
        oracle: bank.oracle,
        tokenAccount: wrappedSolAccount?.publicKey ?? tokenAccountPk,
        tokenAuthority: mangoAccount.owner,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [...preInstructions, ix, ...postInstructions],
      group.addressLookupTablesList,
      {
        additionalSigners,
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async tokenWithdraw(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    amount: number,
    allowBorrow: boolean,
  ): Promise<TransactionSignature> {
    const nativeAmount = toNative(amount, group.getMintDecimals(mintPk));
    return await this.tokenWithdrawNative(
      group,
      mangoAccount,
      mintPk,
      nativeAmount,
      allowBorrow,
    );
  }

  public async tokenWithdrawNative(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    nativeAmount: BN,
    allowBorrow: boolean,
  ): Promise<TransactionSignature> {
    const bank = group.getFirstBankByMint(mintPk);

    const tokenAccountPk = await getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
    );

    // ensure withdraws don't fail with missing ATAs
    const preInstructions: TransactionInstruction[] = [
      await createAssociatedTokenAccountIdempotentInstruction(
        mangoAccount.owner,
        mangoAccount.owner,
        bank.mint,
      ),
    ];

    const postInstructions: TransactionInstruction[] = [];
    if (mintPk.equals(WRAPPED_SOL_MINT)) {
      postInstructions.push(
        closeAccount({
          source: tokenAccountPk,
          destination: mangoAccount.owner,
          owner: mangoAccount.owner,
        }),
      );
    }

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [mangoAccount],
        [bank],
        [],
      );

    const ix = await this.program.methods
      .tokenWithdraw(new BN(nativeAmount), allowBorrow)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: mangoAccount.owner,
        bank: bank.publicKey,
        vault: bank.vault,
        oracle: bank.oracle,
        tokenAccount: tokenAccountPk,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [...preInstructions, ix, ...postInstructions],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  // Serum

  public async serum3RegisterMarket(
    group: Group,
    serum3MarketExternalPk: PublicKey,
    baseBank: Bank,
    quoteBank: Bank,
    marketIndex: number,
    name: string,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .serum3RegisterMarket(marketIndex, name)
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3MarketExternalPk,
        baseBank: baseBank.publicKey,
        quoteBank: quoteBank.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async serum3deregisterMarket(
    group: Group,
    externalMarketPk: PublicKey,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const marketIndexBuf = Buffer.alloc(2);
    marketIndexBuf.writeUInt16LE(serum3Market.marketIndex);
    const [indexReservation] = await PublicKey.findProgramAddress(
      [Buffer.from('Serum3Index'), group.publicKey.toBuffer(), marketIndexBuf],
      this.program.programId,
    );

    return await this.program.methods
      .serum3DeregisterMarket()
      .accounts({
        group: group.publicKey,
        serumMarket: serum3Market.publicKey,
        indexReservation,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async serum3GetMarkets(
    group: Group,
    baseTokenIndex?: number,
    quoteTokenIndex?: number,
  ): Promise<Serum3Market[]> {
    const bumpfbuf = Buffer.alloc(1);
    bumpfbuf.writeUInt8(255);

    const filters: MemcmpFilter[] = [
      {
        memcmp: {
          bytes: group.publicKey.toBase58(),
          offset: 8,
        },
      },
    ];

    if (baseTokenIndex) {
      const bbuf = Buffer.alloc(2);
      bbuf.writeUInt16LE(baseTokenIndex);
      filters.push({
        memcmp: {
          bytes: bs58.encode(bbuf),
          offset: 40,
        },
      });
    }

    if (quoteTokenIndex) {
      const qbuf = Buffer.alloc(2);
      qbuf.writeUInt16LE(quoteTokenIndex);
      filters.push({
        memcmp: {
          bytes: bs58.encode(qbuf),
          offset: 42,
        },
      });
    }

    return (await this.program.account.serum3Market.all(filters)).map((tuple) =>
      Serum3Market.from(tuple.publicKey, tuple.account),
    );
  }

  public async serum3CreateOpenOrders(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<TransactionSignature> {
    const serum3Market: Serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    return await this.program.methods
      .serum3CreateOpenOrders()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        serumMarket: serum3Market.publicKey,
        serumProgram: serum3Market.serumProgram,
        serumMarketExternal: serum3Market.serumMarketExternal,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async serum3CloseOpenOrders(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const openOrders = mangoAccount.serum3.find(
      (account) => account.marketIndex === serum3Market.marketIndex,
    )?.openOrders;

    return await this.program.methods
      .serum3CloseOpenOrders()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        serumMarket: serum3Market.publicKey,
        serumProgram: serum3Market.serumProgram,
        serumMarketExternal: serum3Market.serumMarketExternal,
        openOrders,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async serum3PlaceOrderIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    price: number,
    size: number,
    selfTradeBehavior: Serum3SelfTradeBehavior,
    orderType: Serum3OrderType,
    clientOrderId: number,
    limit: number,
  ): Promise<TransactionInstruction> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;
    if (!mangoAccount.getSerum3Account(serum3Market.marketIndex)) {
      await this.serum3CreateOpenOrders(
        group,
        mangoAccount,
        serum3Market.serumMarketExternal,
      );
      await mangoAccount.reload(this);
    }
    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;
    const serum3MarketExternalVaultSigner =
      await generateSerum3MarketExternalVaultSignerAddress(
        this.cluster,
        serum3Market,
        serum3MarketExternal,
      );

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [mangoAccount],
        [],
        [],
      );

    const limitPrice = serum3MarketExternal.priceNumberToLots(price);
    const maxBaseQuantity = serum3MarketExternal.baseSizeNumberToLots(size);
    const maxQuoteQuantity = serum3MarketExternal.decoded.quoteLotSize
      .mul(
        new BN(
          1 + group.getSerum3FeeRates(orderType === Serum3OrderType.postOnly),
        ),
      )
      .mul(
        serum3MarketExternal
          .baseSizeNumberToLots(size)
          .mul(serum3MarketExternal.priceNumberToLots(price)),
      );
    const payerTokenIndex = ((): TokenIndex => {
      if (side == Serum3Side.bid) {
        return serum3Market.quoteTokenIndex;
      } else {
        return serum3Market.baseTokenIndex;
      }
    })();

    const payerBank = group.getFirstBankByTokenIndex(payerTokenIndex);

    const ix = await this.program.methods
      .serum3PlaceOrder(
        side,
        limitPrice,
        maxBaseQuantity,
        maxQuoteQuantity,
        selfTradeBehavior,
        orderType,
        new BN(clientOrderId),
        limit,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        openOrders: mangoAccount.getSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
        marketRequestQueue: serum3MarketExternal.decoded.requestQueue,
        marketBaseVault: serum3MarketExternal.decoded.baseVault,
        marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
        marketVaultSigner: serum3MarketExternalVaultSigner,
        payerBank: payerBank.publicKey,
        payerVault: payerBank.vault,
        payerOracle: payerBank.oracle,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();

    return ix;
  }

  public async serum3PlaceOrder(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    price: number,
    size: number,
    selfTradeBehavior: Serum3SelfTradeBehavior,
    orderType: Serum3OrderType,
    clientOrderId: number,
    limit: number,
  ): Promise<TransactionSignature> {
    const ix = await this.serum3PlaceOrderIx(
      group,
      mangoAccount,
      externalMarketPk,
      side,
      price,
      size,
      selfTradeBehavior,
      orderType,
      clientOrderId,
      limit,
    );

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async serum3CancelAllOrders(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    limit?: number,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;

    const ix = await this.program.methods
      .serum3CancelAllOrders(limit ? limit : 10)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        openOrders: mangoAccount.getSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async serum3SettleFundsIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<TransactionInstruction> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;
    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;
    const serum3MarketExternalVaultSigner =
      await generateSerum3MarketExternalVaultSignerAddress(
        this.cluster,
        serum3Market,
        serum3MarketExternal,
      );

    const ix = await this.program.methods
      .serum3SettleFunds()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        openOrders: mangoAccount.getSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBaseVault: serum3MarketExternal.decoded.baseVault,
        marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
        marketVaultSigner: serum3MarketExternalVaultSigner,
        quoteBank: group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex)
          .publicKey,
        quoteVault: group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex)
          .vault,
        baseBank: group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
          .publicKey,
        baseVault: group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
          .vault,
      })
      .instruction();

    return ix;
  }

  public async serum3SettleFunds(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<TransactionSignature> {
    const ix = await this.serum3SettleFundsIx(
      group,
      mangoAccount,
      externalMarketPk,
    );

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async serum3CancelOrderIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    orderId: BN,
  ): Promise<TransactionInstruction> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;

    const ix = await this.program.methods
      .serum3CancelOrder(side, orderId)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        openOrders: mangoAccount.getSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .instruction();

    return ix;
  }

  public async serum3CancelOrder(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    orderId: BN,
  ): Promise<TransactionSignature> {
    const ixes = await Promise.all([
      this.serum3CancelOrderIx(
        group,
        mangoAccount,
        externalMarketPk,
        side,
        orderId,
      ),
      this.serum3SettleFundsIx(group, mangoAccount, externalMarketPk),
    ]);

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      ixes,
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  /// perps

  public async perpCreateMarket(
    group: Group,
    oraclePk: PublicKey,
    perpMarketIndex: number,
    name: string,
    oracleConfig: OracleConfigParams,
    baseDecimals: number,
    quoteLotSize: number,
    baseLotSize: number,
    maintAssetWeight: number,
    initAssetWeight: number,
    maintLiabWeight: number,
    initLiabWeight: number,
    liquidationFee: number,
    makerFee: number,
    takerFee: number,
    feePenalty: number,
    minFunding: number,
    maxFunding: number,
    impactQuantity: number,
    groupInsuranceFund: boolean,
    trustedMarket: boolean,
    settleFeeFlat: number,
    settleFeeAmountThreshold: number,
    settleFeeFractionLowHealth: number,
    settleTokenIndex: number,
    settlePnlLimitFactor: number,
    settlePnlLimitWindowSize: number,
  ): Promise<TransactionSignature> {
    const bids = new Keypair();
    const asks = new Keypair();
    const eventQueue = new Keypair();

    const bookSideSize = (this.program as any)._coder.accounts.size(
      (this.program.account.bookSide as any)._idlAccount,
    );
    const eventQueueSize = (this.program as any)._coder.accounts.size(
      (this.program.account.eventQueue as any)._idlAccount,
    );

    return await this.program.methods
      .perpCreateMarket(
        perpMarketIndex,
        name,
        oracleConfig,
        baseDecimals,
        new BN(quoteLotSize),
        new BN(baseLotSize),
        maintAssetWeight,
        initAssetWeight,
        maintLiabWeight,
        initLiabWeight,
        liquidationFee,
        makerFee,
        takerFee,
        minFunding,
        maxFunding,
        new BN(impactQuantity),
        groupInsuranceFund,
        trustedMarket,
        feePenalty,
        settleFeeFlat,
        settleFeeAmountThreshold,
        settleFeeFractionLowHealth,
        settleTokenIndex,
        settlePnlLimitFactor,
        new BN(settlePnlLimitWindowSize),
      )
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        oracle: oraclePk,
        bids: bids.publicKey,
        asks: asks.publicKey,
        eventQueue: eventQueue.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .preInstructions([
        // book sides
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: bookSideSize,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              bookSideSize,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: bids.publicKey,
        }),
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: bookSideSize,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              bookSideSize,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: asks.publicKey,
        }),
        // event queue
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: eventQueueSize,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              eventQueueSize,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: eventQueue.publicKey,
        }),
      ])
      .signers([bids, asks, eventQueue])
      .rpc();
  }

  public async perpEditMarket(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    oracle: PublicKey | null, // TODO: stable price resetting should be a separate flag
    oracleConfig: OracleConfigParams | null,
    baseDecimals: number | null,
    maintAssetWeight: number | null,
    initAssetWeight: number | null,
    maintLiabWeight: number | null,
    initLiabWeight: number | null,
    liquidationFee: number | null,
    makerFee: number | null,
    takerFee: number | null,
    feePenalty: number | null,
    minFunding: number | null,
    maxFunding: number | null,
    impactQuantity: number | null,
    groupInsuranceFund: boolean | null,
    trustedMarket: boolean | null,
    settleFeeFlat: number | null,
    settleFeeAmountThreshold: number | null,
    settleFeeFractionLowHealth: number | null,
    stablePriceDelayIntervalSeconds: number | null,
    stablePriceDelayGrowthLimit: number | null,
    stablePriceGrowthLimit: number | null,
    settlePnlLimitFactor: number | null,
    settlePnlLimitWindowSize: number | null,
  ): Promise<TransactionSignature> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);

    return await this.program.methods
      .perpEditMarket(
        oracle,
        oracleConfig,
        baseDecimals,
        maintAssetWeight,
        initAssetWeight,
        maintLiabWeight,
        initLiabWeight,
        liquidationFee,
        makerFee,
        takerFee,
        minFunding,
        maxFunding,
        impactQuantity !== null ? new BN(impactQuantity) : null,
        groupInsuranceFund,
        trustedMarket,
        feePenalty,
        settleFeeFlat,
        settleFeeAmountThreshold,
        settleFeeFractionLowHealth,
        stablePriceDelayIntervalSeconds,
        stablePriceDelayGrowthLimit,
        stablePriceGrowthLimit,
        settlePnlLimitFactor,
        settlePnlLimitWindowSize !== null
          ? new BN(settlePnlLimitWindowSize)
          : null,
      )
      .accounts({
        group: group.publicKey,
        oracle: oracle ?? perpMarket.oracle,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
      })
      .rpc();
  }

  public async perpCloseMarket(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<TransactionSignature> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);

    return await this.program.methods
      .perpCloseMarket()
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
        eventQueue: perpMarket.eventQueue,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async perpGetMarkets(group: Group): Promise<PerpMarket[]> {
    const bumpfbuf = Buffer.alloc(1);
    bumpfbuf.writeUInt8(255);

    const filters: MemcmpFilter[] = [
      {
        memcmp: {
          bytes: group.publicKey.toBase58(),
          offset: 8,
        },
      },
    ];

    return (await this.program.account.perpMarket.all(filters)).map((tuple) =>
      PerpMarket.from(tuple.publicKey, tuple.account),
    );
  }

  public async perpDeactivatePosition(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<TransactionSignature> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [mangoAccount],
        [],
        [],
      );
    return await this.program.methods
      .perpDeactivatePosition()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .rpc();
  }

  public async perpPlaceOrder(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    price: number,
    quantity: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<TransactionSignature> {
    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [
        await this.perpPlaceOrderIx(
          group,
          mangoAccount,
          perpMarketIndex,
          side,
          price,
          quantity,
          maxQuoteQuantity,
          clientOrderId,
          orderType,
          reduceOnly,
          expiryTimestamp,
          limit,
        ),
      ],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async perpPlaceOrderIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    price: number,
    quantity: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [mangoAccount],
        // Settlement token bank, because a position for it may be created
        [group.getFirstBankByTokenIndex(0 as TokenIndex)],
        [perpMarket],
      );
    return await this.program.methods
      .perpPlaceOrder(
        side,
        perpMarket.uiPriceToLots(price),
        perpMarket.uiBaseToLots(quantity),
        maxQuoteQuantity
          ? perpMarket.uiQuoteToLots(maxQuoteQuantity)
          : I64_MAX_BN,
        new BN(clientOrderId ? clientOrderId : Date.now()),
        orderType ? orderType : PerpOrderType.limit,
        reduceOnly ? reduceOnly : false,
        new BN(expiryTimestamp ? expiryTimestamp : 0),
        limit ? limit : 10,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
        eventQueue: perpMarket.eventQueue,
        oracle: perpMarket.oracle,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();
  }

  public async perpPlaceOrderPegged(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    priceOffset: number,
    pegLimit: number,
    quantity: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<TransactionSignature> {
    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [
        await this.perpPlaceOrderPeggedIx(
          group,
          mangoAccount,
          perpMarketIndex,
          side,
          priceOffset,
          pegLimit,
          quantity,
          maxQuoteQuantity,
          clientOrderId,
          orderType,
          reduceOnly,
          expiryTimestamp,
          limit,
        ),
      ],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async perpPlaceOrderPeggedIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    priceOffset: number,
    pegLimit: number,
    quantity: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [mangoAccount],
        // Settlement token bank, because a position for it may be created
        [group.getFirstBankByTokenIndex(0 as TokenIndex)],
        [perpMarket],
      );
    return await this.program.methods
      .perpPlaceOrderPegged(
        side,
        perpMarket.uiPriceToLots(priceOffset),
        perpMarket.uiPriceToLots(pegLimit),
        perpMarket.uiBaseToLots(quantity),
        maxQuoteQuantity
          ? perpMarket.uiQuoteToLots(maxQuoteQuantity)
          : I64_MAX_BN,
        new BN(clientOrderId ?? Date.now()),
        orderType ? orderType : PerpOrderType.limit,
        reduceOnly ? reduceOnly : false,
        new BN(expiryTimestamp ?? 0),
        limit ? limit : 10,
        -1,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
        eventQueue: perpMarket.eventQueue,
        oracle: perpMarket.oracle,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();
  }

  public async perpCancelOrderIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    orderId: BN,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    return await this.program.methods
      .perpCancelOrder(new BN(orderId))
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
      })
      .instruction();
  }

  public async perpCancelOrder(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    orderId: BN,
  ): Promise<TransactionSignature> {
    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [
        await this.perpCancelOrderIx(
          group,
          mangoAccount,
          perpMarketIndex,
          orderId,
        ),
      ],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async perpCancelAllOrders(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    limit: number,
  ): Promise<TransactionSignature> {
    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [
        await this.perpCancelAllOrdersIx(
          group,
          mangoAccount,
          perpMarketIndex,
          limit,
        ),
      ],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async perpCancelAllOrdersIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    limit: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    return await this.program.methods
      .perpCancelAllOrders(limit)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
  }

  async perpSettlePnl(
    group: Group,
    profitableAccount: MangoAccount,
    unprofitableAccount: MangoAccount,
    settler: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<TransactionSignature> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Scanning,
        group,
        [profitableAccount, unprofitableAccount],
        [group.getFirstBankByTokenIndex(0 as TokenIndex)],
        [perpMarket],
      );
    const bank = group.banksMapByTokenIndex.get(0 as TokenIndex)![0];
    const ix = await this.program.methods
      .perpSettlePnl()
      .accounts({
        group: group.publicKey,
        accountA: profitableAccount.publicKey,
        accountB: unprofitableAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        oracle: perpMarket.oracle,
        settleOracle: bank.oracle,
        settleBank: bank.publicKey,
        settler: settler.publicKey,
        settlerOwner: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  async perpSettleFees(
    group: Group,
    account: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    maxSettleAmount: BN,
  ): Promise<TransactionSignature> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [account], // Account must be unprofitable
        [group.getFirstBankByTokenIndex(0 as TokenIndex)],
        [perpMarket],
      );
    const bank = group.banksMapByTokenIndex.get(0 as TokenIndex)![0];
    const ix = await this.program.methods
      .perpSettleFees(maxSettleAmount)
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        perpMarket: perpMarket.publicKey,
        oracle: perpMarket.oracle,
        settleOracle: bank.oracle,
        settleBank: bank.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async perpConsumeEvents(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    accounts: PublicKey[],
    limit: number,
  ): Promise<TransactionSignature> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    return await this.program.methods
      .perpConsumeEvents(new BN(limit))
      .accounts({
        group: group.publicKey,
        perpMarket: perpMarket.publicKey,
        eventQueue: perpMarket.eventQueue,
      })
      .remainingAccounts(
        accounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: true, isSigner: false } as AccountMeta),
        ),
      )
      .rpc();
  }

  public async perpConsumeAllEvents(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<void> {
    const limit = 8;
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const eventQueue = await perpMarket.loadEventQueue(this);
    const unconsumedEvents = eventQueue.getUnconsumedEvents();
    while (unconsumedEvents.length > 0) {
      const events = unconsumedEvents.splice(0, limit);
      const accounts = events
        .map((ev) => {
          switch (ev.eventType) {
            case PerpEventQueue.FILL_EVENT_TYPE: {
              const fill = <FillEvent>ev;
              return [fill.maker, fill.taker];
            }
            case PerpEventQueue.OUT_EVENT_TYPE: {
              const out = <OutEvent>ev;
              return [out.owner];
            }
            case PerpEventQueue.LIQUIDATE_EVENT_TYPE:
              return [];
            default:
              throw new Error(`Unknown event with eventType ${ev.eventType}!`);
          }
        })
        .flat();

      await this.perpConsumeEvents(group, perpMarketIndex, accounts, limit);
    }
  }

  public async marginTrade({
    group,
    mangoAccount,
    inputMintPk,
    amountIn,
    outputMintPk,
    userDefinedInstructions,
    // margin trade is a general function
    // set flash_loan_type to FlashLoanType.swap if you desire the transaction to be recorded as a swap
    flashLoanType,
  }: {
    group: Group;
    mangoAccount: MangoAccount;
    inputMintPk: PublicKey;
    amountIn: number;
    outputMintPk: PublicKey;
    userDefinedInstructions: TransactionInstruction[];
    flashLoanType: FlashLoanType;
  }): Promise<TransactionSignature> {
    const inputBank: Bank = group.getFirstBankByMint(inputMintPk);
    const outputBank: Bank = group.getFirstBankByMint(outputMintPk);

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [mangoAccount],
        [inputBank, outputBank],
        [],
      );
    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable: false,
          isSigner: false,
        } as AccountMeta),
    );

    /*
     * Find or create associated token accounts
     */
    const inputTokenAccountPk = await getAssociatedTokenAddress(
      inputBank.mint,
      mangoAccount.owner,
    );
    const inputTokenAccExists =
      await this.program.provider.connection.getAccountInfo(
        inputTokenAccountPk,
      );
    const preInstructions: TransactionInstruction[] = [];
    if (!inputTokenAccExists) {
      preInstructions.push(
        await createAssociatedTokenAccountIdempotentInstruction(
          mangoAccount.owner,
          mangoAccount.owner,
          inputBank.mint,
        ),
      );
    }

    const outputTokenAccountPk = await getAssociatedTokenAddress(
      outputBank.mint,
      mangoAccount.owner,
    );
    const outputTokenAccExists =
      await this.program.provider.connection.getAccountInfo(
        outputTokenAccountPk,
      );
    if (!outputTokenAccExists) {
      preInstructions.push(
        await createAssociatedTokenAccountIdempotentInstruction(
          mangoAccount.owner,
          mangoAccount.owner,
          outputBank.mint,
        ),
      );
    }

    const inputBankAccount = {
      pubkey: inputBank.publicKey,
      isWritable: true,
      isSigner: false,
    };
    const outputBankAccount = {
      pubkey: outputBank.publicKey,
      isWritable: true,
      isSigner: false,
    };
    const inputBankVault = {
      pubkey: inputBank.vault,
      isWritable: true,
      isSigner: false,
    };
    const outputBankVault = {
      pubkey: outputBank.vault,
      isWritable: true,
      isSigner: false,
    };
    const inputATA = {
      pubkey: inputTokenAccountPk,
      isWritable: true,
      isSigner: false,
    };
    const outputATA = {
      pubkey: outputTokenAccountPk,
      isWritable: false,
      isSigner: false,
    };
    const groupAM = {
      pubkey: group.publicKey,
      isWritable: false,
      isSigner: false,
    };

    const flashLoanEndIx = await this.program.methods
      .flashLoanEnd(flashLoanType)
      .accounts({
        account: mangoAccount.publicKey,
      })
      .remainingAccounts([
        ...parsedHealthAccounts,
        inputBankVault,
        outputBankVault,
        inputATA,
        {
          isWritable: true,
          pubkey: outputTokenAccountPk,
          isSigner: false,
        },
        groupAM,
      ])
      .instruction();

    const flashLoanBeginIx = await this.program.methods
      .flashLoanBegin([
        toNative(amountIn, inputBank.mintDecimals),
        new BN(
          0,
        ) /* we don't care about borrowing the target amount, this is just a dummy */,
      ])
      .accounts({
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .remainingAccounts([
        inputBankAccount,
        outputBankAccount,
        inputBankVault,
        outputBankVault,
        inputATA,
        outputATA,
        groupAM,
      ])
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [
        ...preInstructions,
        flashLoanBeginIx,
        ...userDefinedInstructions.filter((ix) => ix.keys.length > 2),
        flashLoanEndIx,
      ],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async updateIndexAndRate(
    group: Group,
    mintPk: PublicKey,
  ): Promise<TransactionSignature> {
    const bank = group.getFirstBankByMint(mintPk);
    const mintInfo = group.mintInfosMapByMint.get(mintPk.toString())!;

    return await this.program.methods
      .tokenUpdateIndexAndRate()
      .accounts({
        group: group.publicKey,
        mintInfo: mintInfo.publicKey,
        oracle: mintInfo.oracle,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .remainingAccounts([
        {
          pubkey: bank.publicKey,
          isWritable: true,
          isSigner: false,
        } as AccountMeta,
      ])
      .rpc();
  }

  /// liquidations

  public async liqTokenWithToken(
    group: Group,
    liqor: MangoAccount,
    liqee: MangoAccount,
    assetMintPk: PublicKey,
    liabMintPk: PublicKey,
    maxLiabTransfer: number,
  ): Promise<TransactionSignature> {
    const assetBank: Bank = group.getFirstBankByMint(assetMintPk);
    const liabBank: Bank = group.getFirstBankByMint(liabMintPk);

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Scanning,
        group,
        [liqor, liqee],
        [assetBank, liabBank],
        [],
      );

    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable:
            pk.equals(assetBank.publicKey) || pk.equals(liabBank.publicKey)
              ? true
              : false,
          isSigner: false,
        } as AccountMeta),
    );

    const ix = await this.program.methods
      .liqTokenWithToken(assetBank.tokenIndex, liabBank.tokenIndex, {
        val: I80F48.fromNumber(maxLiabTransfer).getData(),
      })
      .accounts({
        group: group.publicKey,
        liqor: liqor.publicKey,
        liqee: liqee.publicKey,
        liqorOwner: liqor.owner,
      })
      .remainingAccounts(parsedHealthAccounts)
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async altSet(
    group: Group,
    addressLookupTable: PublicKey,
    index: number,
  ): Promise<TransactionSignature> {
    const ix = await this.program.methods
      .altSet(index)
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        addressLookupTable,
      })
      .instruction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async altExtend(
    group: Group,
    addressLookupTable: PublicKey,
    index: number,
    pks: PublicKey[],
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .altExtend(index, pks)
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
        addressLookupTable,
      })
      .rpc();
  }

  public async healthRegionBeginIx(
    group: Group,
    account: MangoAccount,
    banks: Bank[] = [],
    perpMarkets: PerpMarket[] = [],
  ): Promise<TransactionInstruction> {
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [account],
        [...banks],
        [...perpMarkets],
      );
    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable: false,
          isSigner: false,
        } as AccountMeta),
    );

    return await this.program.methods
      .healthRegionBegin()
      .accounts({
        account: account.publicKey,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .remainingAccounts(parsedHealthAccounts)
      .instruction();
  }

  public async healthRegionEndIx(
    group: Group,
    account: MangoAccount,
    banks: Bank[] = [],
    perpMarkets: PerpMarket[] = [],
  ): Promise<TransactionInstruction> {
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [account],
        [...banks],
        [...perpMarkets],
      );
    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable: false,
          isSigner: false,
        } as AccountMeta),
    );

    return await this.program.methods
      .healthRegionEnd()
      .accounts({ account: account.publicKey })
      .remainingAccounts(parsedHealthAccounts)
      .instruction();
  }

  /// static

  static connect(
    provider: Provider,
    cluster: Cluster,
    programId: PublicKey,
    opts?: MangoClientOptions,
  ): MangoClient {
    const idl = IDL;

    return new MangoClient(
      new Program<MangoV4>(idl as MangoV4, programId, provider),
      programId,
      cluster,
      opts,
    );
  }

  static connectForGroupName(
    provider: Provider,
    groupName: string,
  ): MangoClient {
    const idl = IDL;

    const id = Id.fromIdsByName(groupName);

    return new MangoClient(
      new Program<MangoV4>(
        idl as MangoV4,
        new PublicKey(id.mangoProgramId),
        provider,
      ),
      new PublicKey(id.mangoProgramId),
      id.cluster,
    );
  }

  /// private

  private buildHealthRemainingAccounts(
    retriever: AccountRetriever,
    group: Group,
    mangoAccounts: MangoAccount[],
    banks: Bank[],
    perpMarkets: PerpMarket[],
  ): PublicKey[] {
    if (retriever === AccountRetriever.Fixed) {
      return this.buildFixedAccountRetrieverHealthAccounts(
        group,
        mangoAccounts[0],
        banks,
        perpMarkets,
      );
    } else {
      return this.buildScanningAccountRetrieverHealthAccounts(
        group,
        mangoAccounts,
        banks,
        perpMarkets,
      );
    }
  }

  private buildFixedAccountRetrieverHealthAccounts(
    group: Group,
    mangoAccount: MangoAccount,
    // Banks and perpMarkets for whom positions don't exist on mango account,
    // but user would potentially open new positions.
    banks: Bank[],
    perpMarkets: PerpMarket[],
  ): PublicKey[] {
    const healthRemainingAccounts: PublicKey[] = [];

    const allTokenIndices = mangoAccount.tokens.map(
      (token) => token.tokenIndex,
    );

    if (banks) {
      for (const bank of banks) {
        if (allTokenIndices.indexOf(bank.tokenIndex) < 0) {
          allTokenIndices[
            mangoAccount.tokens.findIndex(
              (token, index) =>
                !token.isActive() &&
                allTokenIndices[index] == TokenPosition.TokenIndexUnset,
            )
          ] = bank.tokenIndex;
        }
      }
    }
    const mintInfos = allTokenIndices
      .filter((index) => index != TokenPosition.TokenIndexUnset)
      .map((tokenIndex) => group.mintInfosMapByTokenIndex.get(tokenIndex)!);

    healthRemainingAccounts.push(
      ...mintInfos.map((mintInfo) => mintInfo.firstBank()),
    );
    healthRemainingAccounts.push(
      ...mintInfos.map((mintInfo) => mintInfo.oracle),
    );

    const allPerpIndices = mangoAccount.perps.map((perp) => perp.marketIndex);

    // insert any extra perp markets in the free perp position slots
    if (perpMarkets) {
      for (const perpMarket of perpMarkets) {
        if (allPerpIndices.indexOf(perpMarket.perpMarketIndex) < 0) {
          allPerpIndices[
            mangoAccount.perps.findIndex(
              (perp, index) =>
                !perp.isActive() &&
                allPerpIndices[index] == PerpPosition.PerpMarketIndexUnset,
            )
          ] = perpMarket.perpMarketIndex;
        }
      }
    }
    const allPerpMarkets = allPerpIndices
      .filter((index) => index != PerpPosition.PerpMarketIndexUnset)
      .map((index) => group.findPerpMarket(index)!);
    healthRemainingAccounts.push(
      ...allPerpMarkets.map((perp) => perp.publicKey),
    );
    healthRemainingAccounts.push(...allPerpMarkets.map((perp) => perp.oracle));

    healthRemainingAccounts.push(
      ...mangoAccount.serum3
        .filter((serum3Account) => serum3Account.marketIndex !== 65535)
        .map((serum3Account) => serum3Account.openOrders),
    );

    // debugHealthAccounts(group, mangoAccount, healthRemainingAccounts);

    return healthRemainingAccounts;
  }

  private buildScanningAccountRetrieverHealthAccounts(
    group: Group,
    mangoAccounts: MangoAccount[],
    banks: Bank[],
    perpMarkets: PerpMarket[],
  ): PublicKey[] {
    const healthRemainingAccounts: PublicKey[] = [];

    let tokenIndices: TokenIndex[] = [];
    for (const mangoAccount of mangoAccounts) {
      tokenIndices.push(
        ...mangoAccount.tokens
          .filter((token) => token.tokenIndex !== 65535)
          .map((token) => token.tokenIndex),
      );
    }
    tokenIndices = [...new Set(tokenIndices)];

    if (banks?.length) {
      for (const bank of banks) {
        tokenIndices.push(bank.tokenIndex);
      }
    }
    const mintInfos = [...new Set(tokenIndices)].map(
      (tokenIndex) => group.mintInfosMapByTokenIndex.get(tokenIndex)!,
    );
    healthRemainingAccounts.push(
      ...mintInfos.map((mintInfo) => mintInfo.firstBank()),
    );
    healthRemainingAccounts.push(
      ...mintInfos.map((mintInfo) => mintInfo.oracle),
    );

    const perpIndices: PerpMarketIndex[] = [];
    for (const mangoAccount of mangoAccounts) {
      perpIndices.push(
        ...mangoAccount.perps
          .filter((perp) => perp.marketIndex !== 65535)
          .map((perp) => perp.marketIndex),
      );
    }
    perpIndices.push(...perpMarkets.map((perp) => perp.perpMarketIndex));

    const allPerpMarkets = [...new Set(perpIndices)].map(
      (marketIndex) => group.findPerpMarket(marketIndex)!,
    );

    // Add perp accounts
    healthRemainingAccounts.push(...allPerpMarkets.map((p) => p.publicKey));
    // Add oracle for each perp
    healthRemainingAccounts.push(...allPerpMarkets.map((p) => p.oracle));

    for (const mangoAccount of mangoAccounts) {
      healthRemainingAccounts.push(
        ...mangoAccount.serum3
          .filter((serum3Account) => serum3Account.marketIndex !== 65535)
          .map((serum3Account) => serum3Account.openOrders),
      );
    }

    return healthRemainingAccounts;
  }

  public async modifyPerpOrder(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    orderId: BN,
    side: PerpOrderSide,
    price: number,
    quantity: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<TransactionSignature> {
    const transactionInstructions: TransactionInstruction[] = [];
    const [cancelOrderIx, placeOrderIx] = await Promise.all([
      this.perpCancelOrderIx(group, mangoAccount, perpMarketIndex, orderId),
      this.perpPlaceOrderIx(
        group,
        mangoAccount,
        perpMarketIndex,
        side,
        price,
        quantity,
        maxQuoteQuantity,
        clientOrderId,
        orderType,
        reduceOnly,
        expiryTimestamp,
        limit,
      ),
    ]);
    transactionInstructions.push(cancelOrderIx, placeOrderIx);

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      transactionInstructions,
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }
  public async modifySerum3Order(
    group: Group,
    orderId: BN,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    price: number,
    size: number,
    selfTradeBehavior: Serum3SelfTradeBehavior,
    orderType: Serum3OrderType,
    clientOrderId: number,
    limit: number,
  ): Promise<TransactionSignature> {
    const transactionInstructions: TransactionInstruction[] = [];
    const [cancelOrderIx, settleIx, placeOrderIx] = await Promise.all([
      this.serum3CancelOrderIx(
        group,
        mangoAccount,
        externalMarketPk,
        side,
        orderId,
      ),
      this.serum3SettleFundsIx(group, mangoAccount, externalMarketPk),
      this.serum3PlaceOrderIx(
        group,
        mangoAccount,
        externalMarketPk,
        side,
        price,
        size,
        selfTradeBehavior,
        orderType,
        clientOrderId,
        limit,
      ),
    ]);
    transactionInstructions.push(cancelOrderIx, settleIx, placeOrderIx);

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      transactionInstructions,
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }
}
