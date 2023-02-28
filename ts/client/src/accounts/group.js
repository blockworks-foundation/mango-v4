import { BorshAccountsCoder } from '@coral-xyz/anchor';
import { Market } from '@project-serum/serum';
import { parsePriceData } from '@pythnetwork/client';
import { TOKEN_PROGRAM_ID, unpackAccount } from '@solana/spl-token';
import { PublicKey, } from '@solana/web3.js';
import BN from 'bn.js';
import cloneDeep from 'lodash/cloneDeep';
import merge from 'lodash/merge';
import { OPENBOOK_PROGRAM_ID } from '../constants';
import { I80F48, ONE_I80F48 } from '../numbers/I80F48';
import { toNative, toNativeI80F48, toUiDecimals } from '../utils';
import { Bank, MintInfo } from './bank';
import { isPythOracle, isSwitchboardOracle, parseSwitchboardOracle, } from './oracle';
import { PerpMarket } from './perp';
import { Serum3Market } from './serum3';
export class Group {
    publicKey;
    creator;
    groupNum;
    admin;
    fastListingAdmin;
    mngoTokenIndex;
    insuranceMint;
    insuranceVault;
    testing;
    version;
    buybackFees;
    buybackFeesMngoBonusFactor;
    addressLookupTables;
    securityAdmin;
    depositLimitQuote;
    ixGate;
    buybackFeesSwapMangoAccount;
    addressLookupTablesList;
    banksMapByName;
    banksMapByMint;
    banksMapByTokenIndex;
    serum3MarketsMapByExternal;
    serum3MarketsMapByMarketIndex;
    serum3ExternalMarketsMap;
    perpMarketsMapByOracle;
    perpMarketsMapByMarketIndex;
    perpMarketsMapByName;
    mintInfosMapByTokenIndex;
    mintInfosMapByMint;
    vaultAmountsMap;
    static from(publicKey, obj) {
        return new Group(publicKey, obj.creator, obj.groupNum, obj.admin, obj.fastListingAdmin, obj.mngoTokenIndex, obj.insuranceMint, obj.insuranceVault, obj.testing, obj.version, obj.buybackFees == 1, obj.buybackFeesMngoBonusFactor, obj.addressLookupTables, obj.securityAdmin, obj.depositLimitQuote, obj.ixGate, obj.buybackFeesSwapMangoAccount, [], // addressLookupTablesList
        new Map(), // banksMapByName
        new Map(), // banksMapByMint
        new Map(), // banksMapByTokenIndex
        new Map(), // serum3MarketsMapByExternal
        new Map(), // serum3MarketsMapByMarketIndex
        new Map(), // serum3MarketExternalsMap
        new Map(), // perpMarketsMapByOracle
        new Map(), // perpMarketsMapByMarketIndex
        new Map(), // perpMarketsMapByName
        new Map(), // mintInfosMapByTokenIndex
        new Map(), // mintInfosMapByMint
        new Map());
    }
    constructor(publicKey, creator, groupNum, admin, fastListingAdmin, mngoTokenIndex, insuranceMint, insuranceVault, testing, version, buybackFees, buybackFeesMngoBonusFactor, addressLookupTables, securityAdmin, depositLimitQuote, ixGate, buybackFeesSwapMangoAccount, addressLookupTablesList, banksMapByName, banksMapByMint, banksMapByTokenIndex, serum3MarketsMapByExternal, serum3MarketsMapByMarketIndex, serum3ExternalMarketsMap, perpMarketsMapByOracle, perpMarketsMapByMarketIndex, perpMarketsMapByName, mintInfosMapByTokenIndex, mintInfosMapByMint, vaultAmountsMap) {
        this.publicKey = publicKey;
        this.creator = creator;
        this.groupNum = groupNum;
        this.admin = admin;
        this.fastListingAdmin = fastListingAdmin;
        this.mngoTokenIndex = mngoTokenIndex;
        this.insuranceMint = insuranceMint;
        this.insuranceVault = insuranceVault;
        this.testing = testing;
        this.version = version;
        this.buybackFees = buybackFees;
        this.buybackFeesMngoBonusFactor = buybackFeesMngoBonusFactor;
        this.addressLookupTables = addressLookupTables;
        this.securityAdmin = securityAdmin;
        this.depositLimitQuote = depositLimitQuote;
        this.ixGate = ixGate;
        this.buybackFeesSwapMangoAccount = buybackFeesSwapMangoAccount;
        this.addressLookupTablesList = addressLookupTablesList;
        this.banksMapByName = banksMapByName;
        this.banksMapByMint = banksMapByMint;
        this.banksMapByTokenIndex = banksMapByTokenIndex;
        this.serum3MarketsMapByExternal = serum3MarketsMapByExternal;
        this.serum3MarketsMapByMarketIndex = serum3MarketsMapByMarketIndex;
        this.serum3ExternalMarketsMap = serum3ExternalMarketsMap;
        this.perpMarketsMapByOracle = perpMarketsMapByOracle;
        this.perpMarketsMapByMarketIndex = perpMarketsMapByMarketIndex;
        this.perpMarketsMapByName = perpMarketsMapByName;
        this.mintInfosMapByTokenIndex = mintInfosMapByTokenIndex;
        this.mintInfosMapByMint = mintInfosMapByMint;
        this.vaultAmountsMap = vaultAmountsMap;
    }
    async reloadAll(client) {
        const ids = await client.getIds(this.publicKey);
        // console.time('group.reload');
        await Promise.all([
            this.reloadAlts(client),
            this.reloadBanks(client, ids).then(() => Promise.all([
                this.reloadBankOraclePrices(client),
                this.reloadVaults(client),
                this.reloadPerpMarkets(client, ids).then(() => this.reloadPerpMarketOraclePrices(client)),
            ])),
            this.reloadMintInfos(client, ids),
            this.reloadSerum3Markets(client, ids).then(() => this.reloadSerum3ExternalMarkets(client)),
        ]);
        // console.timeEnd('group.reload');
    }
    async reloadAlts(client) {
        const alts = await Promise.all(this.addressLookupTables
            .filter((alt) => !alt.equals(PublicKey.default))
            .map((alt) => client.program.provider.connection.getAddressLookupTable(alt)));
        this.addressLookupTablesList = alts.map((res, i) => {
            if (!res || !res.value) {
                throw new Error(`Undefined ALT ${this.addressLookupTables[i]}!`);
            }
            return res.value;
        });
    }
    async reloadBanks(client, ids) {
        let banks;
        if (ids && ids.getBanks().length) {
            banks = (await client.program.account.bank.fetchMultiple(ids.getBanks())).map((account, index) => Bank.from(ids.getBanks()[index], account));
        }
        else {
            banks = await client.getBanksForGroup(this);
        }
        const oldbanksMapByTokenIndex = cloneDeep(this.banksMapByTokenIndex);
        this.banksMapByName = new Map();
        this.banksMapByMint = new Map();
        this.banksMapByTokenIndex = new Map();
        for (const bank of banks) {
            // ensure that freshly fetched banks have valid price until we fetch oracles again
            const oldBanks = oldbanksMapByTokenIndex.get(bank.tokenIndex);
            if (oldBanks && oldBanks.length > 0) {
                merge(bank, oldBanks[0]);
            }
            const mintId = bank.mint.toString();
            if (this.banksMapByMint.has(mintId)) {
                this.banksMapByMint.get(mintId)?.push(bank);
                this.banksMapByName.get(bank.name)?.push(bank);
                this.banksMapByTokenIndex.get(bank.tokenIndex)?.push(bank);
            }
            else {
                this.banksMapByMint.set(mintId, [bank]);
                this.banksMapByName.set(bank.name, [bank]);
                this.banksMapByTokenIndex.set(bank.tokenIndex, [bank]);
            }
        }
    }
    async reloadMintInfos(client, ids) {
        let mintInfos;
        if (ids && ids.getMintInfos().length) {
            mintInfos = (await client.program.account.mintInfo.fetchMultiple(ids.getMintInfos())).map((account, index) => MintInfo.from(ids.getMintInfos()[index], account));
        }
        else {
            mintInfos = await client.getMintInfosForGroup(this);
        }
        this.mintInfosMapByTokenIndex = new Map(mintInfos.map((mintInfo) => {
            return [mintInfo.tokenIndex, mintInfo];
        }));
        this.mintInfosMapByMint = new Map(mintInfos.map((mintInfo) => {
            return [mintInfo.mint.toString(), mintInfo];
        }));
    }
    async reloadSerum3Markets(client, ids) {
        let serum3Markets;
        if (ids && ids.getSerum3Markets().length) {
            serum3Markets = (await client.program.account.serum3Market.fetchMultiple(ids.getSerum3Markets())).map((account, index) => Serum3Market.from(ids.getSerum3Markets()[index], account));
        }
        else {
            serum3Markets = await client.serum3GetMarkets(this);
        }
        this.serum3MarketsMapByExternal = new Map(serum3Markets.map((serum3Market) => [
            serum3Market.serumMarketExternal.toBase58(),
            serum3Market,
        ]));
        this.serum3MarketsMapByMarketIndex = new Map(serum3Markets.map((serum3Market) => [
            serum3Market.marketIndex,
            serum3Market,
        ]));
    }
    async reloadSerum3ExternalMarkets(client) {
        const externalMarkets = await Promise.all(Array.from(this.serum3MarketsMapByExternal.values()).map((serum3Market) => Market.load(client.program.provider.connection, serum3Market.serumMarketExternal, { commitment: client.program.provider.connection.commitment }, OPENBOOK_PROGRAM_ID[client.cluster])));
        this.serum3ExternalMarketsMap = new Map(Array.from(this.serum3MarketsMapByExternal.values()).map((serum3Market, index) => [
            serum3Market.serumMarketExternal.toBase58(),
            externalMarkets[index],
        ]));
    }
    async reloadPerpMarkets(client, ids) {
        let perpMarkets;
        if (ids && ids.getPerpMarkets().length) {
            perpMarkets = (await client.program.account.perpMarket.fetchMultiple(ids.getPerpMarkets())).map((account, index) => PerpMarket.from(ids.getPerpMarkets()[index], account));
        }
        else {
            perpMarkets = await client.perpGetMarkets(this);
        }
        // ensure that freshly fetched perp markets have valid price until we fetch oracles again
        const oldPerpMarketByMarketIndex = cloneDeep(this.perpMarketsMapByMarketIndex);
        for (const perpMarket of perpMarkets) {
            const oldPerpMarket = oldPerpMarketByMarketIndex.get(perpMarket.perpMarketIndex);
            if (oldPerpMarket) {
                merge(perpMarket, oldPerpMarket);
            }
        }
        this.perpMarketsMapByName = new Map(perpMarkets.map((perpMarket) => [perpMarket.name, perpMarket]));
        this.perpMarketsMapByOracle = new Map(perpMarkets.map((perpMarket) => [
            perpMarket.oracle.toBase58(),
            perpMarket,
        ]));
        this.perpMarketsMapByMarketIndex = new Map(perpMarkets.map((perpMarket) => [perpMarket.perpMarketIndex, perpMarket]));
    }
    async reloadBankOraclePrices(client) {
        const banks = Array.from(this.banksMapByMint, ([, value]) => value);
        const oracles = banks.map((b) => b[0].oracle);
        const ais = await client.program.provider.connection.getMultipleAccountsInfo(oracles);
        const coder = new BorshAccountsCoder(client.program.idl);
        for (const [index, ai] of ais.entries()) {
            for (const bank of banks[index]) {
                if (bank.name === 'USDC') {
                    bank._price = ONE_I80F48();
                    bank._uiPrice = 1;
                }
                else {
                    if (!ai)
                        throw new Error(`Undefined accountInfo object in reloadBankOraclePrices for ${bank.oracle}!`);
                    const { price, uiPrice, lastUpdatedSlot } = await this.decodePriceFromOracleAi(coder, bank.oracle, ai, this.getMintDecimals(bank.mint), client);
                    bank._price = price;
                    bank._uiPrice = uiPrice;
                    bank._oracleLastUpdatedSlot = lastUpdatedSlot;
                }
            }
        }
    }
    async reloadPerpMarketOraclePrices(client) {
        const perpMarkets = Array.from(this.perpMarketsMapByName.values());
        const oracles = perpMarkets.map((b) => b.oracle);
        const ais = await client.program.provider.connection.getMultipleAccountsInfo(oracles);
        const coder = new BorshAccountsCoder(client.program.idl);
        await Promise.all(Array.from(ais.entries()).map(async ([i, ai]) => {
            const perpMarket = perpMarkets[i];
            if (!ai)
                throw new Error(`Undefined ai object in reloadPerpMarketOraclePrices for ${perpMarket.oracle}!`);
            const { price, uiPrice, lastUpdatedSlot } = await this.decodePriceFromOracleAi(coder, perpMarket.oracle, ai, perpMarket.baseDecimals, client);
            perpMarket._price = price;
            perpMarket._uiPrice = uiPrice;
            perpMarket._oracleLastUpdatedSlot = lastUpdatedSlot;
        }));
    }
    async decodePriceFromOracleAi(coder, oracle, ai, baseDecimals, client) {
        let price, uiPrice, lastUpdatedSlot;
        if (!BorshAccountsCoder.accountDiscriminator('stubOracle').compare(ai.data.slice(0, 8))) {
            const stubOracle = coder.decode('stubOracle', ai.data);
            price = new I80F48(stubOracle.price.val);
            uiPrice = this.toUiPrice(price, baseDecimals);
            lastUpdatedSlot = stubOracle.lastUpdated.val;
        }
        else if (isPythOracle(ai)) {
            const priceData = parsePriceData(ai.data);
            uiPrice = priceData.previousPrice;
            price = this.toNativePrice(uiPrice, baseDecimals);
            lastUpdatedSlot = parseInt(priceData.lastSlot.toString());
        }
        else if (isSwitchboardOracle(ai)) {
            const priceData = await parseSwitchboardOracle(ai, client.program.provider.connection);
            uiPrice = priceData.price;
            price = this.toNativePrice(uiPrice, baseDecimals);
            lastUpdatedSlot = priceData.lastUpdatedSlot;
        }
        else {
            throw new Error(`Unknown oracle provider (parsing not implemented) for oracle ${oracle}, with owner ${ai.owner}!`);
        }
        return { price, uiPrice, lastUpdatedSlot };
    }
    async reloadVaults(client) {
        const vaultPks = Array.from(this.banksMapByMint.values())
            .flat()
            .map((bank) => bank.vault);
        const vaultAccounts = await client.program.provider.connection.getMultipleAccountsInfo(vaultPks);
        const coder = new BorshAccountsCoder(client.program.idl);
        this.vaultAmountsMap = new Map(vaultAccounts.map((vaultAi, i) => {
            if (!vaultAi) {
                throw new Error(`Undefined vaultAi for ${vaultPks[i]}`);
            }
            const vaultAmount = unpackAccount(vaultPks[i], vaultAi, TOKEN_PROGRAM_ID).amount;
            return [vaultPks[i].toBase58(), new BN(Number(vaultAmount))];
        }));
    }
    getMintDecimals(mintPk) {
        const bank = this.getFirstBankByMint(mintPk);
        return bank.mintDecimals;
    }
    getMintDecimalsByTokenIndex(tokenIndex) {
        const bank = this.getFirstBankByTokenIndex(tokenIndex);
        return bank.mintDecimals;
    }
    getInsuranceMintDecimals() {
        return this.getMintDecimals(this.insuranceMint);
    }
    getFirstBankByMint(mintPk) {
        const banks = this.banksMapByMint.get(mintPk.toString());
        if (!banks)
            throw new Error(`No bank found for mint ${mintPk}!`);
        return banks[0];
    }
    getFirstBankByTokenIndex(tokenIndex) {
        const banks = this.banksMapByTokenIndex.get(tokenIndex);
        if (!banks)
            throw new Error(`No bank found for tokenIndex ${tokenIndex}!`);
        return banks[0];
    }
    getFirstBankForMngo() {
        return this.getFirstBankByTokenIndex(this.mngoTokenIndex);
    }
    getFirstBankForPerpSettlement() {
        return this.getFirstBankByTokenIndex(0);
    }
    /**
     *
     * @param mintPk
     * @returns sum of ui balances of vaults for all banks for a token
     */
    getTokenVaultBalanceByMintUi(mintPk) {
        const banks = this.banksMapByMint.get(mintPk.toBase58());
        if (!banks) {
            throw new Error(`No bank found for mint ${mintPk}!`);
        }
        const totalAmount = new BN(0);
        for (const bank of banks) {
            const amount = this.vaultAmountsMap.get(bank.vault.toBase58());
            if (!amount) {
                throw new Error(`Vault balance not found for bank ${bank.name} ${bank.bankNum}!`);
            }
            totalAmount.iadd(amount);
        }
        return toUiDecimals(totalAmount, this.getMintDecimals(mintPk));
    }
    getSerum3MarketByMarketIndex(marketIndex) {
        const serum3Market = this.serum3MarketsMapByMarketIndex.get(marketIndex);
        if (!serum3Market) {
            throw new Error(`No serum3Market found for marketIndex ${marketIndex}!`);
        }
        return serum3Market;
    }
    getSerum3MarketByName(name) {
        const serum3Market = Array.from(this.serum3MarketsMapByExternal.values()).find((serum3Market) => serum3Market.name === name);
        if (!serum3Market) {
            throw new Error(`No serum3Market found by name ${name}!`);
        }
        return serum3Market;
    }
    getSerum3MarketByExternalMarket(externalMarketPk) {
        const serum3Market = Array.from(this.serum3MarketsMapByExternal.values()).find((serum3Market) => serum3Market.serumMarketExternal.equals(externalMarketPk));
        if (!serum3Market) {
            throw new Error(`No serum3Market found for external serum3 market ${externalMarketPk.toString()}!`);
        }
        return serum3Market;
    }
    getSerum3ExternalMarket(externalMarketPk) {
        const market = this.serum3ExternalMarketsMap.get(externalMarketPk.toBase58());
        if (!market) {
            throw new Error(`No external market found for pk ${externalMarketPk.toString()}!`);
        }
        return market;
    }
    async loadSerum3BidsForMarket(client, externalMarketPk) {
        const serum3Market = this.getSerum3MarketByExternalMarket(externalMarketPk);
        return await serum3Market.loadBids(client, this);
    }
    async loadSerum3AsksForMarket(client, externalMarketPk) {
        const serum3Market = this.getSerum3MarketByExternalMarket(externalMarketPk);
        return await serum3Market.loadAsks(client, this);
    }
    findPerpMarket(marketIndex) {
        const perpMarket = Array.from(this.perpMarketsMapByName.values()).find((perpMarket) => perpMarket.perpMarketIndex === marketIndex);
        if (!perpMarket) {
            throw new Error(`No perpMarket found for perpMarketIndex ${marketIndex}!`);
        }
        return perpMarket;
    }
    getPerpMarketByOracle(oracle) {
        const perpMarket = this.perpMarketsMapByOracle.get(oracle.toBase58());
        if (!perpMarket) {
            throw new Error(`No PerpMarket found for oracle ${oracle}!`);
        }
        return perpMarket;
    }
    getPerpMarketByMarketIndex(marketIndex) {
        const perpMarket = this.perpMarketsMapByMarketIndex.get(marketIndex);
        if (!perpMarket) {
            throw new Error(`No PerpMarket found with marketIndex ${marketIndex}!`);
        }
        return perpMarket;
    }
    getPerpMarketByName(perpMarketName) {
        const perpMarket = Array.from(this.perpMarketsMapByMarketIndex.values()).find((perpMarket) => perpMarket.name === perpMarketName);
        if (!perpMarket) {
            throw new Error(`No PerpMarket found by name ${perpMarketName}!`);
        }
        return perpMarket;
    }
    async loadPerpBidsForMarket(client, perpMarketIndex) {
        const perpMarket = this.getPerpMarketByMarketIndex(perpMarketIndex);
        return await perpMarket.loadBids(client);
    }
    async loadPerpAsksForMarket(client, group, perpMarketIndex) {
        const perpMarket = this.getPerpMarketByMarketIndex(perpMarketIndex);
        return await perpMarket.loadAsks(client);
    }
    consoleLogBanks() {
        for (const mintBanks of this.banksMapByMint.values()) {
            for (const bank of mintBanks) {
                console.log(bank.toString());
            }
        }
    }
    toUiPrice(price, baseDecimals) {
        return toUiDecimals(price, this.getInsuranceMintDecimals() - baseDecimals);
    }
    toNativePrice(uiPrice, baseDecimals) {
        return toNativeI80F48(uiPrice, 
        // note: our oracles are quoted in USD and our insurance mint is USD
        // please update when these assumptions change
        this.getInsuranceMintDecimals() - baseDecimals);
    }
    toNativeDecimals(uiAmount, mintPk) {
        const decimals = this.getMintDecimals(mintPk);
        return toNative(uiAmount, decimals);
    }
    toString() {
        let res = 'Group\n';
        res = res + ' pk: ' + this.publicKey.toString();
        res =
            res +
                '\n mintInfos:' +
                Array.from(this.mintInfosMapByTokenIndex.entries())
                    .map((mintInfoTuple) => '  \n' + mintInfoTuple[0] + ') ' + mintInfoTuple[1].toString())
                    .join(', ');
        const banks = [];
        for (const tokenBanks of this.banksMapByMint.values()) {
            for (const bank of tokenBanks) {
                banks.push(bank);
            }
        }
        res =
            res +
                '\n banks:' +
                Array.from(banks)
                    .map((bank) => '  \n' + bank.name + ') ' + bank.toString())
                    .join(', ');
        return res;
    }
}
