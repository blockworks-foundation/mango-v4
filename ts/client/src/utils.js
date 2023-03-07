import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from './utils/spl';
import { MessageV0, PublicKey, SystemProgram, TransactionInstruction, VersionedTransaction, } from '@solana/web3.js';
import BN from 'bn.js';
import { I80F48 } from './numbers/I80F48';
///
/// numeric helpers
///
export const U64_MAX_BN = new BN('18446744073709551615');
export const I64_MAX_BN = new BN('9223372036854775807').toTwos(64);
export function bpsToDecimal(bps) {
    return bps / 1000;
}
export function percentageToDecimal(percentage) {
    return percentage / 100;
}
export function toNativeI80F48ForQuote(uiAmount) {
    return I80F48.fromNumber(uiAmount * Math.pow(10, 6));
}
export function toNativeI80F48(uiAmount, decimals) {
    return I80F48.fromNumber(uiAmount * Math.pow(10, decimals));
}
export function toNative(uiAmount, decimals) {
    return new BN((uiAmount * Math.pow(10, decimals)).toFixed(0));
}
export function toUiDecimals(nativeAmount, decimals) {
    // TODO: remove BN and upgrade to bigint https://github.com/solana-labs/solana/issues/27440
    if (nativeAmount instanceof BN) {
        nativeAmount = I80F48.fromU64(nativeAmount);
    }
    if (nativeAmount instanceof I80F48) {
        return nativeAmount
            .div(I80F48.fromNumber(Math.pow(10, decimals)))
            .toNumber();
    }
    return nativeAmount / Math.pow(10, decimals);
}
export const QUOTE_DECIMALS = 6;
export function toUiDecimalsForQuote(nativeAmount) {
    return toUiDecimals(nativeAmount, QUOTE_DECIMALS);
}
export function toUiI80F48(nativeAmount, decimals) {
    return nativeAmount.div(I80F48.fromNumber(Math.pow(10, decimals)));
}
///
/// web3js extensions
///
/**
 * Get the address of the associated token account for a given mint and owner
 *
 * @param mint                     Token mint account
 * @param owner                    Owner of the new account
 * @param allowOwnerOffCurve       Allow the owner account to be a PDA (Program Derived Address)
 * @param programId                SPL Token program account
 * @param associatedTokenProgramId SPL Associated Token program account
 *
 * @return Address of the associated token account
 */
export async function getAssociatedTokenAddress(mint, owner, allowOwnerOffCurve = false, programId = TOKEN_PROGRAM_ID, associatedTokenProgramId = ASSOCIATED_TOKEN_PROGRAM_ID) {
    if (!allowOwnerOffCurve && !PublicKey.isOnCurve(owner.toBuffer()))
        throw new Error('TokenOwnerOffCurve!');
    const [address] = await PublicKey.findProgramAddress([owner.toBuffer(), programId.toBuffer(), mint.toBuffer()], associatedTokenProgramId);
    return address;
}
export async function createAssociatedTokenAccountIdempotentInstruction(payer, owner, mint) {
    const account = await getAssociatedTokenAddress(mint, owner);
    return new TransactionInstruction({
        keys: [
            { pubkey: payer, isSigner: true, isWritable: true },
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: owner, isSigner: false, isWritable: false },
            { pubkey: mint, isSigner: false, isWritable: false },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        ],
        programId: ASSOCIATED_TOKEN_PROGRAM_ID,
        data: Buffer.from([0x1]),
    });
}
export async function buildVersionedTx(provider, ix, additionalSigners = [], alts = []) {
    const message = MessageV0.compile({
        payerKey: provider.wallet.publicKey,
        instructions: ix,
        recentBlockhash: (await provider.connection.getLatestBlockhash()).blockhash,
        addressLookupTableAccounts: alts,
    });
    const vTx = new VersionedTransaction(message);
    vTx.sign([
        provider.wallet.payer,
        ...additionalSigners,
    ]);
    return vTx;
}
