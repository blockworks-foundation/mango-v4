import { Program, Provider } from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';
import { IDL, MangoV4 } from './mango_v4';

export const MANGO_V4_ID = new PublicKey(
  'm43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD',
);

export class MangoClient {
  constructor(public program: Program<MangoV4>, public devnet?: boolean) {}

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
    addDummyType(idl, 'instructions::NewOrderInstructionData');
    addDummyType(idl, 'instructions::CancelOrderInstructionData');
    addDummyType(idl, 'H');
    addDummyType(idl, 'H::Item');
    addDummyType(idl, 'NodeHandle');

    return new MangoClient(
      new Program<MangoV4>(idl as MangoV4, MANGO_V4_ID, provider),
      devnet,
    );
  }
}
