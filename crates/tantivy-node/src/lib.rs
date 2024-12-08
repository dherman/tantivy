use std::path::PathBuf;

use neon::{prelude::*, types::JsBigInt};
use neon::types::extract::Json;

use ordermap::OrderMap;
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{DocAddress, Document, IndexReader, ReloadPolicy, Score, Searcher};
use tantivy::{schema::{Field, Schema, TextOptions}, Index, IndexSettings, IndexWriter, TantivyDocument};

pub mod boxcell;

use boxcell::BoxCell;

#[derive(Serialize, Deserialize, Debug)]
enum TextOption {
    TEXT,
    STORED,
    STRING,
}

#[neon::export(name = "newSchema")]
fn new_schema<'cx>(
    cx: &mut FunctionContext<'cx>,
    // TODO: TextOptions is already serializable, can we use it directly?
    Json(descriptor): Json<OrderMap<String, Vec<TextOption>>>,
) -> JsResult<'cx, JsBox<BoxCell<Schema>>> {
    let mut builder = Schema::builder();
    for (field_name, options) in descriptor.iter() {
        builder.add_text_field(field_name, options.iter().fold(TextOptions::default(), |acc, option| {
            match option {
                TextOption::TEXT => acc | tantivy::schema::TEXT,
                TextOption::STORED => acc | tantivy::schema::STORED,
                TextOption::STRING => acc | tantivy::schema::STRING,
            }
        }));
    }
    Ok(cx.boxed(BoxCell::new(builder.build())))
}

#[neon::export(name = "commitSync")]
fn commit_sync<'cx>(
    // TODO: can I leave this out even though we need the lifetime for the handle?
    _cx: &mut FunctionContext<'cx>,
    index: Handle<'cx, JsBox<BoxCell<OpenIndex>>>,
) {
    let mut index = index.as_mut();
    index.writer.commit().unwrap();
}

#[neon::export(name = "newSearcher")]
fn new_searcher<'cx>(
    cx: &mut FunctionContext<'cx>,
    index: Handle<'cx, JsBox<BoxCell<OpenIndex>>>,
) -> JsResult<'cx, JsBox<BoxCell<Searcher>>> {
    Ok(cx.boxed(BoxCell::new(index.as_ref().reader.searcher())))
}

struct OpenIndex {
    index: Index,
    writer: IndexWriter,
    reader: IndexReader,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ReloadOnPolicy {
    CommitWithDelay,
    Manual,
}

impl From<ReloadOnPolicy> for ReloadPolicy {
    fn from(policy: ReloadOnPolicy) -> Self {
        match policy {
            ReloadOnPolicy::CommitWithDelay => ReloadPolicy::OnCommitWithDelay,
            ReloadOnPolicy::Manual => ReloadPolicy::Manual,
        }
    }
}

#[neon::export(name = "newIndex")]
fn new_index<'cx>(
    cx: &mut FunctionContext<'cx>,
    path: String,
    heap_size: f64,
    schema: Handle<'cx, JsBox<BoxCell<Schema>>>,
    Json(reload_on): Json<ReloadOnPolicy>,
) -> JsResult<'cx, JsBox<BoxCell<OpenIndex>>> {
    let dir_path = PathBuf::from(path);
    let dir = tantivy::directory::MmapDirectory::open(dir_path).unwrap();
    let index = Index::create(dir, schema.as_ref().clone(), IndexSettings::default()).unwrap();
    let reader = index
        .reader_builder()
        .reload_policy(reload_on.into())
        // TODO: can probably just use .into()
        .try_into()
        .unwrap();
    // TODO: replace `as` with safer cast
    let writer = index.writer(heap_size as usize).unwrap();
    Ok(cx.boxed(BoxCell::new(OpenIndex { index, writer, reader })))
}

#[neon::export(name = "reloadSync")]
fn reload_sync<'cx>(
    _cx: &mut FunctionContext<'cx>,
    index: Handle<'cx, JsBox<BoxCell<OpenIndex>>>,
) {
    let index = index.as_ref();
    index.reader.reload().unwrap();
}

#[neon::export(name = "addDocument")]
fn add_document<'cx>(
    cx: &mut FunctionContext<'cx>,
    index: Handle<'cx, JsBox<BoxCell<OpenIndex>>>,
    document: String,
) -> Handle<'cx, JsBigInt> {
    let index = index.as_ref();
    let td = TantivyDocument::parse_json(&index.index.schema(), &document).unwrap();
    let stamp = index.writer.add_document(td).unwrap();
    JsBigInt::from_u64(cx, stamp)
}

#[neon::export(name = "topDocsSync")]
fn top_docs_sync<'cx>(
    _cx: &mut FunctionContext<'cx>,
    searcher: Handle<'cx, JsBox<BoxCell<Searcher>>>,
    query_str: String,
    Json(fields): Json<Vec<u32>>,
    limit: f64,
) -> Json<Vec<(Score, String)>> {
    let searcher = searcher.as_ref();
    let fields = fields.iter().map(|id| Field::from_field_id(*id)).collect();
    let index = searcher.index();
    let schema = index.schema();
    let query_parser = QueryParser::for_index(index, fields);
    let query = query_parser.parse_query(&query_str).unwrap();
    let collector = TopDocs::with_limit(limit as usize);

    let top_docs: Vec<(Score, DocAddress)> = searcher.search(&query, &collector).unwrap();

    let mut results: Vec<(Score, String)> = vec![];

    for (score, doc_address) in top_docs  {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();

        // TODO: is a TantivyDocument already serializable?
        results.push((score, retrieved_doc.to_json(&schema)));
    }

    Json(results)
}

// TODO: async commit()
// TODO: async reload()
// TODO: async search()
