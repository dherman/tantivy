// This module is the CJS entry point for the library.

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
    schema(): Schema;
    addDocument(doc: any): BigInt;
    commit(): Promise<void>;
    commitSync(): void;
    reload(): Promise<void>;
    reloadSync(): void;
    searcher(): Searcher;
    registerTokenizer(name: string, tokenizer: TextAnalyzer): void;
  }
}

export type Token = {
  byteOffsetFrom: number,
  byteOffsetTo: number,
  charOffsetFrom: number,
  charOffsetTo: number,
  position: number,
  text: string,
  positionLength: number,
}

export enum Language {
  Arabic = "Arabic",
  Danish = "Danish",
  Dutch = "Dutch",
  English = "English",
  Finnish = "Finnish",
  French = "French",
  German = "German",
  Greek = "Greek",
  Hungarian = "Hungarian",
  Italian = "Italian",
  Norwegian = "Norwegian",
  Portuguese = "Portuguese",
  Romanian = "Romanian",
  Russian = "Russian",
  Spanish = "Spanish",
  Swedish = "Swedish",
  Tamil = "Tamil",
  Turkish = "Turkish",
}

export type TextAnalyzerOptions = {
  removeLong?: number | null,
  alphaNumOnly?: boolean,
  asciiFolding?: boolean,
  lowerCase?: boolean,
  // splitCompoundWords: Dictionary,
  stemmer?: Language | null,
  filterStopWords?: Language | null,
};

export enum IndexRecordOption {
  Basic = "BASIC",
  WithFreqs = "WITH_FREQS",
  WithFreqsAndPositions = "WITH_FREQS_AND_POSITIONS",
}

export type TextFieldDescriptor = {
  type: "text",
  flags?: TextOption[],
  index?: IndexRecordOption,
  tokenizer?: string,
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

export type ReloadPolicy = 'COMMIT_WITH_DELAY' | 'MANUAL';

export type IndexOptions = {
  heapSize?: number,
  reloadOn?: ReloadPolicy,
}

export type SearchOptions = {
  top?: number
}

export type FuzzyTermQueryOptions = {
  maxDistance?: number,
  transpositionCostsOne?: boolean,
  isPrefix?: boolean,
};

export type SearchResult = [number, string];

export { Index, Searcher, Query, Schema, TextAnalyzer } from "./load.cjs";
