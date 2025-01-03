use std::path::PathBuf;
use std::sync::Mutex;

use neon::{prelude::*, types::JsBigInt};
use neon::types::extract::{Boxed, Error, Json};

use num::{u53, Project};
use ordermap::OrderMap;
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::{Explanation, FuzzyTermQuery, PhrasePrefixQuery, Query, QueryParser, RegexQuery};
use tantivy::schema::{NumericOptions, SchemaBuilder};
use tantivy::tokenizer::{SimpleTokenizer, TokenStream, Tokenizer, WhitespaceTokenizer};
use tantivy::{Document, IndexReader, ReloadPolicy, Score, Searcher, Term};
use tantivy::{schema::{Field, Schema, TextOptions}, Index, IndexSettings, IndexWriter, TantivyDocument};

pub mod boxcell;
pub mod boxarc;
pub mod num;

use boxcell::BoxCell;
use boxarc::BoxArc;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum FieldDescriptor {
    Text { flags: Option<Vec<TextOption>> },
    String { flags: Option<Vec<TextOption>> },
    F64 { flags: Option<Vec<NumericOption>> },
    // TODO: | F64FieldDescriptor
    // TODO: | I64FieldDescriptor
    // TODO: | U64FieldDescriptor
    // TODO: | DateFieldDescriptor
    // TODO: | BoolFieldDescriptor
    // TODO: | IpAddrFieldDescriptor
}

#[derive(Serialize, Deserialize, Debug)]
enum TextOption {
    STORED,
}

#[derive(Serialize, Deserialize, Debug)]
enum NumericOption {
    STORED,
    INDEXED,
}

fn add_field(builder: &mut SchemaBuilder, name: &str, options: &FieldDescriptor) {
    match options {
        FieldDescriptor::Text { flags } => {
            let mut options = TextOptions::default() | tantivy::schema::TEXT;
            if let Some(flags) = flags {
                for flag in flags {
                    options = match flag {
                        TextOption::STORED => options | tantivy::schema::STORED,
                    };
                }
            }
            builder.add_text_field(name, options);
        }
        FieldDescriptor::String { flags } => {
            let mut options = TextOptions::default() | tantivy::schema::STRING;
            if let Some(flags) = flags {
                for flag in flags {
                    options = match flag {
                        TextOption::STORED => options | tantivy::schema::STORED,
                    };
                }
            }
            builder.add_text_field(name, options);
        }
        FieldDescriptor::F64 { flags } => {
            let mut options = NumericOptions::default();
            if let Some(flags) = flags {
                for flag in flags {
                    options = match flag {
                        NumericOption::STORED => options | tantivy::schema::STORED,
                        NumericOption::INDEXED => options | tantivy::schema::INDEXED,
                    };
                }
            }
            builder.add_f64_field(name, options);
        }
    }
}

#[neon::export]
fn new_schema(
    Json(descriptor): Json<OrderMap<String, FieldDescriptor>>,
) -> Boxed<BoxCell<Schema>> {
    let mut builder = Schema::builder();
    for (field_name, options) in descriptor.iter() {
        add_field(&mut builder, field_name, options);
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
fn new_regex_query(pattern: String, field: f64) -> Result<Boxed<BoxArc<Box<dyn Query>>>, Error> {
    let field = Field::from_field_id(field as u32);
    let query = RegexQuery::from_pattern(&pattern, field)?;
    Ok(Boxed(BoxArc::new(Box::new(query))))
}

#[neon::export]
fn new_phrase_prefix_query(
    Json(terms): Json<Vec<String>>,
    field: f64,
) -> Result<Boxed<BoxArc<Box<dyn Query>>>, Error> {
    let field = Field::from_field_id(field as u32);
    let terms = terms.into_iter().map(|term| {
        Term::from_field_text(field, &term)
    }).collect();
    let query = PhrasePrefixQuery::new(terms);
    Ok(Boxed(BoxArc::new(Box::new(query))))
}

#[neon::export]
fn new_fuzzy_term_query(
    term: String,
    field: f64,
    max_distance: f64,
    transposition_costs_one: bool,
    is_prefix: bool,
) -> Result<Boxed<BoxArc<Box<dyn Query>>>, Error> {
    let field = Field::from_field_id(field as u32);
    let term = Term::from_field_text(field, &term);
    let query = if is_prefix {
        FuzzyTermQuery::new_prefix(term, max_distance as u8, transposition_costs_one)
    } else {
        FuzzyTermQuery::new(term, max_distance as u8, transposition_costs_one)
    };
    Ok(Boxed(BoxArc::new(Box::new(query))))
}

#[neon::export]
fn parse_query(
    Boxed(searcher): Boxed<BoxArc<Searcher>>,
    source: String,
    Json(fields): Json<Vec<f64>>,
) -> Result<Boxed<BoxArc<Box<dyn Query>>>, Error> {
    let index = searcher.index();
    let fields = fields.iter().map(|id| Field::from_field_id(*id as u32)).collect();
    let query_parser: QueryParser = QueryParser::for_index(index, fields);
    Ok(Boxed(BoxArc::new(query_parser.parse_query(&source)?)))
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
    Boxed(query): Boxed<BoxArc<Box<dyn Query>>>,
    limit: f64,
) -> Json<Vec<(Score, String, Explanation)>>{
    let index = searcher.index();
    let schema = index.schema();
    let collector = TopDocs::with_limit(limit as usize);
    Json(
        searcher
            .search(&*query, &collector)
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
    query: Boxed<BoxArc<Box<dyn Query>>>,
    limit: f64,
) -> Json<Vec<(Score, String, Explanation)>>{
    top_docs(searcher, query, limit)
}

#[neon::export]
fn simple_tokenize(input: String) -> Json<Vec<String>> {
    let mut tokenizer = SimpleTokenizer::default();
    let mut stream = tokenizer.token_stream(&input);
    let mut result = vec![];
    stream.process(&mut |token| {
        result.push(token.text.clone());
    });
    Json(result)
}

#[neon::export]
fn whitespace_tokenize(input: String) -> Json<Vec<String>> {
    let mut tokenizer = WhitespaceTokenizer::default();
    let mut stream = tokenizer.token_stream(&input);
    let mut result = vec![];
    stream.process(&mut |token| {
        result.push(token.text.clone());
    });
    Json(result)
}
