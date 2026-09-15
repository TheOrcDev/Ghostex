import { ClientStorageError, type JsonValue, type StorageCodec } from './types';

export const textCodec: StorageCodec<string> = {
  schema: 'text',
  decode: (raw) => raw,
  encode: (value) => {
    if (typeof value !== 'string') throw new TypeError('Expected text.');
    return value;
  },
};

export function enumCodec<const T extends string>(values: readonly T[]): StorageCodec<T> {
  const parse = (value: string): T => {
    if (!values.includes(value as T)) throw new TypeError(`Expected one of ${values.join(', ')}.`);
    return value as T;
  };
  return { schema: values.join(' | '), decode: parse, encode: parse };
}

export function jsonCodec<T = JsonValue>(schema: string, validate: (value: unknown) => value is T): StorageCodec<T> {
  return {
    schema,
    decode(raw) {
      const value: unknown = JSON.parse(raw);
      if (!validate(value)) throw new ClientStorageError('schema', '', `Invalid ${schema}.`);
      return value;
    },
    encode(value) {
      if (!validate(value)) throw new ClientStorageError('schema', '', `Invalid ${schema}.`);
      return JSON.stringify(value);
    },
  };
}
export const isObject = (value: unknown): value is Record<string, JsonValue> =>
  value !== null && typeof value === 'object' && !Array.isArray(value);
export const objectCodec = jsonCodec('JSON object', isObject);
export const arrayCodec = jsonCodec<JsonValue[]>('JSON array', (value): value is JsonValue[] => Array.isArray(value));
export const stringListCodec = jsonCodec<string[]>(
  'list of strings',
  (value): value is string[] => Array.isArray(value) && value.every((item) => typeof item === 'string')
);
