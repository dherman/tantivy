use std::{cell::RefCell, path::PathBuf};

use neon::{prelude::*, types::JsBigInt};
use neon::types::extract::Json;

use tantivy::collector::TopDocs;
use tantivy::query::{Query, QueryParser};
use tantivy::{DocAddress, Document, Score};
use tantivy::{schema::{Field, Schema, SchemaBuilder, TextOptions}, Index, IndexSettings, IndexWriter, TantivyDocument};

struct AutoFinal<T>(pub T);

impl<T> Finalize for AutoFinal<T> {
    fn finalize<'cx, C: Context<'cx>>(self, _: &mut C) {}
}

#[neon::export(name = "buildSchema")]
fn build_schema<'cx>(
    cx: &mut FunctionContext<'cx>,
    builder: Handle<'cx, JsBox<RefCell<AutoFinal<Option<SchemaBuilder>>>>>,
) -> JsResult<'cx, JsBox<RefCell<AutoFinal<Option<Schema>>>>> {
    Ok(cx.boxed(RefCell::new(AutoFinal(Some(builder.borrow_mut().0.take().unwrap().build())))))
}

#[neon::export(name = "newSchemaBuilder")]
fn new_schema_builder<'cx>(
    cx: &mut FunctionContext<'cx>,
) -> JsResult<'cx, JsBox<RefCell<AutoFinal<Option<SchemaBuilder>>>>> {
    Ok(cx.boxed(RefCell::new(AutoFinal(Some(Schema::builder())))))
}

#[neon::export(name = "addTextField")]
fn add_text_field<'cx>(
    _cx: &mut FunctionContext<'cx>,
    builder: Handle<'cx, JsBox<RefCell<AutoFinal<Option<SchemaBuilder>>>>>,
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

    let mut builder = builder.borrow_mut();

    let field = builder.0.as_mut().unwrap().add_text_field(&field_name, options);
    field.field_id()
}

struct Search {
    index: Index,
    schema: Schema,
    index_writer: IndexWriter,
}

// TODO: abstract JsBox<RefCell<AutoFinal<Option<T>>>> as JsCell and represent the failures as JsResult

#[neon::export(name = "newSearch")]
fn new_search<'cx>(
    cx: &mut FunctionContext<'cx>,
    index: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Index>>>>>,
    schema: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Schema>>>>>,
    index_writer: Handle<'cx, JsBox<RefCell<AutoFinal<Option<IndexWriter>>>>>,
) -> JsResult<'cx, JsBox<RefCell<AutoFinal<Option<Search>>>>> {
    Ok(cx.boxed(RefCell::new(AutoFinal(Some(Search {
        index: index.borrow_mut().0.take().unwrap(),
        schema: schema.borrow_mut().0.take().unwrap(),
        index_writer: index_writer.borrow_mut().0.take().unwrap(),
    })))))
}

#[neon::export(name = "createIndex")]
fn create_index<'cx>(
    cx: &mut FunctionContext<'cx>,
    schema: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Schema>>>>>,
    path: String,
) -> JsResult<'cx, JsBox<RefCell<AutoFinal<Option<Index>>>>> {
    let schema = schema.borrow();
    let dir_path = PathBuf::from(path);
    let dir = tantivy::directory::MmapDirectory::open(dir_path).unwrap();
    let index = Index::create(dir, schema.0.as_ref().unwrap().clone(), IndexSettings::default()).unwrap();
    Ok(cx.boxed(RefCell::new(AutoFinal(Some(index)))))
}

#[neon::export(name = "createIndexWriter")]
fn create_index_writer<'cx>(
    cx: &mut FunctionContext<'cx>,
    index: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Index>>>>>,
    heap_size: f64,
) -> JsResult<'cx, JsBox<RefCell<AutoFinal<Option<IndexWriter>>>>> {
    let index = index.borrow();
    // TODO: replace `as` with safer cast
    let writer = index.0.as_ref().unwrap().writer(heap_size as usize).unwrap();
    Ok(cx.boxed(RefCell::new(AutoFinal(Some(writer)))))
}

