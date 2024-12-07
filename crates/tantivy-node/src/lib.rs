use std::path::PathBuf;

use neon::{prelude::*, types::JsBigInt};
use neon::types::extract::Json;

use tantivy::collector::TopDocs;
use tantivy::query::{Query, QueryParser};
use tantivy::{DocAddress, Document, Score};
use tantivy::{schema::{Field, Schema, SchemaBuilder, TextOptions}, Index, IndexSettings, IndexWriter, TantivyDocument};

pub mod boxcell;

use boxcell::BoxCell;

#[neon::export(name = "buildSchema")]
fn build_schema<'cx>(
    cx: &mut FunctionContext<'cx>,
    builder: Handle<'cx, JsBox<BoxCell<SchemaBuilder>>>,
) -> JsResult<'cx, JsBox<BoxCell<Schema>>> {
    Ok(cx.boxed(BoxCell::new(builder.take().build())))
}

#[neon::export(name = "newSchemaBuilder")]
fn new_schema_builder<'cx>(
    cx: &mut FunctionContext<'cx>,
) -> JsResult<'cx, JsBox<BoxCell<SchemaBuilder>>> {
    Ok(cx.boxed(BoxCell::new(Schema::builder())))
}

#[neon::export(name = "addTextField")]
fn add_text_field<'cx>(
    _cx: &mut FunctionContext<'cx>,
    builder: Handle<'cx, JsBox<BoxCell<SchemaBuilder>>>,
    field_name: String,
    options: Json<Vec<String>>,
) -> u32 {
    let options = options
        .0
        .into_iter()
        .fold(TextOptions::default(), |acc, option| {
            match option.as_str() {
                "TEXT" => acc | tantivy::schema::TEXT,
                "STORED" => acc | tantivy::schema::STORED,
                "STRING" => acc | tantivy::schema::STRING,
                _ => panic!("Unknown text field option: {}", option),
            }
        });

    let field = builder.as_mut().add_text_field(&field_name, options);
    field.field_id()
}

struct Search {
    index: Index,
    schema: Schema,
    index_writer: IndexWriter,
}


#[neon::export(name = "newSearch")]
fn new_search<'cx>(
    cx: &mut FunctionContext<'cx>,
    index: Handle<'cx, JsBox<BoxCell<Index>>>,
    schema: Handle<'cx, JsBox<BoxCell<Schema>>>,
    index_writer: Handle<'cx, JsBox<BoxCell<IndexWriter>>>,
) -> JsResult<'cx, JsBox<BoxCell<Search>>> {
    Ok(cx.boxed(BoxCell::new(Search {
        index: index.take(),
        schema: schema.take(),
        index_writer: index_writer.take(),
    })))
}

#[neon::export(name = "createIndex")]
fn create_index<'cx>(
    cx: &mut FunctionContext<'cx>,
    schema: Handle<'cx, JsBox<BoxCell<Schema>>>,
    path: String,
) -> JsResult<'cx, JsBox<BoxCell<Index>>> {
    let dir_path = PathBuf::from(path);
    let dir = tantivy::directory::MmapDirectory::open(dir_path).unwrap();
    let index = Index::create(dir, schema.as_ref().clone(), IndexSettings::default()).unwrap();
    Ok(cx.boxed(BoxCell::new(index)))
}

#[neon::export(name = "createIndexWriter")]
fn create_index_writer<'cx>(
    cx: &mut FunctionContext<'cx>,
    index: Handle<'cx, JsBox<BoxCell<Index>>>,
    heap_size: f64,
) -> JsResult<'cx, JsBox<BoxCell<IndexWriter>>> {
    // TODO: replace `as` with safer cast
    let writer = index.as_ref().writer(heap_size as usize).unwrap();
    Ok(cx.boxed(BoxCell::new(writer)))
}

#[neon::export(name = "addDoc")]
fn add_doc<'cx>(
    cx: &mut FunctionContext<'cx>,
    search: Handle<'cx, JsBox<BoxCell<Search>>>,
    doc: String,
) -> Handle<'cx, JsBigInt> {
    let search = search.as_ref();
    let td = TantivyDocument::parse_json(&search.schema, &doc).unwrap();
    let stamp = search.index_writer.add_document(td).unwrap();
    JsBigInt::from_u64(cx, stamp)
}

#[neon::export(name = "commit")]
fn commit<'cx>(
    // TODO: can I leave this out even though we need the lifetime for the handle?
    _cx: &mut FunctionContext<'cx>,
    search: Handle<'cx, JsBox<BoxCell<Search>>>,
) {
    let mut search = search.as_mut();
    search.index_writer.commit().unwrap();
}

#[neon::export(name = "newQueryParser")]
fn new_query_parser<'cx>(
    cx: &mut FunctionContext<'cx>,
    search: Handle<'cx, JsBox<BoxCell<Search>>>,
    Json(fields): Json<Vec<u32>>,
) -> JsResult<'cx, JsBox<BoxCell<QueryParser>>> {
    let search = search.as_ref();
    let fields = fields.iter().map(|id| Field::from_field_id(*id)).collect();
    Ok(cx.boxed(BoxCell::new(QueryParser::for_index(&search.index, fields))))
}

#[neon::export(name = "parseQuery")]
fn parse_query<'cx>(
    cx: &mut FunctionContext<'cx>,
    query_parser: Handle<'cx, JsBox<BoxCell<QueryParser>>>,
    query: String,
) -> JsResult<'cx, JsBox<BoxCell<Box<dyn Query>>>> {
    let query_parser = query_parser.as_ref();
    Ok(cx.boxed(BoxCell::new(query_parser.parse_query(&query).unwrap())))
}

#[neon::export(name = "topDocs")]
fn top_docs<'cx>(
    cx: &mut FunctionContext<'cx>,
    limit: f64,
) -> JsResult<'cx, JsBox<BoxCell<TopDocs>>> {
    // TODO: safer cast
    Ok(cx.boxed(BoxCell::new(TopDocs::with_limit(limit as usize))))
}

#[neon::export(name = "topSearch")]
fn top_search<'cx>(
    _cx: &mut FunctionContext<'cx>,
    search: Handle<'cx, JsBox<BoxCell<Search>>>,
    query: Handle<'cx, JsBox<BoxCell<Box<dyn Query>>>>,
    collector: Handle<'cx, JsBox<BoxCell<TopDocs>>>,
) -> Json<Vec<(Score, String)>> {
    let index = &search.as_ref().index;
    let schema = &search.as_ref().schema;
    let query = query.as_ref();
    let collector = collector.as_ref();
    let reader = index.reader().unwrap();
    let searcher = reader.searcher();
    let top_docs: Vec<(Score, DocAddress)> = searcher.search(&*query, &*collector).unwrap();

    let mut results: Vec<(Score, String)> = vec![];

    for (score, doc_address) in top_docs  {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();

        // TODO: is a TantivyDocument already serializable?
        results.push((score, retrieved_doc.to_json(schema)));
    }

    Json(results)
}
