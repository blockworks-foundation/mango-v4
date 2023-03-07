import { PublicKey } from '@solana/web3.js';
import fetch from 'cross-fetch';
import ids from '../ids.json';
export class Id {
    cluster;
    name;
    publicKey;
    serum3ProgramId;
    mangoProgramId;
    banks;
    stubOracles;
    mintInfos;
    serum3Markets;
    perpMarkets;
    constructor(cluster, name, publicKey, serum3ProgramId, mangoProgramId, banks, stubOracles, mintInfos, serum3Markets, perpMarkets) {
        this.cluster = cluster;
        this.name = name;
        this.publicKey = publicKey;
        this.serum3ProgramId = serum3ProgramId;
        this.mangoProgramId = mangoProgramId;
        this.banks = banks;
        this.stubOracles = stubOracles;
        this.mintInfos = mintInfos;
        this.serum3Markets = serum3Markets;
        this.perpMarkets = perpMarkets;
    }
    getBanks() {
        return Array.from(this.banks
            .filter((perpMarket) => perpMarket.active)
            .map((bank) => new PublicKey(bank.publicKey)));
    }
    getStubOracles() {
        return Array.from(this.stubOracles.map((stubOracle) => new PublicKey(stubOracle.publicKey)));
    }
    getMintInfos() {
        return Array.from(this.mintInfos.map((mintInfo) => new PublicKey(mintInfo.publicKey)));
    }
    getSerum3Markets() {
        return Array.from(this.serum3Markets
            .filter((perpMarket) => perpMarket.active)
            .map((serum3Market) => new PublicKey(serum3Market.publicKey)));
    }
    getPerpMarkets() {
        return Array.from(this.perpMarkets
            .filter((perpMarket) => perpMarket.active)
            .map((perpMarket) => new PublicKey(perpMarket.publicKey)));
    }
    static fromIdsByName(name) {
        const groupConfig = ids.groups.find((id) => id['name'] === name);
        if (!groupConfig)
            throw new Error(`No group config ${name} found in Ids!`);
        return new Id(groupConfig.cluster, groupConfig.name, groupConfig.publicKey, groupConfig.serum3ProgramId, groupConfig.mangoProgramId, groupConfig['banks'], groupConfig['stubOracles'], groupConfig['mintInfos'], groupConfig['serum3Markets'], groupConfig['perpMarkets']);
    }
    static fromIdsByPk(groupPk) {
        const groupConfig = ids.groups.find((id) => id['publicKey'] === groupPk.toString());
        if (!groupConfig)
            throw new Error(`No group config ${groupPk.toString()} found in Ids!`);
        return new Id(groupConfig.cluster, groupConfig.name, groupConfig.publicKey, groupConfig.serum3ProgramId, groupConfig.mangoProgramId, groupConfig['banks'], groupConfig['stubOracles'], groupConfig['mintInfos'], groupConfig['serum3Markets'], groupConfig['perpMarkets']);
    }
    static async fromApi(groupPk) {
        const groupMetadataApiUrl = 'https://mango-transaction-log.herokuapp.com/v4/group-metadata';
        const response = await fetch(groupMetadataApiUrl);
        const jsonData = await response.json();
        const groupConfig = jsonData.groups.find((group) => group.publicKey === groupPk.toString());
        return new Id(groupConfig.cluster, groupConfig.name, groupConfig.publicKey, groupConfig.serum3ProgramId, groupConfig.mangoProgramId, groupConfig.tokens.flatMap((t) => t.banks.map((b) => ({
            name: t.symbol,
            mint: t.mint,
            tokenIndex: t.tokenIndex,
            bankNum: b.bankNum,
            publicKey: b.publicKey,
        }))), groupConfig.stubOracles.map((s) => ({
            mint: s.mint,
            publicKey: s.publicKey,
        })), groupConfig.tokens.map((t) => ({
            name: t.symbol,
            mint: t.mint,
            tokenIndex: t.tokenIndex,
            publicKey: t.mintInfo,
        })), groupConfig.serum3Markets.map((s) => ({
            name: s.name,
            publicKey: s.publicKey,
            marketExternal: s.marketExternal,
        })), groupConfig.perpMarkets.map((p) => ({
            name: p.name,
            publicKey: p.publicKey,
        })));
    }
}
