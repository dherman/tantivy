// This module is the CJS entry point for the library.

// The Rust addon.
import * as addon from './load.cjs';

// Use this declaration to assign types to the addon's exports,
// which otherwise by default are `any`.
declare module "./load.cjs" {
  interface BoxedSchema {}
  interface BoxedIndex {}
  interface BoxedSearcher {}

  type SearchResult = [number, string];

  function newSchema(schema: SchemaDescriptor): BoxedSchema;
  function newIndex(path: string, heapSize: number, schema: BoxedSchema, reload_on: string): BoxedIndex;
  function addDocument(index: BoxedIndex, doc: string): BigInt;
  function commitSync(index: BoxedIndex): void;
  function reloadSync(index: BoxedIndex): void;
  function newSearcher(index: BoxedIndex): BoxedSearcher;
  function topDocsSync(searcher: BoxedSearcher, query: string, fields: number[], limit: number): SearchResult[];
}

export type TextOption = 'TEXT' | 'STORED' | 'STRING';
export type TextOptions = TextOption[];
export type Field = number;

export type SchemaDescriptor = {
  [key: string]: TextOptions
};

export type FieldMap = {
  [key: string]: number
};

const _BOXED_SCHEMA: unique symbol = Symbol();

export class Schema {
  [_BOXED_SCHEMA]: addon.BoxedSchema;
  private _fields: FieldMap;

  constructor(descriptor: SchemaDescriptor) {
    this[_BOXED_SCHEMA] = addon.newSchema(descriptor);
    this._fields = Object.create(null);
    let i = 0;
    for (let key in descriptor) {
      this._fields[key] = i;
      i++;
    }
    Object.freeze(this._fields);
  }

  get fields(): FieldMap {
    return this._fields;
  }
}

export type ReloadPolicy = 'COMMIT_WITH_DELAY' | 'MANUAL';

export type CreateIndexOptions = {
  path: string,
  schema: Schema,
  heapSize?: number,
  reloadOn?: ReloadPolicy,
}

const _BOXED_INDEX: unique symbol = Symbol();

export class Index {
  [_BOXED_INDEX]: addon.BoxedIndex;
  private _schema: Schema;

  constructor(options: CreateIndexOptions) {
    this[_BOXED_INDEX] = addon.newIndex(
      options.path,
      options.heapSize || 10_000_000,
      options.schema[_BOXED_SCHEMA],
      options.reloadOn || 'COMMIT_WITH_DELAY'
    );
    this._schema = options.schema;
  }

  schema(): Schema {
    return this._schema;
  }

  addDocument(doc: any): BigInt {
    return addon.addDocument(this[_BOXED_INDEX], JSON.stringify(doc));
  }

  commitSync(): void {
    addon.commitSync(this[_BOXED_INDEX]);
  }

  reloadSync(): void {
    addon.reloadSync(this[_BOXED_INDEX]);
  }

  searcher(): Searcher {
    return new Searcher(this, addon.newSearcher(this[_BOXED_INDEX]));
  }
}

export type SearchOptions = {
  fields: (string | number)[]
  top?: number
}

export type SearchResult = addon.SearchResult;

const _BOXED_SEARCHER: unique symbol = Symbol();

export class Searcher {
  private _index: Index;
  [_BOXED_SEARCHER]: addon.BoxedSearcher;

  constructor(index: Index, boxedSearcher: addon.BoxedSearcher) {
    this._index = index;
    this[_BOXED_SEARCHER] = boxedSearcher;
  }

  private interpretFields(fields: (string | number)[]): number[] {
    const fieldsMap = this._index.schema().fields;
    return fields.map(field => {
      return (typeof field === 'string') ? fieldsMap[field] : field;
    });
  }

  searchSync(query: string, options: SearchOptions): SearchResult[] {
    if (!options.top) {
      throw new Error("only top search is implemented");
    }
    return addon.topDocsSync(
      this[_BOXED_SEARCHER],
      query,
      this.interpretFields(options.fields),
      options.top
    );
  }
}
