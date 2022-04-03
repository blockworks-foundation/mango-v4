//
// group
//

//
// token / bank
//

//
// mango account
//

//
// deposit & withdraw
//

//
// Serum3 instructions
//

// export async function serum3PlaceOrder(
//   client: MangoClient,
//   side: Serum3Side,
//   limitPrice: number,
//   maxBaseQty: number,
//   maxNativeQuoteQtyIncludingFees: number,
//   selfTradeBehavior: Serum3SelfTradeBehavior,
//   orderType: Serum3OrderType,
//   clientOrderId: number,
//   limit: number,
// ): Promise<void> {
//   return await client.program.methods
//     .serum3PlaceOrder(
//       side,
//       limitPrice,
//       maxBaseQty,
//       maxNativeQuoteQtyIncludingFees,
//       selfTradeBehavior,
//       orderType,
//       clientOrderId,
//       limit,
//     )
//     .accounts({
//       group: groupPk,
//       admin: adminPk,
//       serumProgram: serumProgramPk,
//       serumMarketExternal: serumMarketExternalPk,
//       quoteBank: quoteBankPk,
//       baseBank: baseBankPk,
//       payer: payer.publicKey,
//     })
//     .rpc();
// }

//
// Oracle
//
