// This module is the CJS entry point for the library.

import type {
  FieldDescriptor,
  FuzzyTermQueryOptions,
  IndexOptions,
  IndexRecordOption,
  SearchOptions,
  TextAnalyzerOptions,
  Token,
} from "./generated.cjs";

// Re-export all types generated from the Rust definitions.
export type {
  FieldDescriptor,
  FuzzyTermQueryOptions,
  IndexOptions,
  IndexRecordOption,
  Language,
  NumericOption,
  ReloadPolicy,
  SearchOptions,
  TextAnalyzerOptions,
  TextOption,
  Token,
} from "./generated.cjs";

// Use this declaration to assign types to the addon's exports,
// which otherwise by default are `any`.
declare module "./load.cjs" {
  export interface Query {}

  export class Schema {
    constructor(fields: SchemaDescriptor);
    fields(): SchemaDescriptor;
  }

  export interface Searcher {
    termQuery(term: string, field: string, options?: IndexRecordOption): Query;
    phraseQuery(terms: string[], field: string): Query;
    fuzzyTermQuery(term: string, field: string, options?: FuzzyTermQueryOptions): Query;
    regexpQuery(pattern: string, field: string): Query;
    phrasePrefixQuery(terms: string[], field: string): Query;
    searchSync(query: Query, options: SearchOptions): SearchResult[];
    search(query: Query, options: SearchOptions): Promise<SearchResult[]>;
    searchTerms(field: string, pattern: string): string[];
  }

  export class TextAnalyzer {
    constructor(options?: TextAnalyzerOptions);
    tokenize(text: string): Token[];
  }

  export class Index {
    constructor(path: string, schema: Schema, options?: IndexOptions);
    addDocument(doc: any): BigInt;
    commit(): Promise<void>;
    commitSync(): void;
    reload(): Promise<void>;
    reloadSync(): void;
    searcher(): Searcher;
    registerTokenizer(name: string, tokenizer: TextAnalyzer): void;
  }
}

// Type aliases that aren't auto-generated.
export type Field = number;
export type SchemaDescriptor = Record<string, FieldDescriptor>;
export type SearchResult = [number, string];

export { Index, Searcher, Query, Schema, TextAnalyzer } from "./load.cjs";
