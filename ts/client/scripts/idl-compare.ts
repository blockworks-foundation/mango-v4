import { Idl, IdlError } from '@coral-xyz/anchor';
import { IdlField, IdlType, IdlTypeDef } from '@coral-xyz/anchor/dist/cjs/idl';
import fs from 'fs';

const ignoredIx = [
  'tokenRegister',
  'groupEdit',
  'tokenEdit',
  'openbookV2EditMarket',
  'openbookV2RegisterMarket',
];

const emptyFieldPrefixes = ['padding', 'reserved'];

const skippedErrors = {
  '0.25.0': [
    ['Instruction', 'openbookV2CreateOpenOrders'],
    ['Instruction', 'openbookV2PlaceOrder'],
    ['Instruction', 'openbookV2PlaceTakerOrder'],
    ['Instruction', 'openbookV2CancelAllOrders'],
    ['Account', 'OpenbookV2Market'],
  ],
};

function skipError(newIdl, errorTuple): boolean {
  const errors = skippedErrors[newIdl.version];
  if (!errors) {
    return false;
  }
  return errors.some(
    (a) =>
      a.length == errorTuple.length &&
      a.every((value, index) => value === errorTuple[index]),
  );
}

function isEmptyField(name: string): boolean {
  return emptyFieldPrefixes.some((s) => name.startsWith(s));
}

function main(): void {
  let hasError = false;

  const oldIdl = JSON.parse(fs.readFileSync(process.argv[2], 'utf-8')) as Idl;
  const newIdl = JSON.parse(fs.readFileSync(process.argv[3], 'utf-8')) as Idl;

  // Old instructions still exist
  for (const oldIx of oldIdl.instructions) {
    if (skipError(newIdl, ['Instruction', oldIx.name])) {
      continue;
    }
    const newIx = newIdl.instructions.find((x) => x.name == oldIx.name);
    if (!newIx) {
      console.log(`Error: instruction '${oldIx.name}' was removed`);
      hasError = true;
      continue;
    }

    if (ignoredIx.includes(oldIx.name)) {
      continue;
    }

    if (
      fieldsHaveErrorIx(
        oldIx.args,
        newIx.args,
        `instruction ${oldIx.name}`,
        oldIdl,
        newIdl,
      )
    ) {
      hasError = true;
    }
  }

  for (const oldType of oldIdl.types ?? []) {
    const newType = newIdl.types?.find((x) => x.name == oldType.name);

    if (!newType) {
      console.log(`Warning: type '${oldType.name}' was removed`);
      continue;
    }

    if (oldType.type.kind !== newType?.type.kind) {
      console.log(`Error: type '${oldType.name}' has changed kind`);
      hasError = true;
      continue;
    }

    const oldSize = accountSize(oldIdl, oldType);
    const newSize = accountSize(newIdl, newType);
    if (oldSize != newSize) {
      console.log(`Error: type ${oldType.name}' has changed size`);
      hasError = true;
    }

    if (oldType.type.kind === 'struct' && newType.type.kind === 'struct') {
      if (
        fieldsHaveError(
          oldType.type.fields,
          newType.type.fields,
          `type ${oldType.name}`,
          oldIdl,
          newIdl,
        )
      ) {
        hasError = true;
      }
    } else if (oldType.type.kind === 'enum' && newType.type.kind === 'enum') {
      const oldVariants = oldType.type.variants.map((v) => v.name);
      const newVariants = newType.type.variants.map((v) => v.name);

      if (newVariants.length < oldVariants.length) {
        console.log(
          `Error: enum ${oldType.name}' has fewer variants than before`,
        );
        hasError = true;
        continue;
      }

      for (let i = 0; i < oldVariants.length; i++) {
        if (oldVariants[i] !== newVariants[i]) {
          console.log(
            `Error: enum ${oldType.name}' has a changed variant: ${oldVariants[i]} vs ${newVariants[i]}`,
          );
          hasError = true;
        }
      }
    }
  }

  for (const oldAcc of oldIdl.accounts ?? []) {
    if (skipError(newIdl, ['Account', oldAcc.name])) {
      continue;
    }
    const newAcc = newIdl.accounts?.find((x) => x.name == oldAcc.name);

    // Old accounts still exist
    if (!newAcc) {
      console.log(`Error: account '${oldAcc.name}' was removed`);
      hasError = true;
      continue;
    }

    const oldSize = accountSize(oldIdl, oldAcc);
    const newSize = accountSize(newIdl, newAcc);
    if (
      oldSize != newSize &&
      !skipError(newIdl, ['AccountSize', oldAcc.name, oldSize, newSize])
    ) {
      console.log(`Error: account '${oldAcc.name}' has changed size`);
      hasError = true;
    }

    if (
      fieldsHaveError(
        oldAcc.type.fields,
        newAcc.type.fields,
        `account ${oldAcc.name}`,
        oldIdl,
        newIdl,
      )
    ) {
      hasError = true;
    }
  }

  process.exit(hasError ? 1 : 0);
}