#[neon::export(name = "addDoc")]
fn add_doc<'cx>(
    cx: &mut FunctionContext<'cx>,
    search: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Search>>>>>,
    doc: String,
) -> Handle<'cx, JsBigInt> {
    let search = search.borrow();
    let search = search.0.as_ref().unwrap();
    let td = TantivyDocument::parse_json(&search.schema, &doc).unwrap();
    let stamp = search.index_writer.add_document(td).unwrap();
    JsBigInt::from_u64(cx, stamp)
}

/*
#[neon::export(name = "addDoc")]
fn add_doc<'cx>(
    cx: &mut FunctionContext<'cx>,
    search: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Search>>>>>,
    doc: Json<Map<String, Value>>,
) -> Handle<'cx, JsBigInt> {
    let search = search.borrow();
    let search = search.0.as_ref().unwrap();
    let mut td = TantivyDocument::default();

    for (key, value) in doc.0 {
        let field = search.schema.get_field(&key).unwrap();
        // TODO: handle non-strings
        let value = value.as_str().unwrap();
        td.add_field_value(field, value);
    }

    let stamp = search.index_writer.add_document(td).unwrap();
    JsBigInt::from_u64(cx, stamp)
}
*/

#[neon::export(name = "commit")]
fn commit<'cx>(
    // TODO: can I leave this out even though we need the lifetime for the handle?
    _cx: &mut FunctionContext<'cx>,
    search: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Search>>>>>,
) {
    let mut search = search.borrow_mut();
    let search = search.0.as_mut().unwrap();
    search.index_writer.commit().unwrap();
}

#[neon::export(name = "newQueryParser")]
fn new_query_parser<'cx>(
    cx: &mut FunctionContext<'cx>,
    search: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Search>>>>>,
    Json(fields): Json<Vec<u32>>,
) -> JsResult<'cx, JsBox<RefCell<AutoFinal<Option<QueryParser>>>>> {
    let search = search.borrow();
    let search = search.0.as_ref().unwrap();
    let fields = fields.iter().map(|id| Field::from_field_id(*id)).collect();
    Ok(cx.boxed(RefCell::new(AutoFinal(Some(QueryParser::for_index(&search.index, fields))))))
}

#[neon::export(name = "parseQuery")]
fn parse_query<'cx>(
    cx: &mut FunctionContext<'cx>,
    query_parser: Handle<'cx, JsBox<RefCell<AutoFinal<Option<QueryParser>>>>>,
    query: String,
) -> JsResult<'cx, JsBox<RefCell<AutoFinal<Option<Box<dyn Query>>>>>> {
    let query_parser = query_parser.borrow();
    let query_parser = query_parser.0.as_ref().unwrap();
    Ok(cx.boxed(RefCell::new(AutoFinal(Some(query_parser.parse_query(&query).unwrap())))))
}

#[neon::export(name = "topDocs")]
fn top_docs<'cx>(
    cx: &mut FunctionContext<'cx>,
    limit: f64,
) -> JsResult<'cx, JsBox<RefCell<AutoFinal<Option<TopDocs>>>>> {
    // TODO: safer cast
    Ok(cx.boxed(RefCell::new(AutoFinal(Some(TopDocs::with_limit(limit as usize))))))
}

#[neon::export(name = "topSearch")]
fn top_search<'cx>(
    _cx: &mut FunctionContext<'cx>,
    search: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Search>>>>>,
    query: Handle<'cx, JsBox<RefCell<AutoFinal<Option<Box<dyn Query>>>>>>,
    collector: Handle<'cx, JsBox<RefCell<AutoFinal<Option<TopDocs>>>>>,
) -> Json<Vec<(Score, String)>> {
    let index = search.borrow();
    let index = &index.0.as_ref().unwrap().index;
    let schema = search.borrow();
    let schema = &schema.0.as_ref().unwrap().schema;
    let query = query.borrow();
    let query = query.0.as_ref().unwrap();
    let collector = collector.borrow();
    let collector = collector.0.as_ref().unwrap();
    let reader = index.reader().unwrap();
    let searcher = reader.searcher();
    let top_docs: Vec<(Score, DocAddress)> = searcher.search(query, collector).unwrap();

    let mut results: Vec<(Score, String)> = vec![];

    for (score, doc_address) in top_docs  {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();

        // TODO: is a TantivyDocument already serializable?
        results.push((score, retrieved_doc.to_json(schema)));
    }

    Json(results)
}
