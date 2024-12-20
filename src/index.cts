// This module is the CJS entry point for the library.

// The Rust addon.
import * as addon from './load.cjs';

// Use this declaration to assign types to the addon's exports,
// which otherwise by default are `any`.
declare module "./load.cjs" {
  interface BoxedSchema {}
  interface BoxedIndex {}
  interface BoxedSearcher {}
  interface BoxedQuery {}

  type SearchResult = [number, string];

  function newSchema(schema: SchemaDescriptor): BoxedSchema;
  function newIndex(path: string, heapSize: number, schema: BoxedSchema, reload_on: string): BoxedIndex;
  function addDocument(index: BoxedIndex, doc: string): BigInt;
  function commit(index: BoxedIndex): Promise<void>;
  function commitSync(index: BoxedIndex): void;
  function reload(index: BoxedIndex): Promise<void>;
  function reloadSync(index: BoxedIndex): void;
  function parseQuery(searcher: BoxedSearcher, source: string, fields: number[]): BoxedQuery;
  function newSearcher(index: BoxedIndex): BoxedSearcher;
  function newRegexQuery(pattern: string, field: number): BoxedQuery;
  function newPhrasePrefixQuery(terms: string[], field: number): BoxedQuery;
  function newFuzzyTermQuery(term: string, field: number, maxDistance: number, transpositionCostsOne: boolean, isPrefix: boolean): BoxedQuery;
  function topDocs(searcher: BoxedSearcher, query: BoxedQuery, limit: number): Promise<SearchResult[]>;
  function topDocsSync(searcher: BoxedSearcher, query: BoxedQuery, limit: number): SearchResult[];
}

export type TextFieldDescriptor = {
  type: "text",
  flags?: TextOption[],
};

export type StringFieldDescriptor = {
  type: "string",
  flags?: TextOption[],
};

export type F64FieldDescriptor = {
  type: "f64",
  flags?: NumericOption[],
};

// TODO: | I64FieldDescriptor
// TODO: | U64FieldDescriptor
// TODO: | DateFieldDescriptor
// TODO: | BoolFieldDescriptor
// TODO: | IpAddrFieldDescriptor
export type FieldDescriptor =
  TextFieldDescriptor
  | StringFieldDescriptor
  | F64FieldDescriptor;

export type TextOption = 'STORED';
export type NumericOption = 'STORED' | 'INDEXED';

export type Field = number;

export type SchemaDescriptor = {
  [key: string]: FieldDescriptor
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

const _BOXED_QUERY: unique symbol = Symbol();

export class Query {
  [_BOXED_QUERY]: addon.BoxedQuery;

  constructor(boxedQuery: addon.BoxedQuery) {
    this[_BOXED_QUERY] = boxedQuery;
  }
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

  async commit(): Promise<void> {
    await addon.commit(this[_BOXED_INDEX]);
  }

  async reload(): Promise<void> {
    await addon.reload(this[_BOXED_INDEX]);
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

export type FuzzyTermQueryOptions = {
  maxDistance?: number,
  transpositionCostsOne?: boolean,
  isPrefix?: boolean,
};

export class Searcher {
  private _index: Index;
  [_BOXED_SEARCHER]: addon.BoxedSearcher;

  constructor(index: Index, boxedSearcher: addon.BoxedSearcher) {
    this._index = index;
    this[_BOXED_SEARCHER] = boxedSearcher;
  }

  private interpretField(field: string | number): number {
    return typeof field === 'string' ? this._index.schema().fields[field] : field;
  }

  private interpretFields(fields: (string | number)[]): number[] {
    const fieldsMap = this._index.schema().fields;
    return fields.map(field => {
      return (typeof field === 'string') ? fieldsMap[field] : field;
    });
  }

  fuzzyTermQuery(term: string, field: string, options: FuzzyTermQueryOptions = {}): Query {
    const maxDistance = options.maxDistance || 0;
    const transpositionCostsOne = (typeof options.transpositionCostsOne === 'boolean') ? options.transpositionCostsOne : true;
    const isPrefix = (typeof options.isPrefix === 'boolean') ? options.isPrefix : false;
    return new Query(addon.newFuzzyTermQuery(term, this.interpretField(field), maxDistance, transpositionCostsOne, isPrefix));
  }

  regexpQuery(pattern: string, field: string): Query {
    return new Query(addon.newRegexQuery(pattern, this.interpretField(field)));
  }

  phrasePrefixQuery(terms: string[], field: string): Query {
    return new Query(addon.newPhrasePrefixQuery(terms, this.interpretField(field)));
  }

  searchSync(query: string | Query, options: SearchOptions): SearchResult[] {
    if (!options.top) {
      throw new Error("only top search is implemented");
    }
    if (typeof query === 'string') {
      const fields = this.interpretFields(options.fields);
      query = new Query(addon.parseQuery(this[_BOXED_SEARCHER], query, fields));
    }
    return addon.topDocsSync(
      this[_BOXED_SEARCHER],
      query[_BOXED_QUERY],
      options.top
    );
  }

  async search(query: string | Query, options: SearchOptions): Promise<SearchResult[]> {
    if (!options.top) {
      throw new Error("only top search is implemented");
    }
    if (typeof query === 'string') {
      const fields = this.interpretFields(options.fields);
      query = new Query(addon.parseQuery(this[_BOXED_SEARCHER], query, fields));
    }
    return addon.topDocs(
      this[_BOXED_SEARCHER],
      query[_BOXED_QUERY],
      options.top
    );
  }
}
