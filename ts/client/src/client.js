import { BN, Program } from '@coral-xyz/anchor';
import { createCloseAccountInstruction, createInitializeAccount3Instruction, } from '@solana/spl-token';
import { Keypair, PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY, SYSVAR_RENT_PUBKEY, SystemProgram, } from '@solana/web3.js';
import bs58 from 'bs58';
import { Bank, MintInfo } from './accounts/bank';
import { Group } from './accounts/group';
import { MangoAccount, PerpPosition, Serum3Orders, TokenPosition, } from './accounts/mangoAccount';
import { StubOracle } from './accounts/oracle';
import { PerpEventQueue, PerpMarket, PerpOrderType, } from './accounts/perp';
import { Serum3Market, Serum3OrderType, Serum3Side, generateSerum3MarketExternalVaultSignerAddress, } from './accounts/serum3';
import { buildIxGate, } from './clientIxParamBuilder';
import { OPENBOOK_PROGRAM_ID } from './constants';
import { Id } from './ids';
import { IDL } from './mango_v4';
import { I80F48 } from './numbers/I80F48';
import { I64_MAX_BN, U64_MAX_BN, createAssociatedTokenAccountIdempotentInstruction, getAssociatedTokenAddress, toNative, } from './utils';
import { sendTransaction } from './utils/rpc';
import { NATIVE_MINT, TOKEN_PROGRAM_ID } from './utils/spl';
export var AccountRetriever;
(function (AccountRetriever) {
    AccountRetriever[AccountRetriever["Scanning"] = 0] = "Scanning";
    AccountRetriever[AccountRetriever["Fixed"] = 1] = "Fixed";
})(AccountRetriever || (AccountRetriever = {}));
export class MangoClient {
    program;
    programId;
    cluster;
    opts;
    idsSource;
    postSendTxCallback;
    prioritizationFee;
    txConfirmationCommitment;
    openbookFeesToDao;
    constructor(program, programId, cluster, opts = {}) {
        this.program = program;
        this.programId = programId;
        this.cluster = cluster;
        this.opts = opts;
        this.idsSource = opts?.idsSource || 'get-program-accounts';
        this.prioritizationFee = opts?.prioritizationFee || 0;
        this.postSendTxCallback = opts?.postSendTxCallback;
        this.openbookFeesToDao = opts?.openbookFeesToDao ?? true;
        this.txConfirmationCommitment =
            opts?.txConfirmationCommitment ??
                program.provider.opts.commitment ??
                'processed';
        // TODO: evil side effect, but limited backtraces are a nightmare
        Error.stackTraceLimit = 1000;
    }
    /// Transactions
    async sendAndConfirmTransaction(ixs, opts = {}) {
        return await sendTransaction(this.program.provider, ixs, opts.alts ?? [], {
            postSendTxCallback: this.postSendTxCallback,
            prioritizationFee: this.prioritizationFee,
            txConfirmationCommitment: this.txConfirmationCommitment,
            ...opts,
        });
    }
    async sendAndConfirmTransactionForGroup(group, ixs, opts = {}) {
        return await this.sendAndConfirmTransaction(ixs, {
            alts: group.addressLookupTablesList,
            ...opts,
        });
    }
    // Group
    async groupCreate(groupNum, testing, version, insuranceMintPk) {
        const adminPk = this.program.provider.wallet.publicKey;
        const ix = await this.program.methods
            .groupCreate(groupNum, testing ? 1 : 0, version)
            .accounts({
            creator: adminPk,
            payer: adminPk,
            insuranceMint: insuranceMintPk,
        })
            .instruction();
        return await this.sendAndConfirmTransaction([ix]);
    }
    async groupEdit(group, admin, fastListingAdmin, securityAdmin, testing, version, depositLimitQuote, feesPayWithMngo, feesMngoBonusRate, feesSwapMangoAccount, feesMngoTokenIndex, feesExpiryInterval) {
        const ix = await this.program.methods
            .groupEdit(admin ?? null, fastListingAdmin ?? null, securityAdmin ?? null, testing ?? null, version ?? null, depositLimitQuote !== undefined ? depositLimitQuote : null, feesPayWithMngo ?? null, feesMngoBonusRate ?? null, feesSwapMangoAccount ?? null, feesMngoTokenIndex ?? null, feesExpiryInterval ?? null)
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async ixGateSet(group, ixGateParams) {
        const ix = await this.program.methods
            .ixGateSet(buildIxGate(ixGateParams))
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async groupClose(group) {
        const adminPk = this.program.provider.wallet.publicKey;
        const ix = await this.program.methods
            .groupClose()
            .accounts({
            group: group.publicKey,
            insuranceVault: group.insuranceVault,
            admin: adminPk,
            solDestination: this.program.provider.wallet
                .publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async getGroup(groupPk) {
        const groupAccount = await this.program.account.group.fetch(groupPk);
        const group = Group.from(groupPk, groupAccount);
        await group.reloadAll(this);
        return group;
    }
    async getGroupsForCreator(creatorPk) {
        const filters = [
            {
                memcmp: {
                    bytes: creatorPk.toBase58(),
                    offset: 8,
                },
            },
        ];
        const groups = (await this.program.account.group.all(filters)).map((tuple) => Group.from(tuple.publicKey, tuple.account));
        groups.forEach((group) => group.reloadAll(this));
        return groups;
    }
    async getGroupForCreator(creatorPk, groupNum) {
        const bbuf = Buffer.alloc(4);
        bbuf.writeUInt32LE(groupNum);
        const filters = [
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
        const groups = (await this.program.account.group.all(filters)).map((tuple) => Group.from(tuple.publicKey, tuple.account));
        await groups[0].reloadAll(this);
        return groups[0];
    }
    async getIds(groupPk) {
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
    async tokenRegister(group, mintPk, oraclePk, oracleConfig, tokenIndex, name, interestRateParams, loanFeeRate, loanOriginationFeeRate, maintAssetWeight, initAssetWeight, maintLiabWeight, initLiabWeight, liquidationFee, minVaultToDepositsRatio, netBorrowLimitWindowSizeTs, netBorrowLimitPerWindowQuote) {
        const ix = await this.program.methods
            .tokenRegister(tokenIndex, name, oracleConfig, interestRateParams, loanFeeRate, loanOriginationFeeRate, maintAssetWeight, initAssetWeight, maintLiabWeight, initLiabWeight, liquidationFee, minVaultToDepositsRatio, new BN(netBorrowLimitWindowSizeTs), new BN(netBorrowLimitPerWindowQuote))
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
            mint: mintPk,
            oracle: oraclePk,
            payer: this.program.provider.wallet.publicKey,
            rent: SYSVAR_RENT_PUBKEY,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async tokenRegisterTrustless(group, mintPk, oraclePk, tokenIndex, name) {
        const ix = await this.program.methods
            .tokenRegisterTrustless(tokenIndex, name)
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
            mint: mintPk,
            oracle: oraclePk,
            payer: this.program.provider.wallet.publicKey,
            rent: SYSVAR_RENT_PUBKEY,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async tokenEdit(group, mintPk, params) {
        const bank = group.getFirstBankByMint(mintPk);
        const mintInfo = group.mintInfosMapByTokenIndex.get(bank.tokenIndex);
        const ix = await this.program.methods
            .tokenEdit(params.oracle, params.oracleConfig, params.groupInsuranceFund, params.interestRateParams, params.loanFeeRate, params.loanOriginationFeeRate, params.maintAssetWeight, params.initAssetWeight, params.maintLiabWeight, params.initLiabWeight, params.liquidationFee, params.stablePriceDelayIntervalSeconds, params.stablePriceDelayGrowthLimit, params.stablePriceGrowthLimit, params.minVaultToDepositsRatio, params.netBorrowLimitPerWindowQuote !== null
            ? new BN(params.netBorrowLimitPerWindowQuote)
            : null, params.netBorrowLimitWindowSizeTs !== null
            ? new BN(params.netBorrowLimitWindowSizeTs)
            : null, params.borrowWeightScaleStartQuote, params.depositWeightScaleStartQuote, params.resetStablePrice ?? false, params.resetNetBorrowLimit ?? false, params.reduceOnly, params.name)
            .accounts({
            group: group.publicKey,
            oracle: params.oracle ?? bank.oracle,
            admin: this.program.provider.wallet.publicKey,
            mintInfo: mintInfo.publicKey,
        })
            .remainingAccounts([
            {
                pubkey: bank.publicKey,
                isWritable: true,
                isSigner: false,
            },
        ])
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async tokenDeregister(group, mintPk) {
        const bank = group.getFirstBankByMint(mintPk);
        const adminPk = this.program.provider.wallet.publicKey;
        const dustVaultPk = await getAssociatedTokenAddress(bank.mint, adminPk);
        const ai = await this.program.provider.connection.getAccountInfo(dustVaultPk);
        const preInstructions = [];
        if (!ai) {
            preInstructions.push(await createAssociatedTokenAccountIdempotentInstruction(adminPk, adminPk, bank.mint));
        }
        const ix = await this.program.methods
            .tokenDeregister()
            .accounts({
            group: group.publicKey,
            admin: adminPk,
            mintInfo: group.mintInfosMapByTokenIndex.get(bank.tokenIndex)
                ?.publicKey,
            dustVault: dustVaultPk,
            solDestination: this.program.provider.wallet
                .publicKey,
        })
            .remainingAccounts([bank.publicKey, bank.vault].map((pk) => ({ pubkey: pk, isWritable: true, isSigner: false })))
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [
            ...preInstructions,
            ix,
        ]);
    }
    async getBanksForGroup(group) {
        return (await this.program.account.bank.all([
            {
                memcmp: {
                    bytes: group.publicKey.toBase58(),
                    offset: 8,
                },
            },
        ])).map((tuple) => Bank.from(tuple.publicKey, tuple.account));
    }
    async getMintInfosForGroup(group) {
        return (await this.program.account.mintInfo.all([
            {
                memcmp: {
                    bytes: group.publicKey.toBase58(),
                    offset: 8,
                },
            },
        ])).map((tuple) => {
            return MintInfo.from(tuple.publicKey, tuple.account);
        });
    }
    async getMintInfoForTokenIndex(group, tokenIndex) {
        const tokenIndexBuf = Buffer.alloc(2);
        tokenIndexBuf.writeUInt16LE(tokenIndex);
        return (await this.program.account.mintInfo.all([
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
        ])).map((tuple) => {
            return MintInfo.from(tuple.publicKey, tuple.account);
        });
    }
    // Stub Oracle
    async stubOracleCreate(group, mintPk, price) {
        const ix = await this.program.methods
            .stubOracleCreate({ val: I80F48.fromNumber(price).getData() })
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
            mint: mintPk,
            payer: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async stubOracleClose(group, oracle) {
        const ix = await this.program.methods
            .stubOracleClose()
            .accounts({
            group: group.publicKey,
            oracle: oracle,
            solDestination: this.program.provider.wallet
                .publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async stubOracleSet(group, oraclePk, price) {
        const ix = await this.program.methods
            .stubOracleSet({ val: I80F48.fromNumber(price).getData() })
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
            oracle: oraclePk,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async getStubOracle(group, mintPk) {
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
        return (await this.program.account.stubOracle.all(filters)).map((pa) => StubOracle.from(pa.publicKey, pa.account));
    }
    // MangoAccount
    async getOrCreateMangoAccount(group, loadSerum3Oo = false) {
        const clientOwner = this.program.provider.wallet
            .publicKey;
        let mangoAccounts = await this.getMangoAccountsForOwner(group, this.program.provider.wallet.publicKey, loadSerum3Oo);
        if (mangoAccounts.length === 0) {
            await this.createMangoAccount(group);
            mangoAccounts = await this.getMangoAccountsForOwner(group, clientOwner, loadSerum3Oo);
        }
        return mangoAccounts.sort((a, b) => a.accountNum - b.accountNum)[0];
    }
    async createMangoAccount(group, accountNumber, name, tokenCount, serum3Count, perpCount, perpOoCount) {
        const ix = await this.program.methods
            .accountCreate(accountNumber ?? 0, tokenCount ?? 8, serum3Count ?? 8, perpCount ?? 8, perpOoCount ?? 32, name ?? '')
            .accounts({
            group: group.publicKey,
            owner: this.program.provider.wallet.publicKey,
            payer: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async createAndFetchMangoAccount(group, accountNumber, name, tokenCount, serum3Count, perpCount, perpOoCount, loadSerum3Oo = false) {
        const accNum = accountNumber ?? 0;
        await this.createMangoAccount(group, accNum, name, tokenCount, serum3Count, perpCount, perpOoCount);
        return await this.getMangoAccountForOwner(group, this.program.provider.wallet.publicKey, accNum, loadSerum3Oo);
    }
    async expandMangoAccount(group, account, tokenCount, serum3Count, perpCount, perpOoCount) {
        const ix = await this.program.methods
            .accountExpand(tokenCount, serum3Count, perpCount, perpOoCount)
            .accounts({
            group: group.publicKey,
            account: account.publicKey,
            owner: this.program.provider.wallet.publicKey,
            payer: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async editMangoAccount(group, mangoAccount, name, delegate) {
        const ix = await this.program.methods
            .accountEdit(name ?? null, delegate ?? null)
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            owner: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async computeAccountData(group, mangoAccount) {
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [mangoAccount], [], []);
        const ix = await this.program.methods
            .computeAccountData()
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
        })
            .remainingAccounts(healthRemainingAccounts.map((pk) => ({ pubkey: pk, isWritable: false, isSigner: false })))
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async toggleMangoAccountFreeze(group, mangoAccount, freeze) {
        const ix = await this.program.methods
            .accountToggleFreeze(freeze)
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            admin: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async getMangoAccount(mangoAccount, loadSerum3Oo = false) {
        const mangoAccountPk = mangoAccount instanceof MangoAccount
            ? mangoAccount.publicKey
            : mangoAccount;
        const mangoAccount_ = MangoAccount.from(mangoAccountPk, await this.program.account.mangoAccount.fetch(mangoAccountPk));
        if (loadSerum3Oo) {
            await mangoAccount_?.reloadSerum3OpenOrders(this);
        }
        return mangoAccount_;
    }
    async getMangoAccountWithSlot(mangoAccountPk, loadSerum3Oo = false) {
        const resp = await this.program.provider.connection.getAccountInfoAndContext(mangoAccountPk);
        if (!resp?.value)
            return;
        const decodedMangoAccount = this.program.coder.accounts.decode('mangoAccount', resp.value.data);
        const mangoAccount = MangoAccount.from(mangoAccountPk, decodedMangoAccount);
        if (loadSerum3Oo) {
            await mangoAccount?.reloadSerum3OpenOrders(this);
        }
        return { slot: resp.context.slot, value: mangoAccount };
    }
    async getMangoAccountForOwner(group, ownerPk, accountNumber, loadSerum3Oo = false) {
        const mangoAccounts = await this.getMangoAccountsForOwner(group, ownerPk, loadSerum3Oo);
        const foundMangoAccount = mangoAccounts.find((a) => a.accountNum == accountNumber);
        return foundMangoAccount;
    }
    async getMangoAccountsForOwner(group, ownerPk, loadSerum3Oo = false) {
        const accounts = (await this.program.account.mangoAccount.all([
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
        ])).map((pa) => {
            return MangoAccount.from(pa.publicKey, pa.account);
        });
        if (loadSerum3Oo) {
            await Promise.all(accounts.map(async (a) => await a.reloadSerum3OpenOrders(this)));
        }
        return accounts;
    }
    async getMangoAccountsForDelegate(group, delegate, loadSerum3Oo = false) {
        const accounts = (await this.program.account.mangoAccount.all([
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
        ])).map((pa) => {
            return MangoAccount.from(pa.publicKey, pa.account);
        });
        if (loadSerum3Oo) {
            await Promise.all(accounts.map(async (a) => await a.reloadSerum3OpenOrders(this)));
        }
        return accounts;
    }
    async getAllMangoAccounts(group, loadSerum3Oo = false) {
        const accounts = (await this.program.account.mangoAccount.all([
            {
                memcmp: {
                    bytes: group.publicKey.toBase58(),
                    offset: 8,
                },
            },
        ])).map((pa) => {
            return MangoAccount.from(pa.publicKey, pa.account);
        });
        if (loadSerum3Oo) {
            await Promise.all(accounts.map(async (a) => await a.reloadSerum3OpenOrders(this)));
        }
        return accounts;
    }
    /**
     * Note: this ix doesn't settle liabs, reduce open positions, or withdraw tokens to wallet,
     * it simply closes the account. To close successfully ensure all positions are closed, or
     * use forceClose flag
     * @param group
     * @param mangoAccount
     * @param forceClose
     * @returns
     */
    async closeMangoAccount(group, mangoAccount, forceClose = false) {
        const ix = await this.program.methods
            .accountClose(forceClose)
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            owner: this.program.provider.wallet.publicKey,
            solDestination: mangoAccount.owner,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async emptyAndCloseMangoAccount(group, mangoAccount) {
        const instructions = [];
        const healthAccountsToExclude = [];
        for (const serum3Account of mangoAccount.serum3Active()) {
            const serum3Market = group.serum3MarketsMapByMarketIndex.get(serum3Account.marketIndex);
            const closeOOIx = await this.serum3CloseOpenOrdersIx(group, mangoAccount, serum3Market.serumMarketExternal);
            healthAccountsToExclude.push(serum3Account.openOrders);
            instructions.push(closeOOIx);
        }
        for (const perp of mangoAccount.perpActive()) {
            const perpMarketIndex = perp.marketIndex;
            const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
            const deactivatingPositionIx = await this.perpDeactivatePositionIx(group, mangoAccount, perpMarketIndex);
            healthAccountsToExclude.push(perpMarket.publicKey, perpMarket.oracle);
            instructions.push(deactivatingPositionIx);
        }
        for (const index in mangoAccount.tokensActive()) {
            const indexNum = Number(index);
            const accountsToExclude = [...healthAccountsToExclude];
            const token = mangoAccount.tokensActive()[indexNum];
            const bank = group.getFirstBankByTokenIndex(token.tokenIndex);
            //to withdraw from all token accounts we need to exclude previous tokens pubkeys
            //used to build health remaining accounts
            if (indexNum !== 0) {
                for (let i = indexNum; i--; i >= 0) {
                    const prevToken = mangoAccount.tokensActive()[i];
                    const prevBank = group.getFirstBankByTokenIndex(prevToken.tokenIndex);
                    accountsToExclude.push(prevBank.publicKey, prevBank.oracle);
                }
            }
            const withdrawIx = await this.tokenWithdrawNativeIx(group, mangoAccount, bank.mint, U64_MAX_BN, false, [...accountsToExclude]);
            instructions.push(...withdrawIx);
        }
        const closeIx = await this.program.methods
            .accountClose(false)
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            owner: this.program.provider.wallet.publicKey,
            solDestination: mangoAccount.owner,
        })
            .instruction();
        instructions.push(closeIx);
        return await this.sendAndConfirmTransactionForGroup(group, instructions);
    }
    async accountBuybackFeesWithMngoIx(group, mangoAccount, maxBuyback) {
        maxBuyback = maxBuyback ?? mangoAccount.getMaxFeesBuybackUi(group);
        return await this.program.methods
            .accountBuybackFeesWithMngo(new BN(maxBuyback))
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            daoAccount: group.buybackFeesSwapMangoAccount,
            mngoBank: group.getFirstBankForMngo().publicKey,
            mngoOracle: group.getFirstBankForMngo().oracle,
            feesBank: group.getFirstBankByTokenIndex(0).publicKey,
            feesOracle: group.getFirstBankByTokenIndex(0).oracle,
        })
            .instruction();
    }
    async accountBuybackFeesWithMngo(group, mangoAccount, maxBuyback) {
        const ix = await this.accountBuybackFeesWithMngoIx(group, mangoAccount, maxBuyback);
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async tokenDeposit(group, mangoAccount, mintPk, amount, reduceOnly = false) {
        const decimals = group.getMintDecimals(mintPk);
        const nativeAmount = toNative(amount, decimals);
        return await this.tokenDepositNative(group, mangoAccount, mintPk, nativeAmount, reduceOnly);
    }
    async tokenDepositNative(group, mangoAccount, mintPk, nativeAmount, reduceOnly = false) {
        const bank = group.getFirstBankByMint(mintPk);
        const tokenAccountPk = await getAssociatedTokenAddress(mintPk, mangoAccount.owner);
        let wrappedSolAccount;
        let preInstructions = [];
        let postInstructions = [];
        const additionalSigners = [];
        if (mintPk.equals(NATIVE_MINT)) {
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
                createInitializeAccount3Instruction(wrappedSolAccount.publicKey, NATIVE_MINT, mangoAccount.owner),
            ];
            postInstructions = [
                createCloseAccountInstruction(wrappedSolAccount.publicKey, mangoAccount.owner, mangoAccount.owner),
            ];
            additionalSigners.push(wrappedSolAccount);
        }
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [mangoAccount], [bank], []);
        const ix = await this.program.methods
            .tokenDeposit(new BN(nativeAmount), reduceOnly)
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
            .remainingAccounts(healthRemainingAccounts.map((pk) => ({ pubkey: pk, isWritable: false, isSigner: false })))
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [...preInstructions, ix, ...postInstructions], { additionalSigners });
    }
    async tokenWithdraw(group, mangoAccount, mintPk, amount, allowBorrow) {
        const nativeAmount = toNative(amount, group.getMintDecimals(mintPk));
        const ixes = await this.tokenWithdrawNativeIx(group, mangoAccount, mintPk, nativeAmount, allowBorrow);
        return await this.sendAndConfirmTransactionForGroup(group, ixes);
    }
    async tokenWithdrawNativeIx(group, mangoAccount, mintPk, nativeAmount, allowBorrow, healthAccountsToExclude = []) {
        const bank = group.getFirstBankByMint(mintPk);
        const tokenAccountPk = await getAssociatedTokenAddress(bank.mint, mangoAccount.owner);
        // ensure withdraws don't fail with missing ATAs
        const preInstructions = [
            await createAssociatedTokenAccountIdempotentInstruction(mangoAccount.owner, mangoAccount.owner, bank.mint),
        ];
        const postInstructions = [];
        if (mintPk.equals(NATIVE_MINT)) {
            postInstructions.push(createCloseAccountInstruction(tokenAccountPk, mangoAccount.owner, mangoAccount.owner));
        }
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [mangoAccount], [bank], []);
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
            .remainingAccounts(healthRemainingAccounts
            .filter((accounts) => !healthAccountsToExclude.find((accountsToExclude) => accounts.equals(accountsToExclude)))
            .map((pk) => ({
            pubkey: pk,
            isWritable: false,
            isSigner: false,
        })))
            .instruction();
        return [...preInstructions, ix, ...postInstructions];
    }
    async tokenWithdrawNative(group, mangoAccount, mintPk, nativeAmount, allowBorrow, healthAccountsToExclude = []) {
        const ixs = await this.tokenWithdrawNativeIx(group, mangoAccount, mintPk, nativeAmount, allowBorrow, healthAccountsToExclude);
        return await this.sendAndConfirmTransactionForGroup(group, ixs);
    }
    // Serum
    async serum3RegisterMarket(group, serum3MarketExternalPk, baseBank, quoteBank, marketIndex, name) {
        const ix = await this.program.methods
            .serum3RegisterMarket(marketIndex, name)
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
            serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
            serumMarketExternal: serum3MarketExternalPk,
            baseBank: baseBank.publicKey,
            quoteBank: quoteBank.publicKey,
            payer: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async serum3deregisterMarket(group, externalMarketPk) {
        const serum3Market = group.serum3MarketsMapByExternal.get(externalMarketPk.toBase58());
        const marketIndexBuf = Buffer.alloc(2);
        marketIndexBuf.writeUInt16LE(serum3Market.marketIndex);
        const [indexReservation] = await PublicKey.findProgramAddress([Buffer.from('Serum3Index'), group.publicKey.toBuffer(), marketIndexBuf], this.program.programId);
        const ix = await this.program.methods
            .serum3DeregisterMarket()
            .accounts({
            group: group.publicKey,
            serumMarket: serum3Market.publicKey,
            indexReservation,
            solDestination: this.program.provider.wallet
                .publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async serum3GetMarkets(group, baseTokenIndex, quoteTokenIndex) {
        const bumpfbuf = Buffer.alloc(1);
        bumpfbuf.writeUInt8(255);
        const filters = [
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
        return (await this.program.account.serum3Market.all(filters)).map((tuple) => Serum3Market.from(tuple.publicKey, tuple.account));
    }
    async serum3CreateOpenOrders(group, mangoAccount, externalMarketPk) {
        const serum3Market = group.serum3MarketsMapByExternal.get(externalMarketPk.toBase58());
        const ix = await this.program.methods
            .serum3CreateOpenOrders()
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            serumMarket: serum3Market.publicKey,
            serumProgram: serum3Market.serumProgram,
            serumMarketExternal: serum3Market.serumMarketExternal,
            owner: this.program.provider.wallet.publicKey,
            payer: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async serum3CreateOpenOrdersIx(group, mangoAccount, externalMarketPk) {
        const serum3Market = group.serum3MarketsMapByExternal.get(externalMarketPk.toBase58());
        const ix = await this.program.methods
            .serum3CreateOpenOrders()
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            serumMarket: serum3Market.publicKey,
            serumProgram: serum3Market.serumProgram,
            serumMarketExternal: serum3Market.serumMarketExternal,
            owner: this.program.provider.wallet.publicKey,
            payer: this.program.provider.wallet.publicKey,
        })
            .instruction();
        return ix;
    }
    async serum3CloseOpenOrdersIx(group, mangoAccount, externalMarketPk) {
        const serum3Market = group.serum3MarketsMapByExternal.get(externalMarketPk.toBase58());
        const openOrders = mangoAccount.serum3.find((account) => account.marketIndex === serum3Market.marketIndex)?.openOrders;
        return await this.program.methods
            .serum3CloseOpenOrders()
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            serumMarket: serum3Market.publicKey,
            serumProgram: serum3Market.serumProgram,
            serumMarketExternal: serum3Market.serumMarketExternal,
            openOrders,
            solDestination: this.program.provider.wallet
                .publicKey,
        })
            .instruction();
    }
    async serum3CloseOpenOrders(group, mangoAccount, externalMarketPk) {
        const ix = await this.serum3CloseOpenOrdersIx(group, mangoAccount, externalMarketPk);
        return await sendTransaction(this.program.provider, [ix], group.addressLookupTablesList, {
            postSendTxCallback: this.postSendTxCallback,
        });
    }
    async serum3PlaceOrderIx(group, mangoAccount, externalMarketPk, side, price, size, selfTradeBehavior, orderType, clientOrderId, limit) {
        const ixs = [];
        const serum3Market = group.serum3MarketsMapByExternal.get(externalMarketPk.toBase58());
        let openOrderPk = undefined;
        const banks = [];
        const openOrdersForMarket = [];
        if (!mangoAccount.getSerum3Account(serum3Market.marketIndex)) {
            const ix = await this.serum3CreateOpenOrdersIx(group, mangoAccount, serum3Market.serumMarketExternal);
            ixs.push(ix);
            openOrderPk = await serum3Market.findOoPda(this.program.programId, mangoAccount.publicKey);
            openOrdersForMarket.push([serum3Market, openOrderPk]);
            const baseTokenIndex = serum3Market.baseTokenIndex;
            const quoteTokenIndex = serum3Market.quoteTokenIndex;
            // only include banks if no deposit has been previously made for same token
            if (!mangoAccount.getToken(baseTokenIndex)?.isActive()) {
                banks.push(group.getFirstBankByTokenIndex(baseTokenIndex));
            }
            if (!mangoAccount.getToken(quoteTokenIndex)?.isActive()) {
                banks.push(group.getFirstBankByTokenIndex(quoteTokenIndex));
            }
        }
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [mangoAccount], banks, [], openOrdersForMarket);
        const serum3MarketExternal = group.serum3ExternalMarketsMap.get(externalMarketPk.toBase58());
        const serum3MarketExternalVaultSigner = await generateSerum3MarketExternalVaultSignerAddress(this.cluster, serum3Market, serum3MarketExternal);
        const limitPrice = serum3MarketExternal.priceNumberToLots(price);
        const maxBaseQuantity = serum3MarketExternal.baseSizeNumberToLots(size);
        const isTaker = orderType !== Serum3OrderType.postOnly;
        const maxQuoteQuantity = new BN(serum3MarketExternal.decoded.quoteLotSize.toNumber() *
            (1 + Math.max(serum3Market.getFeeRates(isTaker), 0)) *
            serum3MarketExternal.baseSizeNumberToLots(size).toNumber() *
            serum3MarketExternal.priceNumberToLots(price).toNumber());
        const payerTokenIndex = (() => {
            if (side == Serum3Side.bid) {
                return serum3Market.quoteTokenIndex;
            }
            else {
                return serum3Market.baseTokenIndex;
            }
        })();
        const payerBank = group.getFirstBankByTokenIndex(payerTokenIndex);
        const ix = await this.program.methods
            .serum3PlaceOrder(side, limitPrice, maxBaseQuantity, maxQuoteQuantity, selfTradeBehavior, orderType, new BN(clientOrderId), limit)
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            owner: this.program.provider.wallet.publicKey,
            openOrders: openOrderPk ||
                mangoAccount.getSerum3Account(serum3Market.marketIndex)?.openOrders,
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
            .remainingAccounts(healthRemainingAccounts.map((pk) => ({ pubkey: pk, isWritable: false, isSigner: false })))
            .instruction();
        ixs.push(ix);
        return ixs;
    }
    async serum3PlaceOrder(group, mangoAccount, externalMarketPk, side, price, size, selfTradeBehavior, orderType, clientOrderId, limit) {
        const placeOrderIxes = await this.serum3PlaceOrderIx(group, mangoAccount, externalMarketPk, side, price, size, selfTradeBehavior, orderType, clientOrderId, limit);
        const settleIx = await this.serum3SettleFundsIx(group, mangoAccount, externalMarketPk);
        return await this.sendAndConfirmTransactionForGroup(group, [
            ...placeOrderIxes,
            settleIx,
        ]);
    }
    async serum3CancelAllOrders(group, mangoAccount, externalMarketPk, limit) {
        const serum3Market = group.serum3MarketsMapByExternal.get(externalMarketPk.toBase58());
        const serum3MarketExternal = group.serum3ExternalMarketsMap.get(externalMarketPk.toBase58());
        const ix = await this.program.methods
            .serum3CancelAllOrders(limit ? limit : 10)
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            owner: this.program.provider.wallet.publicKey,
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
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async serum3SettleFundsIx(group, mangoAccount, externalMarketPk) {
        if (this.openbookFeesToDao == false) {
            throw new Error(`openbookFeesToDao is set to false, please use serum3SettleFundsV2Ix`);
        }
        const serum3Market = group.serum3MarketsMapByExternal.get(externalMarketPk.toBase58());
        const serum3MarketExternal = group.serum3ExternalMarketsMap.get(externalMarketPk.toBase58());
        const [serum3MarketExternalVaultSigner, openOrderPublicKey] = await Promise.all([
            generateSerum3MarketExternalVaultSignerAddress(this.cluster, serum3Market, serum3MarketExternal),
            serum3Market.findOoPda(this.program.programId, mangoAccount.publicKey),
        ]);
        const ix = await this.program.methods
            .serum3SettleFunds()
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            owner: this.program.provider.wallet.publicKey,
            openOrders: openOrderPublicKey,
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
    async serum3SettleFundsV2Ix(group, mangoAccount, externalMarketPk) {
        const serum3Market = group.serum3MarketsMapByExternal.get(externalMarketPk.toBase58());
        const serum3MarketExternal = group.serum3ExternalMarketsMap.get(externalMarketPk.toBase58());
        const [serum3MarketExternalVaultSigner, openOrderPublicKey] = await Promise.all([
            generateSerum3MarketExternalVaultSignerAddress(this.cluster, serum3Market, serum3MarketExternal),
            serum3Market.findOoPda(this.program.programId, mangoAccount.publicKey),
        ]);
        const ix = await this.program.methods
            .serum3SettleFundsV2(this.openbookFeesToDao)
            .accounts({
            v1: {
                group: group.publicKey,
                account: mangoAccount.publicKey,
                owner: this.program.provider.wallet.publicKey,
                openOrders: openOrderPublicKey,
                serumMarket: serum3Market.publicKey,
                serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
                serumMarketExternal: serum3Market.serumMarketExternal,
                marketBaseVault: serum3MarketExternal.decoded.baseVault,
                marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
                marketVaultSigner: serum3MarketExternalVaultSigner,
                quoteBank: group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex).publicKey,
                quoteVault: group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex).vault,
                baseBank: group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
                    .publicKey,
                baseVault: group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
                    .vault,
            },
            v2: {
                quoteOracle: group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex).oracle,
                baseOracle: group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex).oracle,
            },
        })
            .instruction();
        return ix;
    }
    async serum3SettleFunds(group, mangoAccount, externalMarketPk) {
        const ix = await this.serum3SettleFundsV2Ix(group, mangoAccount, externalMarketPk);
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async serum3CancelOrderIx(group, mangoAccount, externalMarketPk, side, orderId) {
        const serum3Market = group.serum3MarketsMapByExternal.get(externalMarketPk.toBase58());
        const serum3MarketExternal = group.serum3ExternalMarketsMap.get(externalMarketPk.toBase58());
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
    async serum3CancelOrder(group, mangoAccount, externalMarketPk, side, orderId) {
        const ixes = await Promise.all([
            this.serum3CancelOrderIx(group, mangoAccount, externalMarketPk, side, orderId),
            this.serum3SettleFundsV2Ix(group, mangoAccount, externalMarketPk),
        ]);
        return await this.sendAndConfirmTransactionForGroup(group, ixes);
    }
    /// perps
    async perpCreateMarket(group, oraclePk, perpMarketIndex, name, oracleConfig, baseDecimals, quoteLotSize, baseLotSize, maintBaseAssetWeight, initBaseAssetWeight, maintBaseLiabWeight, initBaseLiabWeight, maintOverallAssetWeight, initOverallAssetWeight, baseLiquidationFee, makerFee, takerFee, feePenalty, minFunding, maxFunding, impactQuantity, groupInsuranceFund, settleFeeFlat, settleFeeAmountThreshold, settleFeeFractionLowHealth, settleTokenIndex, settlePnlLimitFactor, settlePnlLimitWindowSize, positivePnlLiquidationFee) {
        const bids = new Keypair();
        const asks = new Keypair();
        const eventQueue = new Keypair();
        const bookSideSize = this.program._coder.accounts.size(this.program.account.bookSide._idlAccount);
        const eventQueueSize = this.program._coder.accounts.size(this.program.account.eventQueue._idlAccount);
        const ix = await this.program.methods
            .perpCreateMarket(perpMarketIndex, name, oracleConfig, baseDecimals, new BN(quoteLotSize), new BN(baseLotSize), maintBaseAssetWeight, initBaseAssetWeight, maintBaseLiabWeight, initBaseLiabWeight, maintOverallAssetWeight, initOverallAssetWeight, baseLiquidationFee, makerFee, takerFee, minFunding, maxFunding, new BN(impactQuantity), groupInsuranceFund, feePenalty, settleFeeFlat, settleFeeAmountThreshold, settleFeeFractionLowHealth, settleTokenIndex, settlePnlLimitFactor, new BN(settlePnlLimitWindowSize), positivePnlLiquidationFee)
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
            oracle: oraclePk,
            bids: bids.publicKey,
            asks: asks.publicKey,
            eventQueue: eventQueue.publicKey,
            payer: this.program.provider.wallet.publicKey,
        })
            .instruction();
        const preInstructions = [
            // book sides
            SystemProgram.createAccount({
                programId: this.program.programId,
                space: bookSideSize,
                lamports: await this.program.provider.connection.getMinimumBalanceForRentExemption(bookSideSize),
                fromPubkey: this.program.provider.wallet.publicKey,
                newAccountPubkey: bids.publicKey,
            }),
            SystemProgram.createAccount({
                programId: this.program.programId,
                space: bookSideSize,
                lamports: await this.program.provider.connection.getMinimumBalanceForRentExemption(bookSideSize),
                fromPubkey: this.program.provider.wallet.publicKey,
                newAccountPubkey: asks.publicKey,
            }),
            // event queue
            SystemProgram.createAccount({
                programId: this.program.programId,
                space: eventQueueSize,
                lamports: await this.program.provider.connection.getMinimumBalanceForRentExemption(eventQueueSize),
                fromPubkey: this.program.provider.wallet.publicKey,
                newAccountPubkey: eventQueue.publicKey,
            }),
        ];
        return await this.sendAndConfirmTransactionForGroup(group, [...preInstructions, ix], {
            additionalSigners: [bids, asks, eventQueue],
        });
    }
    async perpEditMarket(group, perpMarketIndex, params) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const ix = await this.program.methods
            .perpEditMarket(params.oracle, params.oracleConfig, params.baseDecimals, params.maintBaseAssetWeight, params.initBaseAssetWeight, params.maintBaseLiabWeight, params.initBaseLiabWeight, params.maintOverallAssetWeight, params.initOverallAssetWeight, params.baseLiquidationFee, params.makerFee, params.takerFee, params.minFunding, params.maxFunding, params.impactQuantity !== null ? new BN(params.impactQuantity) : null, params.groupInsuranceFund, params.feePenalty, params.settleFeeFlat, params.settleFeeAmountThreshold, params.settleFeeFractionLowHealth, params.stablePriceDelayIntervalSeconds, params.stablePriceDelayGrowthLimit, params.stablePriceGrowthLimit, params.settlePnlLimitFactor, params.settlePnlLimitWindowSize !== null
            ? new BN(params.settlePnlLimitWindowSize)
            : null, params.reduceOnly, params.resetStablePrice ?? false, params.positivePnlLiquidationFee, params.name)
            .accounts({
            group: group.publicKey,
            oracle: params.oracle ?? perpMarket.oracle,
            admin: this.program.provider.wallet.publicKey,
            perpMarket: perpMarket.publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async perpCloseMarket(group, perpMarketIndex) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const ix = await this.program.methods
            .perpCloseMarket()
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
            perpMarket: perpMarket.publicKey,
            bids: perpMarket.bids,
            asks: perpMarket.asks,
            eventQueue: perpMarket.eventQueue,
            solDestination: this.program.provider.wallet
                .publicKey,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async perpGetMarkets(group) {
        const bumpfbuf = Buffer.alloc(1);
        bumpfbuf.writeUInt8(255);
        const filters = [
            {
                memcmp: {
                    bytes: group.publicKey.toBase58(),
                    offset: 8,
                },
            },
        ];
        return (await this.program.account.perpMarket.all(filters)).map((tuple) => PerpMarket.from(tuple.publicKey, tuple.account));
    }
    async perpDeactivatePositionIx(group, mangoAccount, perpMarketIndex) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [mangoAccount], [], []);
        return await this.program.methods
            .perpDeactivatePosition()
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            perpMarket: perpMarket.publicKey,
            owner: this.program.provider.wallet.publicKey,
        })
            .remainingAccounts(healthRemainingAccounts.map((pk) => ({ pubkey: pk, isWritable: false, isSigner: false })))
            .instruction();
    }
    async perpDeactivatePosition(group, mangoAccount, perpMarketIndex) {
        const ix = await this.perpDeactivatePositionIx(group, mangoAccount, perpMarketIndex);
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    // perpPlaceOrder ix returns an optional, custom order id,
    // but, since we use a customer tx sender, this method
    // doesn't return it
    async perpPlaceOrder(group, mangoAccount, perpMarketIndex, side, price, quantity, maxQuoteQuantity, clientOrderId, orderType, reduceOnly, expiryTimestamp, limit) {
        const ix = await this.perpPlaceOrderIx(group, mangoAccount, perpMarketIndex, side, price, quantity, maxQuoteQuantity, clientOrderId, orderType, reduceOnly, expiryTimestamp, limit);
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async perpPlaceOrderIx(group, mangoAccount, perpMarketIndex, side, price, quantity, maxQuoteQuantity, clientOrderId, orderType, reduceOnly, expiryTimestamp, limit) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [mangoAccount], 
        // Settlement token bank, because a position for it may be created
        [group.getFirstBankForPerpSettlement()], [perpMarket]);
        return await this.program.methods
            .perpPlaceOrder(side, perpMarket.uiPriceToLots(price), perpMarket.uiBaseToLots(quantity), maxQuoteQuantity
            ? perpMarket.uiQuoteToLots(maxQuoteQuantity)
            : I64_MAX_BN, new BN(clientOrderId ? clientOrderId : Date.now()), orderType ? orderType : PerpOrderType.limit, reduceOnly ? reduceOnly : false, new BN(expiryTimestamp ? expiryTimestamp : 0), limit ? limit : 10)
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            perpMarket: perpMarket.publicKey,
            bids: perpMarket.bids,
            asks: perpMarket.asks,
            eventQueue: perpMarket.eventQueue,
            oracle: perpMarket.oracle,
            owner: this.program.provider.wallet.publicKey,
        })
            .remainingAccounts(healthRemainingAccounts.map((pk) => ({ pubkey: pk, isWritable: false, isSigner: false })))
            .instruction();
    }
    async perpPlaceOrderPegged(group, mangoAccount, perpMarketIndex, side, priceOffset, pegLimit, quantity, maxQuoteQuantity, clientOrderId, orderType, reduceOnly, expiryTimestamp, limit) {
        const ix = await this.perpPlaceOrderPeggedIx(group, mangoAccount, perpMarketIndex, side, priceOffset, pegLimit, quantity, maxQuoteQuantity, clientOrderId, orderType, reduceOnly, expiryTimestamp, limit);
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async perpPlaceOrderPeggedIx(group, mangoAccount, perpMarketIndex, side, priceOffset, pegLimit, quantity, maxQuoteQuantity, clientOrderId, orderType, reduceOnly, expiryTimestamp, limit) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [mangoAccount], 
        // Settlement token bank, because a position for it may be created
        [group.getFirstBankForPerpSettlement()], [perpMarket]);
        return await this.program.methods
            .perpPlaceOrderPegged(side, perpMarket.uiPriceToLots(priceOffset), perpMarket.uiPriceToLots(pegLimit), perpMarket.uiBaseToLots(quantity), maxQuoteQuantity
            ? perpMarket.uiQuoteToLots(maxQuoteQuantity)
            : I64_MAX_BN, new BN(clientOrderId ?? Date.now()), orderType ? orderType : PerpOrderType.limit, reduceOnly ? reduceOnly : false, new BN(expiryTimestamp ?? 0), limit ? limit : 10, -1)
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            perpMarket: perpMarket.publicKey,
            bids: perpMarket.bids,
            asks: perpMarket.asks,
            eventQueue: perpMarket.eventQueue,
            oracle: perpMarket.oracle,
            owner: this.program.provider.wallet.publicKey,
        })
            .remainingAccounts(healthRemainingAccounts.map((pk) => ({ pubkey: pk, isWritable: false, isSigner: false })))
            .instruction();
    }
    async perpCancelOrderIx(group, mangoAccount, perpMarketIndex, orderId) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        return await this.program.methods
            .perpCancelOrder(new BN(orderId))
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            owner: this.program.provider.wallet.publicKey,
            perpMarket: perpMarket.publicKey,
            bids: perpMarket.bids,
            asks: perpMarket.asks,
        })
            .instruction();
    }
    async perpCancelOrder(group, mangoAccount, perpMarketIndex, orderId) {
        const ix = await this.perpCancelOrderIx(group, mangoAccount, perpMarketIndex, orderId);
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async perpCancelAllOrders(group, mangoAccount, perpMarketIndex, limit) {
        const ix = await this.perpCancelAllOrdersIx(group, mangoAccount, perpMarketIndex, limit);
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async perpCancelAllOrdersIx(group, mangoAccount, perpMarketIndex, limit) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        return await this.program.methods
            .perpCancelAllOrders(limit)
            .accounts({
            group: group.publicKey,
            account: mangoAccount.publicKey,
            perpMarket: perpMarket.publicKey,
            bids: perpMarket.bids,
            asks: perpMarket.asks,
            owner: this.program.provider.wallet.publicKey,
        })
            .instruction();
    }
    async perpSettlePnl(group, profitableAccount, unprofitableAccount, settler, perpMarketIndex) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Scanning, group, [profitableAccount, unprofitableAccount], [group.getFirstBankForPerpSettlement()], [perpMarket]);
        const bank = group.banksMapByTokenIndex.get(0)[0];
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
            settlerOwner: this.program.provider.wallet
                .publicKey,
        })
            .remainingAccounts(healthRemainingAccounts.map((pk) => ({ pubkey: pk, isWritable: false, isSigner: false })))
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async perpSettleFees(group, account, perpMarketIndex, maxSettleAmount) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [account], // Account must be unprofitable
        [group.getFirstBankForPerpSettlement()], [perpMarket]);
        const bank = group.banksMapByTokenIndex.get(0)[0];
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
            .remainingAccounts(healthRemainingAccounts.map((pk) => ({ pubkey: pk, isWritable: false, isSigner: false })))
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async perpConsumeEvents(group, perpMarketIndex, accounts, limit) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const ix = await this.program.methods
            .perpConsumeEvents(new BN(limit))
            .accounts({
            group: group.publicKey,
            perpMarket: perpMarket.publicKey,
            eventQueue: perpMarket.eventQueue,
        })
            .remainingAccounts(accounts.map((pk) => ({ pubkey: pk, isWritable: true, isSigner: false })))
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async perpConsumeAllEvents(group, perpMarketIndex) {
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
                        const fill = ev;
                        return [fill.maker, fill.taker];
                    }
                    case PerpEventQueue.OUT_EVENT_TYPE: {
                        const out = ev;
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
    async marginTrade({ group, mangoAccount, inputMintPk, amountIn, outputMintPk, userDefinedInstructions, userDefinedAlts = [], 
    // margin trade is a general function
    // set flash_loan_type to FlashLoanType.swap if you desire the transaction to be recorded as a swap
    flashLoanType, }) {
        const inputBank = group.getFirstBankByMint(inputMintPk);
        const outputBank = group.getFirstBankByMint(outputMintPk);
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [mangoAccount], [inputBank, outputBank], []);
        const parsedHealthAccounts = healthRemainingAccounts.map((pk) => ({
            pubkey: pk,
            isWritable: false,
            isSigner: false,
        }));
        /*
         * Find or create associated token accounts
         */
        const inputTokenAccountPk = await getAssociatedTokenAddress(inputBank.mint, mangoAccount.owner);
        const inputTokenAccExists = await this.program.provider.connection.getAccountInfo(inputTokenAccountPk);
        const preInstructions = [];
        if (!inputTokenAccExists) {
            preInstructions.push(await createAssociatedTokenAccountIdempotentInstruction(mangoAccount.owner, mangoAccount.owner, inputBank.mint));
        }
        const outputTokenAccountPk = await getAssociatedTokenAddress(outputBank.mint, mangoAccount.owner);
        const outputTokenAccExists = await this.program.provider.connection.getAccountInfo(outputTokenAccountPk);
        if (!outputTokenAccExists) {
            preInstructions.push(await createAssociatedTokenAccountIdempotentInstruction(mangoAccount.owner, mangoAccount.owner, outputBank.mint));
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
            new BN(0) /* we don't care about borrowing the target amount, this is just a dummy */,
        ])
            .accounts({
            account: mangoAccount.publicKey,
            owner: this.program.provider.wallet.publicKey,
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
        return await this.sendAndConfirmTransactionForGroup(group, [
            ...preInstructions,
            flashLoanBeginIx,
            ...userDefinedInstructions.filter((ix) => ix.keys.length > 2),
            flashLoanEndIx,
        ], { alts: [...group.addressLookupTablesList, ...userDefinedAlts] });
    }
    async updateIndexAndRate(group, mintPk) {
        const bank = group.getFirstBankByMint(mintPk);
        const mintInfo = group.mintInfosMapByMint.get(mintPk.toString());
        const ix = await this.program.methods
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
            },
        ])
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    /// liquidations
    async liqTokenWithToken(group, liqor, liqee, assetMintPk, liabMintPk, maxLiabTransfer) {
        const assetBank = group.getFirstBankByMint(assetMintPk);
        const liabBank = group.getFirstBankByMint(liabMintPk);
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Scanning, group, [liqor, liqee], [assetBank, liabBank], []);
        const parsedHealthAccounts = healthRemainingAccounts.map((pk) => ({
            pubkey: pk,
            isWritable: pk.equals(assetBank.publicKey) || pk.equals(liabBank.publicKey)
                ? true
                : false,
            isSigner: false,
        }));
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
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async altSet(group, addressLookupTable, index) {
        const ix = await this.program.methods
            .altSet(index)
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
            addressLookupTable,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async altExtend(group, addressLookupTable, index, pks) {
        const ix = await this.program.methods
            .altExtend(index, pks)
            .accounts({
            group: group.publicKey,
            admin: this.program.provider.wallet.publicKey,
            payer: this.program.provider.wallet.publicKey,
            addressLookupTable,
        })
            .instruction();
        return await this.sendAndConfirmTransactionForGroup(group, [ix]);
    }
    async healthRegionBeginIx(group, account, banks = [], perpMarkets = []) {
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [account], [...banks], [...perpMarkets]);
        const parsedHealthAccounts = healthRemainingAccounts.map((pk) => ({
            pubkey: pk,
            isWritable: false,
            isSigner: false,
        }));
        return await this.program.methods
            .healthRegionBegin()
            .accounts({
            group: group.publicKey,
            account: account.publicKey,
            instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        })
            .remainingAccounts(parsedHealthAccounts)
            .instruction();
    }
    async healthRegionEndIx(group, account, banks = [], perpMarkets = []) {
        const healthRemainingAccounts = this.buildHealthRemainingAccounts(AccountRetriever.Fixed, group, [account], [...banks], [...perpMarkets]);
        const parsedHealthAccounts = healthRemainingAccounts.map((pk) => ({
            pubkey: pk,
            isWritable: false,
            isSigner: false,
        }));
        return await this.program.methods
            .healthRegionEnd()
            .accounts({ account: account.publicKey })
            .remainingAccounts(parsedHealthAccounts)
            .instruction();
    }
    /// static
    static connect(provider, cluster, programId, opts) {
        const idl = IDL;
        return new MangoClient(new Program(idl, programId, provider), programId, cluster, opts);
    }
    static connectForGroupName(provider, groupName) {
        const idl = IDL;
        const id = Id.fromIdsByName(groupName);
        return new MangoClient(new Program(idl, new PublicKey(id.mangoProgramId), provider), new PublicKey(id.mangoProgramId), id.cluster);
    }
    buildHealthRemainingAccounts(retriever, group, mangoAccounts, banks = [], perpMarkets = [], openOrdersForMarket = []) {
        if (retriever === AccountRetriever.Fixed) {
            return this.buildFixedAccountRetrieverHealthAccounts(group, mangoAccounts[0], banks, perpMarkets, openOrdersForMarket);
        }
        else {
            return this.buildScanningAccountRetrieverHealthAccounts(group, mangoAccounts, banks, perpMarkets);
        }
    }
    buildFixedAccountRetrieverHealthAccounts(group, mangoAccount, 
    // Banks and perpMarkets for whom positions don't exist on mango account,
    // but user would potentially open new positions.
    banks, perpMarkets, openOrdersForMarket) {
        const healthRemainingAccounts = [];
        const tokenPositionIndices = mangoAccount.tokens.map((t) => t.tokenIndex);
        for (const bank of banks) {
            const tokenPositionExists = tokenPositionIndices.indexOf(bank.tokenIndex) > -1;
            if (!tokenPositionExists) {
                const inactiveTokenPosition = tokenPositionIndices.findIndex((index) => index === TokenPosition.TokenIndexUnset);
                if (inactiveTokenPosition != -1) {
                    tokenPositionIndices[inactiveTokenPosition] = bank.tokenIndex;
                }
            }
        }
        const mintInfos = tokenPositionIndices
            .filter((tokenIndex) => tokenIndex !== TokenPosition.TokenIndexUnset)
            .map((tokenIndex) => group.mintInfosMapByTokenIndex.get(tokenIndex));
        healthRemainingAccounts.push(...mintInfos.map((mintInfo) => mintInfo.firstBank()));
        healthRemainingAccounts.push(...mintInfos.map((mintInfo) => mintInfo.oracle));
        // insert any extra perp markets in the free perp position slots
        const perpPositionIndices = mangoAccount.perps.map((p) => p.marketIndex);
        for (const perpMarket of perpMarkets) {
            const perpPositionExists = perpPositionIndices.indexOf(perpMarket.perpMarketIndex) > -1;
            if (!perpPositionExists) {
                const inactivePerpPosition = perpPositionIndices.findIndex((perpIdx) => perpIdx === PerpPosition.PerpMarketIndexUnset);
                if (inactivePerpPosition != -1) {
                    perpPositionIndices[inactivePerpPosition] =
                        perpMarket.perpMarketIndex;
                }
            }
        }
        const allPerpMarkets = perpPositionIndices
            .filter((perpIdx) => perpIdx !== PerpPosition.PerpMarketIndexUnset)
            .map((perpIdx) => group.getPerpMarketByMarketIndex(perpIdx));
        healthRemainingAccounts.push(...allPerpMarkets.map((perp) => perp.publicKey));
        healthRemainingAccounts.push(...allPerpMarkets.map((perp) => perp.oracle));
        // insert any extra open orders accounts in the cooresponding free serum market slot
        const serumPositionIndices = mangoAccount.serum3.map((s) => ({
            marketIndex: s.marketIndex,
            openOrders: s.openOrders,
        }));
        for (const [serum3Market, openOrderPk] of openOrdersForMarket) {
            const ooPositionExists = serumPositionIndices.findIndex((i) => i.marketIndex === serum3Market.marketIndex) > -1;
            if (!ooPositionExists) {
                const inactiveSerumPosition = serumPositionIndices.findIndex((serumPos) => serumPos.marketIndex === Serum3Orders.Serum3MarketIndexUnset);
                if (inactiveSerumPosition != -1) {
                    serumPositionIndices[inactiveSerumPosition].marketIndex =
                        serum3Market.marketIndex;
                    serumPositionIndices[inactiveSerumPosition].openOrders = openOrderPk;
                }
            }
        }
        healthRemainingAccounts.push(...serumPositionIndices
            .filter((serumPosition) => serumPosition.marketIndex !== Serum3Orders.Serum3MarketIndexUnset)
            .map((serumPosition) => serumPosition.openOrders));
        // debugHealthAccounts(group, mangoAccount, healthRemainingAccounts);
        return healthRemainingAccounts;
    }
    buildScanningAccountRetrieverHealthAccounts(group, mangoAccounts, banks, perpMarkets) {
        const healthRemainingAccounts = [];
        let tokenIndices = [];
        for (const mangoAccount of mangoAccounts) {
            tokenIndices.push(...mangoAccount.tokens
                .filter((token) => token.tokenIndex !== 65535)
                .map((token) => token.tokenIndex));
        }
        tokenIndices = [...new Set(tokenIndices)];
        if (banks?.length) {
            for (const bank of banks) {
                tokenIndices.push(bank.tokenIndex);
            }
        }
        const mintInfos = [...new Set(tokenIndices)].map((tokenIndex) => group.mintInfosMapByTokenIndex.get(tokenIndex));
        healthRemainingAccounts.push(...mintInfos.map((mintInfo) => mintInfo.firstBank()));
        healthRemainingAccounts.push(...mintInfos.map((mintInfo) => mintInfo.oracle));
        const perpIndices = [];
        for (const mangoAccount of mangoAccounts) {
            perpIndices.push(...mangoAccount.perps
                .filter((perp) => perp.marketIndex !== 65535)
                .map((perp) => perp.marketIndex));
        }
        perpIndices.push(...perpMarkets.map((perp) => perp.perpMarketIndex));
        const allPerpMarkets = [...new Set(perpIndices)].map((marketIndex) => group.findPerpMarket(marketIndex));
        // Add perp accounts
        healthRemainingAccounts.push(...allPerpMarkets.map((p) => p.publicKey));
        // Add oracle for each perp
        healthRemainingAccounts.push(...allPerpMarkets.map((p) => p.oracle));
        for (const mangoAccount of mangoAccounts) {
            healthRemainingAccounts.push(...mangoAccount.serum3
                .filter((serum3Account) => serum3Account.marketIndex !== 65535)
                .map((serum3Account) => serum3Account.openOrders));
        }
        return healthRemainingAccounts;
    }
    async modifyPerpOrder(group, mangoAccount, perpMarketIndex, orderId, side, price, quantity, maxQuoteQuantity, clientOrderId, orderType, reduceOnly, expiryTimestamp, limit) {
        const transactionInstructions = [];
        const [cancelOrderIx, placeOrderIx] = await Promise.all([
            this.perpCancelOrderIx(group, mangoAccount, perpMarketIndex, orderId),
            this.perpPlaceOrderIx(group, mangoAccount, perpMarketIndex, side, price, quantity, maxQuoteQuantity, clientOrderId, orderType, reduceOnly, expiryTimestamp, limit),
        ]);
        transactionInstructions.push(cancelOrderIx, placeOrderIx);
        return await this.sendAndConfirmTransactionForGroup(group, transactionInstructions);
    }
    async modifySerum3Order(group, orderId, mangoAccount, externalMarketPk, side, price, size, selfTradeBehavior, orderType, clientOrderId, limit) {
        const transactionInstructions = [];
        const [cancelOrderIx, settleIx, placeOrderIx] = await Promise.all([
            this.serum3CancelOrderIx(group, mangoAccount, externalMarketPk, side, orderId),
            this.serum3SettleFundsV2Ix(group, mangoAccount, externalMarketPk),
            this.serum3PlaceOrderIx(group, mangoAccount, externalMarketPk, side, price, size, selfTradeBehavior, orderType, clientOrderId, limit),
        ]);
        transactionInstructions.push(cancelOrderIx, settleIx, ...placeOrderIx);
        return await this.sendAndConfirmTransactionForGroup(group, transactionInstructions);
    }
}
