// This module is the CJS entry point for the library.

import type { FieldDescriptor } from "./load.cjs";

// Re-export all types generated from the Rust definitions.
export type * from "./load.cjs";

// Type aliases that aren't auto-generated.
export type Field = number;
export type SchemaDescriptor = Record<string, FieldDescriptor>;
export type SearchResult = [number, string];

export { Index, Searcher, Query, Schema, TextAnalyzer } from "./load.cjs";
