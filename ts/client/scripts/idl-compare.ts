import { Idl } from '@coral-xyz/anchor';
import {
  IdlEnumVariant,
  IdlField,
  IdlType,
  IdlTypeDef,
} from '@coral-xyz/anchor/dist/cjs/idl';
import fs from 'fs';

function main(): void {
  let hasError = false;

  const oldIdl = JSON.parse(fs.readFileSync(process.argv[2], 'utf-8')) as Idl;
  const newIdl = JSON.parse(fs.readFileSync(process.argv[3], 'utf-8')) as Idl;

  // Old instructions still exist
  for (const oldIx of oldIdl.instructions) {
    if (!newIdl.instructions.find((x) => x.name == oldIx.name)) {
      console.log(`Error: instruction '${oldIx.name}' was removed`);
      hasError = true;
    }
  }

  for (const oldAcc of oldIdl.accounts ?? []) {
    const newAcc = newIdl.accounts?.find((x) => x.name == oldAcc.name);

    // Old accounts still exist
    if (!newAcc) {
      console.log(`Error: account '${oldAcc.name}' was removed`);
      hasError = true;
      continue;
    }

    const oldSize = accountSize(oldIdl, oldAcc);
    const newSize = accountSize(newIdl, newAcc);
    if (oldSize != newSize) {
      console.log(`Error: account '${oldAcc.name}' has changed size`);
      hasError = true;
    }

    // Data offset matches for each account field with the same name
    const newFields = newAcc.type.fields;
    const oldFields = oldAcc.type.fields;
    for (const oldField of oldFields) {
      const newField = newFields.find((x) => x.name == oldField.name);

      if (
        oldField.name.startsWith('reserved') ||
        oldField.name.startsWith('padding')
      ) {
        continue;
      }

      // Old fields may be renamed / deprecated
      if (!newField) {
        console.log(
          `Warning: account field '${oldAcc.name}.${oldField.name}' was removed`,
        );
        continue;
      }

      // Fields may not change size
      const oldSize = typeSize(oldIdl, oldField.type);
      const newSize = typeSize(newIdl, newField.type);
      if (oldSize != newSize) {
        console.log(
          `Error: account field '${oldAcc.name}.${oldField.name}' has changed size`,
        );
        hasError = true;
      }

      // Fields may not change offset
      const oldOffset = fieldOffset(oldFields, oldField, oldIdl);
      const newOffset = fieldOffset(newFields, newField, newIdl);
      if (oldOffset != newOffset) {
        console.log(
          `Error: account field '${oldAcc.name}.${oldField.name}' has changed offset`,
        );
        hasError = true;
      }
    }
  }

  process.exit(hasError ? 1 : 0);
}

main();

function fieldOffset(fields: IdlField[], field: IdlField, idl: Idl): number {
  let offset = 0;
  for (const f of fields) {
    if (f.name == field.name) {
      break;
    }
    offset += typeSize(idl, f.type);
  }
  return offset;
}

//
// The following code is essentially copied from anchor's common.ts
//

export function accountSize(idl: Idl, idlAccount: IdlTypeDef): number {
  if (idlAccount.type.kind === 'enum') {
    const variantSizes = idlAccount.type.variants.map(
      (variant: IdlEnumVariant) => {
        if (variant.fields === undefined) {
          return 0;
        }
        return variant.fields
          .map((f: IdlField | IdlType) => {
            if (!(typeof f === 'object' && 'name' in f)) {
              throw new Error('Tuple enum variants not yet implemented.');
            }
            return typeSize(idl, f.type);
          })
          .reduce((a: number, b: number) => a + b);
      },
    );
    return Math.max(...variantSizes) + 1;
  }
  if (idlAccount.type.fields === undefined) {
    return 0;
  }
  return idlAccount.type.fields
    .map((f) => typeSize(idl, f.type))
    .reduce((a, b) => a + b, 0);
}

function typeSize(idl: Idl, ty: IdlType): number {
  switch (ty) {
    case 'bool':
      return 1;
    case 'u8':
      return 1;
    case 'i8':
      return 1;
    case 'i16':
      return 2;
    case 'u16':
      return 2;
    case 'u32':
      return 4;
    case 'i32':
      return 4;
    case 'f32':
      return 4;
    case 'u64':
      return 8;
    case 'i64':
      return 8;
    case 'f64':
      return 8;
    case 'u128':
      return 16;
    case 'i128':
      return 16;
    case 'u256':
      return 32;
    case 'i256':
      return 32;
    case 'bytes':
      return 1;
    case 'string':
      return 1;
    case 'publicKey':
      return 32;
    default:
      if ('vec' in ty) {
        return 1;
      }
      if ('option' in ty) {
        return 1 + typeSize(idl, ty.option);
      }
      if ('coption' in ty) {
        return 4 + typeSize(idl, ty.coption);
      }
      if ('defined' in ty) {
        const filtered = idl.types?.filter((t) => t.name === ty.defined) ?? [];
        if (filtered.length !== 1) {
          throw new Error(`Type not found: ${JSON.stringify(ty)}`);
        }
        const typeDef = filtered[0];

        return accountSize(idl, typeDef);
      }
      if ('array' in ty) {
        const arrayTy = ty.array[0];
        const arraySize = ty.array[1];
        return typeSize(idl, arrayTy) * arraySize;
      }
      throw new Error(`Invalid type ${JSON.stringify(ty)}`);
  }
}
