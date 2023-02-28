///
/// debugging
///
export function debugAccountMetas(ams) {
    for (const am of ams) {
        console.log(`${am.pubkey.toBase58()}, isSigner: ${am.isSigner
            .toString()
            .padStart(5, ' ')}, isWritable - ${am.isWritable
            .toString()
            .padStart(5, ' ')}`);
    }
}
export function debugHealthAccounts(group, mangoAccount, publicKeys) {
    const banks = new Map(Array.from(group.banksMapByName.values()).map((banks) => [
        banks[0].publicKey.toBase58(),
        `${banks[0].name} bank`,
    ]));
    const oracles = new Map(Array.from(group.banksMapByName.values()).map((banks) => [
        banks[0].oracle.toBase58(),
        `${banks[0].name} oracle`,
    ]));
    const serum3 = new Map(mangoAccount.serum3Active().map((serum3) => {
        const serum3Market = Array.from(group.serum3MarketsMapByExternal.values()).find((serum3Market) => serum3Market.marketIndex === serum3.marketIndex);
        if (!serum3Market) {
            throw new Error(`Serum3Orders for non existent market with market index ${serum3.marketIndex}`);
        }
        return [serum3.openOrders.toBase58(), `${serum3Market.name} spot oo`];
    }));
    const perps = new Map(Array.from(group.perpMarketsMapByName.values()).map((perpMarket) => [
        perpMarket.publicKey.toBase58(),
        `${perpMarket.name} perp market`,
    ]));
    publicKeys.map((pk) => {
        if (banks.get(pk.toBase58())) {
            console.log(banks.get(pk.toBase58()));
        }
        if (oracles.get(pk.toBase58())) {
            console.log(oracles.get(pk.toBase58()));
        }
        if (serum3.get(pk.toBase58())) {
            console.log(serum3.get(pk.toBase58()));
        }
        if (perps.get(pk.toBase58())) {
            console.log(perps.get(pk.toBase58()));
        }
    });
}
