// This module is the CJS entry point for the library.

// The Rust addon.
import * as addon from './load.cjs';

// Use this declaration to assign types to the addon's exports,
// which otherwise by default are `any`.
declare module "./load.cjs" {
  interface BoxedSchemaBuilder {}
  interface BoxedSchema {}
  interface BoxedIndex {}
  interface BoxedIndexWriter {}
  interface BoxedSearch {}
  interface BoxedQueryParser {}
  interface BoxedQuery {}
  interface BoxedTopDocs {}

  type SearchResult = [number, string];

  function newSchemaBuilder(): BoxedSchemaBuilder;
  function addTextField(builder: BoxedSchemaBuilder, name: string, options: TextOptions): Field;
  function buildSchema(builder: BoxedSchemaBuilder): BoxedSchema;
  function createIndex(schema: BoxedSchema, path: string): BoxedIndex;
  function createIndexWriter(index: BoxedIndex, heapSize: number): BoxedIndexWriter;
  function newSearch(index: BoxedIndex, schema: BoxedSchema, indexWriter: BoxedIndexWriter): BoxedSearch;
  function addDoc(search: BoxedSearch, doc: string): void;
  function commit(search: BoxedSearch): void;
  function newQueryParser(search: BoxedSearch, fields: number[]): BoxedQueryParser;
  function parseQuery(queryParser: BoxedQueryParser, query: string): BoxedQuery;
  function topDocs(limit: number): BoxedTopDocs;
  function topSearch(search: BoxedSearch, query: BoxedQuery, collector: BoxedTopDocs): SearchResult[];
}

export type TextOption = 'TEXT' | 'STORED' | 'STRING';
export type TextOptions = TextOption[];
export type Field = number;

const _BOXED_SCHEMA: unique symbol = Symbol();

export class Schema {
  [_BOXED_SCHEMA]: addon.BoxedSchema;

  constructor(boxedSchema: addon.BoxedSchema) {
    this[_BOXED_SCHEMA] = boxedSchema;
  }
}

export class SchemaBuilder {
  private _boxedSchemaBuilder: addon.BoxedSchemaBuilder;

  constructor() {
    this._boxedSchemaBuilder = addon.newSchemaBuilder();
  }

  addTextField(name: string, options: TextOptions): Field {
    return addon.addTextField(this._boxedSchemaBuilder, name, options);
  }

  build(): Schema {
    return new Schema(addon.buildSchema(this._boxedSchemaBuilder));
  }
}

const _BOXED_INDEX: unique symbol = Symbol();

export class Index {
  [_BOXED_INDEX]: addon.BoxedIndex;

  constructor(schema: Schema, path: string) {
    this[_BOXED_INDEX] = addon.createIndex(schema[_BOXED_SCHEMA], path);
  }
}

const _BOXED_INDEX_WRITER: unique symbol = Symbol();

export class IndexWriter {
  [_BOXED_INDEX_WRITER]: addon.BoxedIndexWriter;

  constructor(index: Index, heapSize: number) {
    this[_BOXED_INDEX_WRITER] = addon.createIndexWriter(index[_BOXED_INDEX], heapSize);
  }
}

export type SearchResult = addon.SearchResult;

const _BOXED_SEARCH: unique symbol = Symbol();
const _DEFAULT_TEXT_FIELDS: unique symbol = Symbol();

export class Search {
  [_BOXED_SEARCH]: addon.BoxedSearch;
  [_DEFAULT_TEXT_FIELDS]: number[];

  constructor(index: Index, schema: Schema, indexWriter: IndexWriter, defaultTextFields: number[]) {
    this[_BOXED_SEARCH] = addon.newSearch(index[_BOXED_INDEX], schema[_BOXED_SCHEMA], indexWriter[_BOXED_INDEX_WRITER]);
    this[_DEFAULT_TEXT_FIELDS] = defaultTextFields;
  }

  addDoc(doc: any) {
    addon.addDoc(this[_BOXED_SEARCH], JSON.stringify(doc));
  }

  commit() {
    addon.commit(this[_BOXED_SEARCH]);
  }

  topSearch(query: Query, collector: TopDocs): SearchResult[] {
    return addon.topSearch(this[_BOXED_SEARCH], query[_BOXED_QUERY], collector[_BOXED_TOP_DOCS]);
  }
}

export class QueryParser {
  private _boxedQueryParser: addon.BoxedQueryParser;

  constructor(search: Search, fields?: number[]) {
    this._boxedQueryParser = addon.newQueryParser(
      search[_BOXED_SEARCH],
      fields ?? search[_DEFAULT_TEXT_FIELDS]
    );
  }

  parse(query: string): Query {
    return new Query(addon.parseQuery(this._boxedQueryParser, query));
  }
}

const _BOXED_QUERY: unique symbol = Symbol();

export class Query {
  [_BOXED_QUERY]: addon.BoxedQuery;

  constructor(boxedQuery: addon.BoxedQuery) {
    this[_BOXED_QUERY] = boxedQuery;
  }
}

const _BOXED_TOP_DOCS: unique symbol = Symbol();

export class TopDocs {
  [_BOXED_TOP_DOCS]: addon.BoxedTopDocs;

  constructor(limit: number) {
    this[_BOXED_TOP_DOCS] = addon.topDocs(limit);
  }
}
