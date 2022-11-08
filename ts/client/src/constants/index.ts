import { PublicKey } from '@solana/web3.js';

export const SERUM3_PROGRAM_ID = {
  testnet: new PublicKey('DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY'),
  devnet: new PublicKey('DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY'),
  'mainnet-beta': new PublicKey('9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin'),
};

export const MANGO_V4_ID = {
  testnet: new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD'),
  devnet: new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD'),
  'mainnet-beta': new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD'),
};

export const MSRM_MINTS = {
  testnet: new PublicKey('3Ho7PN3bYv9bp1JDErBD2FxsRepPkL88vju3oDX9c3Ez'),
  devnet: new PublicKey('8DJBo4bF4mHNxobjdax3BL9RMh5o71Jf8UiKsf5C5eVH'),
  'mainnet-beta': new PublicKey('MSRMcoVyrFxnSgo5uXwone5SKcGhT1KEJMFEkMEWf9L'),
};
