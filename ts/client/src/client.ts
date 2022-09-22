import { AnchorProvider, BN, Program, Provider } from '@project-serum/anchor';
import {
  closeAccount,
  initializeAccount,
  WRAPPED_SOL_MINT,
} from '@project-serum/serum/lib/token-instructions';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  Token,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
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
  Transaction,
  TransactionInstruction,
  TransactionSignature,
} from '@solana/web3.js';
import bs58 from 'bs58';
import { Bank, MintInfo } from './accounts/bank';
import { Group } from './accounts/group';
import { I80F48 } from './accounts/I80F48';
import {
  MangoAccount,
  MangoAccountData,
  TokenPosition,
} from './accounts/mangoAccount';
import { StubOracle } from './accounts/oracle';
import { PerpMarket, PerpOrderType, Side } from './accounts/perp';
import {
  generateSerum3MarketExternalVaultSignerAddress,
  Serum3Market,
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from './accounts/serum3';
import { SERUM3_PROGRAM_ID } from './constants';
import { Id } from './ids';
import { IDL, MangoV4 } from './mango_v4';
import { FlashLoanType, InterestRateParams } from './types';
import {
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddress,
  I64_MAX_BN,
  toNativeDecimals,
} from './utils';
import { simulate } from './utils/anchor';
import { sendTransaction } from './utils/rpc';

enum AccountRetriever {
  Scanning,
  Fixed,
}

export type IdsSource = 'api' | 'static' | 'get-program-accounts';

// TODO: replace ui values with native as input wherever possible
// TODO: replace token/market names with token or market indices
export class MangoClient {
  private postSendTxCallback?: ({ txid }) => void;
  private prioritizationFee: number;

  constructor(
    public program: Program<MangoV4>,
    public programId: PublicKey,
    public cluster: Cluster,
    public opts: {
      postSendTxCallback?: ({ txid }: { txid: string }) => void;
      prioritizationFee?: number;
    } = {},
    public idsSource: IdsSource = 'api',
  ) {
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
    admin: PublicKey | undefined,
    fastListingAdmin: PublicKey | undefined,
    testing: number | undefined,
    version: number | undefined,
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
    await group.reloadAll(this);
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

  // Tokens/Banks

  public async tokenRegister(
    group: Group,
    mintPk: PublicKey,
    oraclePk: PublicKey,
    oracleConfFilter: number,
    tokenIndex: number,
    name: string,
    adjustmentFactor: number,
    util0: number,
    rate0: number,
    util1: number,
    rate1: number,
    maxRate: number,
    loanFeeRate: number,
    loanOriginationFeeRate: number,
    maintAssetWeight: number,
    initAssetWeight: number,
    maintLiabWeight: number,
    initLiabWeight: number,
    liquidationFee: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .tokenRegister(
        tokenIndex,
        name,
        {
          confFilter: {
            val: I80F48.fromNumber(oracleConfFilter).getData(),
          },
        } as any, // future: nested custom types dont typecheck, fix if possible?
        { adjustmentFactor, util0, rate0, util1, rate1, maxRate },
        loanFeeRate,
        loanOriginationFeeRate,
        maintAssetWeight,
        initAssetWeight,
        maintLiabWeight,
        initLiabWeight,
        liquidationFee,
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
    oracleConfFilter: number | null,
    groupInsuranceFund: boolean | null,
    interestRateParams: InterestRateParams | null,
    loanFeeRate: number | null,
    loanOriginationFeeRate: number | null,
    maintAssetWeight: number | null,
    initAssetWeight: number | null,
    maintLiabWeight: number | null,
    initLiabWeight: number | null,
    liquidationFee: number | null,
  ): Promise<TransactionSignature> {
    const bank = group.getFirstBankByMint(mintPk);
    const mintInfo = group.mintInfosMapByTokenIndex.get(bank.tokenIndex)!;

    let oracleConf;
    if (oracleConfFilter !== null) {
      oracleConf = {
        confFilter: {
          val: I80F48.fromNumber(oracleConfFilter).getData(),
        },
      } as any; // future: nested custom types dont typecheck, fix if possible?
    } else {
      oracleConf = null;
    }

    return await this.program.methods
      .tokenEdit(
        oracle,
        oracleConf,
        groupInsuranceFund,
        interestRateParams,
        loanFeeRate,
        loanOriginationFeeRate,
        maintAssetWeight,
        initAssetWeight,
        maintLiabWeight,
        initLiabWeight,
        liquidationFee,
      )
      .accounts({
        group: group.publicKey,
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
      .rpc({ skipPreflight: true });
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
    if (!ai) {
      const tx = new Transaction();
      tx.add(
        Token.createAssociatedTokenAccountInstruction(
          ASSOCIATED_TOKEN_PROGRAM_ID,
          TOKEN_PROGRAM_ID,
          bank.mint,
          dustVaultPk,
          adminPk,
          adminPk,
        ),
      );
      await (this.program.provider as AnchorProvider).sendAndConfirm(tx);
    }

    return await this.program.methods
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
    tokenIndex: number,
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
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
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

  public async getOrCreateMangoAccount(
    group: Group,
    ownerPk: PublicKey,
    accountNumber?: number,
    name?: string,
  ): Promise<MangoAccount | undefined> {
    // TODO: this function discards accountSize and name when the account exists already!
    // TODO: this function always creates accounts for this.program.owner, and not
    //       ownerPk! It needs to get passed a keypair, and we need to add
    //       createMangoAccountForOwner
    if (accountNumber === undefined) {
      // Get any MangoAccount
      // TODO: should probably sort by accountNum for deterministic output!
      let mangoAccounts = await this.getMangoAccountsForOwner(group, ownerPk);
      if (mangoAccounts.length === 0) {
        await this.createMangoAccount(group, accountNumber, name);
        mangoAccounts = await this.getMangoAccountsForOwner(group, ownerPk);
      }
      return mangoAccounts[0];
    } else {
      let account = await this.getMangoAccountForOwner(
        group,
        ownerPk,
        accountNumber,
      );
      if (account === undefined) {
        await this.createMangoAccount(group, accountNumber, name);
        account = await this.getMangoAccountForOwner(
          group,
          ownerPk,
          accountNumber,
        );
      }
      return account;
    }
  }

  public async createMangoAccount(
    group: Group,
    accountNumber?: number,
    name?: string,
  ): Promise<TransactionSignature> {
    const transaction = await this.program.methods
      .accountCreate(accountNumber ?? 0, 8, 8, 0, 0, name ?? '')
      .accounts({
        group: group.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .transaction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      transaction,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
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
    const transaction = await this.program.methods
      .accountEdit(name ?? null, delegate ?? null)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .transaction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      transaction,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async getMangoAccount(mangoAccount: MangoAccount) {
    return MangoAccount.from(
      mangoAccount.publicKey,
      await this.program.account.mangoAccount.fetch(mangoAccount.publicKey),
    );
  }

  public async getMangoAccountWithSlot(mangoAccountPk: PublicKey) {
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
    const transaction = await this.program.methods
      .accountClose()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        solDestination: mangoAccount.owner,
      })
      .transaction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      transaction,
      {
        postSendTxCallback: this.postSendTxCallback,
      },
    );
  }

  public async computeAccountData(
    group: Group,
    mangoAccount: MangoAccount,
  ): Promise<MangoAccountData | undefined> {
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [mangoAccount],
        [],
        [],
      );

    // Use our custom simulate fn in utils/anchor.ts so signing the tx is not required
    this.program.provider.simulate = simulate;

    const res = await this.program.methods
      .computeAccountData()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({
              pubkey: pk,
              isWritable: false,
              isSigner: false,
            } as AccountMeta),
        ),
      )
      .simulate();

    if (res.events) {
      const accountDataEvent = res?.events.find(
        (event) => (event.name = 'MangoAccountData'),
      );
      return accountDataEvent
        ? MangoAccountData.from(accountDataEvent.data as any)
        : undefined;
    } else {
      return undefined;
    }
  }

  public async tokenDeposit(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    amount: number,
  ): Promise<TransactionSignature> {
    const decimals = group.getMintDecimals(mintPk);
    const nativeAmount = toNativeDecimals(amount, decimals).toNumber();
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
    nativeAmount: number,
  ) {
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
      const lamports = nativeAmount + 1e7;

      preInstructions = [
        SystemProgram.createAccount({
          fromPubkey: mangoAccount.owner,
          newAccountPubkey: wrappedSolAccount.publicKey,
          lamports,
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

    const transaction = await this.program.methods
      .tokenDeposit(new BN(nativeAmount))
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
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
      .preInstructions(preInstructions)
      .postInstructions(postInstructions)
      .signers(additionalSigners)
      .transaction();

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      transaction,
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
    const nativeAmount = toNativeDecimals(
      amount,
      group.getMintDecimals(mintPk),
    ).toNumber();
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
    nativeAmount: number,
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

    const tx = await this.program.methods
      .tokenWithdraw(new BN(nativeAmount), allowBorrow)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        oracle: bank.oracle,
        tokenAccount: tokenAccountPk,
        owner: mangoAccount.owner,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .preInstructions(preInstructions)
      .postInstructions(postInstructions)
      .transaction();

    return await sendTransaction(this.program.provider as AnchorProvider, tx, {
      postSendTxCallback: this.postSendTxCallback,
    });
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
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
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
  ) {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;
    if (!mangoAccount.findSerum3Account(serum3Market.marketIndex)) {
      await this.serum3CreateOpenOrders(
        group,
        mangoAccount,
        serum3Market.serumMarketExternal,
      );
      await mangoAccount.reload(this, group);
    }
    const serum3MarketExternal = group.serum3MarketExternalsMap.get(
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
      .mul(new BN(1 + group.getFeeRate(orderType === Serum3OrderType.postOnly)))
      .mul(
        serum3MarketExternal
          .baseSizeNumberToLots(size)
          .mul(serum3MarketExternal.priceNumberToLots(price)),
      );
    const payerTokenIndex = (() => {
      if (side == Serum3Side.bid) {
        return serum3Market.quoteTokenIndex;
      } else {
        return serum3Market.baseTokenIndex;
      }
    })();

    const tx = await this.program.methods
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
        openOrders: mangoAccount.findSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
        marketRequestQueue: serum3MarketExternal.decoded.requestQueue,
        marketBaseVault: serum3MarketExternal.decoded.baseVault,
        marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
        marketVaultSigner: serum3MarketExternalVaultSigner,
        payerBank: group.getFirstBankByTokenIndex(payerTokenIndex).publicKey,
        payerVault: group.getFirstBankByTokenIndex(payerTokenIndex).vault,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .transaction();

    return await sendTransaction(this.program.provider as AnchorProvider, tx, {
      postSendTxCallback: this.postSendTxCallback,
    });
  }

  async serum3CancelAllorders(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    limit: number,
  ) {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const serum3MarketExternal = group.serum3MarketExternalsMap.get(
      externalMarketPk.toBase58(),
    )!;

    const tx = await this.program.methods
      .serum3CancelAllOrders(limit)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        openOrders: mangoAccount.findSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .transaction();

    return await sendTransaction(this.program.provider as AnchorProvider, tx, {
      postSendTxCallback: this.postSendTxCallback,
    });
  }

  async serum3SettleFunds(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;
    const serum3MarketExternal = group.serum3MarketExternalsMap.get(
      externalMarketPk.toBase58(),
    )!;
    const serum3MarketExternalVaultSigner =
      await generateSerum3MarketExternalVaultSignerAddress(
        this.cluster,
        serum3Market,
        serum3MarketExternal,
      );

    const tx = await this.program.methods
      .serum3SettleFunds()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        openOrders: mangoAccount.findSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
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
      .transaction();

    return await sendTransaction(this.program.provider as AnchorProvider, tx, {
      postSendTxCallback: this.postSendTxCallback,
    });
  }

  async serum3CancelOrder(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    orderId: BN,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const serum3MarketExternal = group.serum3MarketExternalsMap.get(
      externalMarketPk.toBase58(),
    )!;

    const tx = await this.program.methods
      .serum3CancelOrder(side, orderId)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        openOrders: mangoAccount.findSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .transaction();

    return await sendTransaction(this.program.provider as AnchorProvider, tx, {
      postSendTxCallback: this.postSendTxCallback,
    });
  }

  /// perps

  async perpCreateMarket(
    group: Group,
    oraclePk: PublicKey,
    perpMarketIndex: number,
    name: string,
    oracleConfFilter: number,
    baseDecimals: number,
    quoteTokenIndex: number,
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
  ): Promise<TransactionSignature> {
    const bids = new Keypair();
    const asks = new Keypair();
    const eventQueue = new Keypair();

    return await this.program.methods
      .perpCreateMarket(
        perpMarketIndex,
        name,
        {
          confFilter: {
            val: I80F48.fromNumber(oracleConfFilter).getData(),
          },
        } as any, // future: nested custom types dont typecheck, fix if possible?
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
        feePenalty
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
        // TODO: try to pick up sizes of bookside and eventqueue from IDL, so we can stay in sync with program

        // book sides
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: 8 + 98584,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              8 + 98584,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: bids.publicKey,
        }),
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: 8 + 98584,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              8 + 98584,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: asks.publicKey,
        }),
        // event queue
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: 8 + 4 * 2 + 8 + 488 * 208,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              8 + 4 * 2 + 8 + 488 * 208,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: eventQueue.publicKey,
        }),
      ])
      .signers([bids, asks, eventQueue])
      .rpc();
  }

  async perpEditMarket(
    group: Group,
    perpMarketName: string,
    oracle: PublicKey,
    oracleConfFilter: number,
    baseDecimals: number,
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
  ): Promise<TransactionSignature> {
    const perpMarket = group.perpMarketsMap.get(perpMarketName)!;

    return await this.program.methods
      .perpEditMarket(
        oracle,
        {
          confFilter: {
            val: I80F48.fromNumber(oracleConfFilter).getData(),
          },
        } as any, // future: nested custom types dont typecheck, fix if possible?
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
        new BN(impactQuantity),
        groupInsuranceFund,
        trustedMarket,
        feePenalty
      )
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
      })
      .rpc();
  }

  async perpCloseMarket(
    group: Group,
    perpMarketName: string,
  ): Promise<TransactionSignature> {
    const perpMarket = group.perpMarketsMap.get(perpMarketName)!;

    return await this.program.methods
      .perpCloseMarket()
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
        asks: perpMarket.asks,
        bids: perpMarket.bids,
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

  async perpPlaceOrder(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketName: string,
    side: Side,
    price: number,
    quantity: number,
    maxQuoteQuantity: number,
    clientOrderId: number,
    orderType: PerpOrderType,
    expiryTimestamp: number,
    limit: number,
  ): Promise<TransactionSignature> {
    const perpMarket = group.perpMarketsMap.get(perpMarketName)!;
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        AccountRetriever.Fixed,
        group,
        [mangoAccount],
        [],
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
        new BN(clientOrderId),
        orderType,
        new BN(expiryTimestamp),
        limit,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        asks: perpMarket.asks,
        bids: perpMarket.bids,
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
      .rpc();
  }

  async perpCancelAllOrders(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketName: string,
    limit: number,
  ): Promise<TransactionSignature> {
    const perpMarket = group.perpMarketsMap.get(perpMarketName)!;
    return await this.program.methods
      .perpCancelAllOrders(limit)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        asks: perpMarket.asks,
        bids: perpMarket.bids,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
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

    if (!inputBank || !outputBank) throw new Error('Invalid token');

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
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
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
        toNativeDecimals(amountIn, inputBank.mintDecimals),
        new BN(
          0,
        ) /* we don't care about borrowing the target amount, this is just a dummy */,
      ])
      .accounts({
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

    const tx = new Transaction();
    for (const ix of preInstructions) {
      tx.add(ix);
    }
    tx.add(flashLoanBeginIx);
    for (const ix of userDefinedInstructions.filter(
      (ix) => ix.keys.length > 2,
    )) {
      tx.add(ix);
    }
    tx.add(flashLoanEndIx);

    return await sendTransaction(this.program.provider as AnchorProvider, tx, {
      postSendTxCallback: this.postSendTxCallback,
    });
  }

  async updateIndexAndRate(group: Group, mintPk: PublicKey) {
    // TODO: handle updating multiple banks
    const bank = group.getFirstBankByMint(mintPk);
    const mintInfo = group.mintInfosMapByMint.get(mintPk.toString())!;

    await this.program.methods
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

  async liqTokenWithToken(
    group: Group,
    liqor: MangoAccount,
    liqee: MangoAccount,
    assetMintPk: PublicKey,
    liabMintPk: PublicKey,
    maxLiabTransfer: number,
  ) {
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

    await this.program.methods
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
      .rpc();
  }

  /// static

  static connect(
    provider: Provider,
    cluster: Cluster,
    programId: PublicKey,
    opts: any = {},
    getIdsFromApi: IdsSource = 'api',
  ): MangoClient {
    // TODO: use IDL on chain or in repository? decide...
    // Alternatively we could fetch IDL from chain.
    // const idl = await Program.fetchIdl(MANGO_V4_ID, provider);
    const idl = IDL;

    return new MangoClient(
      new Program<MangoV4>(idl as MangoV4, programId, provider),
      programId,
      cluster,
      opts,
      getIdsFromApi,
    );
  }

  static connectForGroupName(
    provider: Provider,
    groupName: string,
  ): MangoClient {
    // TODO: use IDL on chain or in repository? decide...
    // Alternatively we could fetch IDL from chain.
    // const idl = await Program.fetchIdl(MANGO_V4_ID, provider);
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

  // todo make private
  public buildHealthRemainingAccounts(
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

  // todo make private
  public buildFixedAccountRetrieverHealthAccounts(
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

    healthRemainingAccounts.push(
      ...mangoAccount.perps
        .filter((perp) => perp.marketIndex !== 65535)
        .map(
          (perp) =>
            Array.from(group.perpMarketsMap.values()).filter(
              (perpMarket) => perpMarket.perpMarketIndex === perp.marketIndex,
            )[0].publicKey,
        ),
    );

    healthRemainingAccounts.push(
      ...mangoAccount.perps
        .filter((perp) => perp.marketIndex !== 65535)
        .map(
          (perp) =>
            Array.from(group.perpMarketsMap.values()).filter(
              (perpMarket) => perpMarket.perpMarketIndex === perp.marketIndex,
            )[0].oracle,
        ),
    );

    for (const perpMarket of perpMarkets) {
      const alreadyAdded = mangoAccount.perps.find(
        (p) => p.marketIndex === perpMarket.perpMarketIndex,
      );
      if (!alreadyAdded) {
        healthRemainingAccounts.push(
          Array.from(group.perpMarketsMap.values()).filter(
            (p) => p.perpMarketIndex === perpMarket.perpMarketIndex,
          )[0].publicKey,
        );
      }
    }

    healthRemainingAccounts.push(
      ...mangoAccount.serum3
        .filter((serum3Account) => serum3Account.marketIndex !== 65535)
        .map((serum3Account) => serum3Account.openOrders),
    );

    // debugHealthAccounts(group, mangoAccount, healthRemainingAccounts);

    return healthRemainingAccounts;
  }

  // todo make private
  public buildScanningAccountRetrieverHealthAccounts(
    group: Group,
    mangoAccounts: MangoAccount[],
    banks: Bank[],
    perpMarkets: PerpMarket[],
  ): PublicKey[] {
    const healthRemainingAccounts: PublicKey[] = [];

    let tokenIndices: number[] = [];
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

    const perpsToAdd: PerpMarket[] = [];

    for (const mangoAccount of mangoAccounts) {
      perpsToAdd.push(
        ...mangoAccount.perps
          .filter((perp) => perp.marketIndex !== 65535)
          .map(
            (perp) =>
              Array.from(group.perpMarketsMap.values()).filter(
                (perpMarket) => perpMarket.perpMarketIndex === perp.marketIndex,
              )[0],
          ),
      );
    }
    for (const mangoAccount of mangoAccounts) {
      for (const perpMarket of perpMarkets) {
        const alreadyAdded = mangoAccount.perps.find(
          (p) => p.marketIndex === perpMarket.perpMarketIndex,
        );
        if (!alreadyAdded) {
          perpsToAdd.push(
            Array.from(group.perpMarketsMap.values()).filter(
              (p) => p.perpMarketIndex === perpMarket.perpMarketIndex,
            )[0],
          );
        }
      }
    }

    // Add perp accounts
    healthRemainingAccounts.push(...perpsToAdd.map((p) => p.publicKey));
    // Add oracle for each perp
    healthRemainingAccounts.push(...perpsToAdd.map((p) => p.oracle));

    for (const mangoAccount of mangoAccounts) {
      healthRemainingAccounts.push(
        ...mangoAccount.serum3
          .filter((serum3Account) => serum3Account.marketIndex !== 65535)
          .map((serum3Account) => serum3Account.openOrders),
      );
    }

    return healthRemainingAccounts;
  }
}