main();

function fieldsHaveError(
  oldFields: IdlField[],
  newFields: IdlField[],
  context: string,
  oldIdl: Idl,
  newIdl: Idl,
): boolean {
  let hasError = false;
  for (const oldField of oldFields) {
    let newField = newFields.find((x) => x.name == oldField.name);

    if (isEmptyField(oldField.name)) {
      continue;
    }

    // Old fields may be renamed / deprecated
    const oldOffset = fieldOffset(oldFields, oldField, oldIdl);
    if (!newField) {
      // Try to find it by offset
      for (const field of newFields) {
        const offset = fieldOffset(newFields, field, newIdl);
        if (offset == oldOffset && !isEmptyField(field.name)) {
          console.log(
            `Warning: field '${oldField.name}' in ${context} was renamed(?) to ${field.name}`,
          );
          newField = field;
        }
      }
    }
    if (!newField) {
      console.log(
        `Warning: field '${oldField.name}' in ${context} was removed`,
      );
      continue;
    }

    // Fields may not change size
    const oldSize = typeSize(oldIdl, oldField.type);
    const newSize = typeSize(newIdl, newField.type);
    if (oldSize != newSize) {
      console.log(
        `Error: field '${oldField.name}' in ${context} has changed size`,
      );
      hasError = true;
    }

    // Fields may not change offset
    const newOffset = fieldOffset(newFields, newField, newIdl);
    if (oldOffset != newOffset) {
      console.log(
        `Error: field '${oldField.name}' in ${context} has changed offset`,
      );
      hasError = true;
    }
  }

  return hasError;
}

function fieldsHaveErrorIx(
  oldFields: IdlField[],
  newFields: IdlField[],
  context: string,
  oldIdl: Idl,
  newIdl: Idl,
): boolean {
  let hasError = false;
  const renameTargets: string[] = [];
  for (const oldField of oldFields) {
    let newField = newFields.find((x) => x.name == oldField.name);

    // Old fields may not be removed, but could be renamed
    const oldOffset = fieldOffset(oldFields, oldField, oldIdl);
    if (!newField) {
      // Try to find it by offset
      for (const field of newFields) {
        const offset = fieldOffset(newFields, field, newIdl);
        if (offset == oldOffset) {
          console.log(
            `Warning: field '${oldField.name}' in ${context} was renamed(?) to ${field.name}`,
          );
          renameTargets.push(field.name);
          newField = field;
        }
      }
    }
    if (!newField) {
      console.log(`Error: field '${oldField.name}' in ${context} was removed`);
      continue;
    }

    // Fields may not change size
    const oldSize = typeSize(oldIdl, oldField.type);
    const newSize = typeSize(newIdl, newField.type);
    if (oldSize != newSize) {
      console.log(
        `Error: field '${oldField.name}' in ${context} has changed size`,
      );
      hasError = true;
    }

    // Fields may not change offset
    const newOffset = fieldOffset(newFields, newField, newIdl);
    if (oldOffset != newOffset) {
      console.log(
        `Error: field '${oldField.name}' in ${context} has changed offset`,
      );
      hasError = true;
    }
  }

  for (const newField of newFields) {
    const oldField = oldFields.find((x) => x.name == newField.name);

    if (!oldField && !renameTargets.includes(newField.name)) {
      console.log(`Error: field '${newField.name}' in ${context} was added`);
      continue;
    }
  }

  return hasError;
}

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

export function accountSize(idl: Idl, idlAccount: IdlTypeDef) {
  switch (idlAccount.type.kind) {
    case 'struct': {
      return idlAccount.type.fields
        .map((f) => typeSize(idl, f.type))
        .reduce((acc, size) => acc + size, 0);
    }

    case 'enum': {
      const variantSizes = idlAccount.type.variants.map((variant) => {
        if (!variant.fields) {
          return 0;
        }
        return variant.fields
          .map((f: IdlField | IdlType) => {
            if (!(typeof f === 'object' && 'name' in f)) {
              return typeSize(idl, f);
            }
            return typeSize(idl, f.type);
          })
          .reduce((acc, size) => acc + size, 0);
      });

      return Math.max(...variantSizes) + 1;
    }

    case 'alias': {
      return typeSize(idl, idlAccount.type.value);
    }
  }
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
        return 4;
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
          throw new IdlError(`Type not found: ${JSON.stringify(ty)}`);
        }
        let typeDef = filtered[0];

        return accountSize(idl, typeDef);
      }
      if ('array' in ty) {
        let arrayTy = ty.array[0];
        let arraySize = ty.array[1];
        return typeSize(idl, arrayTy) * arraySize;
      }
      throw new Error(`Invalid type ${JSON.stringify(ty)}`);
  }
}
