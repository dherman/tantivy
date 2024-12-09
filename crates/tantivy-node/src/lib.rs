use std::path::PathBuf;
use std::sync::Mutex;

use neon::{prelude::*, types::JsBigInt};
use neon::types::extract::{Boxed, Error, Json};

use num::{u53, Project};
use ordermap::OrderMap;
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::{Explanation, QueryParser};
use tantivy::{Document, IndexReader, ReloadPolicy, Score, Searcher};
use tantivy::{schema::{Field, Schema, TextOptions}, Index, IndexSettings, IndexWriter, TantivyDocument};

pub mod boxcell;
pub mod boxarc;
pub mod num;

use boxcell::BoxCell;
use boxarc::BoxArc;

#[derive(Serialize, Deserialize, Debug)]
enum TextOption {
    TEXT,
    STORED,
    STRING,
}

#[neon::export]
fn new_schema(
    Json(descriptor): Json<OrderMap<String, Vec<TextOption>>>,
) -> Boxed<BoxCell<Schema>> {
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
    Boxed(BoxCell::new(builder.build()))
}

#[neon::export(task)]
fn commit(
    Boxed(index): Boxed<BoxArc<OpenIndex>>,
) -> Result<(), Error> {
    index.writer.lock().map_err(|_| "mutex poisoned")?.commit()?;
    Ok(())
}

#[neon::export]
fn commit_sync(
    index: Boxed<BoxArc<OpenIndex>>,
) -> Result<(), Error> {
    commit(index)
}

#[neon::export]
fn new_searcher(
    Boxed(index): Boxed<BoxArc<OpenIndex>>,
) -> Result<Boxed<BoxArc<Searcher>>, Error> {
    Ok(Boxed(BoxArc::new(index.reader.lock().map_err(|_| "mutex poisoned")?.searcher())))
}

struct OpenIndex {
    index: Index,
    writer: Mutex<IndexWriter>,
    reader: Mutex<IndexReader>,
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

#[neon::export]
fn new_index(
    path: String,
    heap_size: f64,
    Boxed(schema): Boxed<BoxCell<Schema>>,
    Json(reload_on): Json<ReloadOnPolicy>,
) -> Result<Boxed<BoxArc<OpenIndex>>, Error> {
    let dir_path = PathBuf::from(path);
    let dir = tantivy::directory::MmapDirectory::open(dir_path)?;
    let index = Index::create(dir, schema.as_ref().clone(), IndexSettings::default())?;
    let reader = Mutex::new(
        index
            .reader_builder()
            .reload_policy(reload_on.into())
            .try_into()?
    );
    let heap_size: u53 = heap_size.project()?;
    let heap_size: u64 = heap_size.into();
    let heap_size: usize = heap_size.try_into()?;
    let writer = Mutex::new(index.writer(heap_size)?);
    Ok(Boxed(BoxArc::new(OpenIndex { index, writer, reader })))
}

#[neon::export(task)]
fn reload(
    Boxed(index): Boxed<BoxArc<OpenIndex>>,
) -> Result<(), Error> {
    index.reader.lock().map_err(|_| "mutex poisoned")?.reload()?;
    Ok(())
}

#[neon::export]
fn reload_sync(
    index: Boxed<BoxArc<OpenIndex>>
) -> Result<(), Error> {
    reload(index)
}

#[neon::export]
fn add_document<'cx>(
    cx: &mut FunctionContext<'cx>,
    Boxed(index): Boxed<BoxArc<OpenIndex>>,
    document: String,
) -> JsResult<'cx, JsBigInt> {
    let td = TantivyDocument::parse_json(&index.index.schema(), &document)
        .or_else(|err| cx.throw_error(err.to_string()))?;
    let stamp = index.writer
        .lock()
        .or_else(|_| cx.throw_error("mutex poisoned"))?
        .add_document(td)
        .or_else(|err| cx.throw_error(err.to_string()))?;
    Ok(JsBigInt::from_u64(cx, stamp))
}

#[neon::export(task)]
fn top_docs(
    Boxed(searcher): Boxed<BoxArc<Searcher>>,
    query_str: String,
    Json(fields): Json<Vec<u32>>,
    limit: f64,
) -> Json<Vec<(Score, String, Explanation)>>{
    let fields = fields.iter().map(|id| Field::from_field_id(*id)).collect();
    let index = searcher.index();
    let schema = index.schema();
    let query_parser = QueryParser::for_index(index, fields);
    let query = query_parser.parse_query(&query_str).unwrap();
    let collector = TopDocs::with_limit(limit as usize);
    Json(
        searcher
            .search(&query, &collector)
            .unwrap()
            .iter()
            .map(|&(score, doc_address)| {
                let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
                (score, retrieved_doc.to_json(&schema), query.explain(&searcher, doc_address).unwrap())
            })
            .collect::<Vec<_>>()
    )
}

#[neon::export]
fn top_docs_sync<'cx>(
    searcher: Boxed<BoxArc<Searcher>>,
    query_str: String,
    fields: Json<Vec<u32>>,
    limit: f64,
) -> Json<Vec<(Score, String, Explanation)>>{
    top_docs(searcher, query_str, fields, limit)
}
