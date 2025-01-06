use std::path::PathBuf;
use std::str::CharIndices;
use std::sync::Mutex;

use neon::{prelude::*, types::JsBigInt};
use neon::types::extract::{Boxed, Error, Json};

use num::{u53, Project};
use ordermap::OrderMap;
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::{Explanation, FuzzyTermQuery, PhrasePrefixQuery, PhraseQuery, Query, QueryParser, RegexQuery, TermQuery};
use tantivy::schema::{NumericOptions, SchemaBuilder, TextFieldIndexing};
use tantivy::tokenizer::{AlphaNumOnlyFilter, AsciiFoldingFilter, Language, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer, StopWordFilter, TextAnalyzer, TextAnalyzerBuilder, TokenStream, Tokenizer, WhitespaceTokenizer};
use tantivy::{Document, IndexReader, ReloadPolicy, Score, Searcher, Term};
use tantivy::{schema::{Field, Schema, TextOptions}, Index, IndexSettings, IndexWriter, TantivyDocument};

pub mod boxcell;
pub mod boxarc;
pub mod boxmutex;
pub mod num;

use boxcell::BoxCell;
use boxarc::BoxArc;
use boxmutex::BoxMutex;
use tantivy_fst::Regex;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TextAnalyzerFilters {
    remove_long: Option<f64>,
    alpha_num_only: bool,
    ascii_folding: bool,
    lower_case: bool,
    // TODO: split_compound_words
    stemmer: Option<LanguageName>,
    filter_stop_words: Option<LanguageName>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Token {
    // TODO: how should we deal with larger than 32 bits?
    byte_offset_from: u32,
    byte_offset_to: u32,
    char_offset_from: u32,
    char_offset_to: u32,
    position: u32,
    text: String,
    position_length: u32,
}

impl Token {
    fn new(token: &tantivy::tokenizer::Token, char_offset_from: usize, char_offset_to: usize) -> Self {
        Self {
            byte_offset_from: token.offset_from as u32,
            byte_offset_to: token.offset_to as u32,
            char_offset_from: char_offset_from as u32,
            char_offset_to: char_offset_to as u32,
            position: token.position as u32,
            text: token.text.clone(),
            position_length: token.position_length as u32,
        }
    }
}

impl TextAnalyzerFilters {
    fn apply<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> TextAnalyzer {
        // Step through the filters one at a time. This tail recursive style
        // allows each method to take a generic base type since the specific
        // base type depends on which filters actually get applied, but the
        // result type always ends up as TextAnalyzer.
        self.apply_remove_long(builder)
    }

    fn apply_remove_long<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> TextAnalyzer {
        match self.remove_long {
            // TODO: better cast
            Some(n) => self.apply_alpha_num_only(builder.filter(RemoveLongFilter::limit(n as usize))),
            None => self.apply_alpha_num_only(builder),
        }
    }

    fn apply_alpha_num_only<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> TextAnalyzer {
        match self.alpha_num_only {
            true => self.apply_ascii_folding(builder.filter(AlphaNumOnlyFilter)),
            false => self.apply_ascii_folding(builder),
        }
    }

    fn apply_ascii_folding<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> TextAnalyzer {
        match self.ascii_folding {
            true => self.apply_lower_caser(builder.filter(AsciiFoldingFilter)),
            false => self.apply_lower_caser(builder),
        }
    }

    fn apply_lower_caser<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> TextAnalyzer {
        match self.lower_case {
            true => self.apply_stemmer(builder.filter(LowerCaser)),
            false => self.apply_stemmer(builder),
        }
    }

    fn apply_stemmer<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> TextAnalyzer {
        match self.stemmer {
            Some(language) => self.apply_stop_words(builder.filter(Stemmer::new(language.into()))),
            None => self.apply_stop_words(builder),
        }
    }

    fn apply_stop_words<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> TextAnalyzer {
        match self.filter_stop_words {
            Some(language) => builder.filter(StopWordFilter::new(language.into()).unwrap()).build(),
            None => builder.build(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum IndexRecordOption {
    Basic,
    WithFreqs,
    WithFreqsAndPositions,
}

impl From<IndexRecordOption> for tantivy::schema::IndexRecordOption {
    fn from(value: IndexRecordOption) -> Self {
        match value {
            IndexRecordOption::Basic => tantivy::schema::IndexRecordOption::Basic,
            IndexRecordOption::WithFreqs => tantivy::schema::IndexRecordOption::WithFreqs,
            IndexRecordOption::WithFreqsAndPositions => {
                tantivy::schema::IndexRecordOption::WithFreqsAndPositions
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
enum LanguageName {
    Arabic,
    Danish,
    Dutch,
    English,
    Finnish,
    French,
    German,
    Greek,
    Hungarian,
    Italian,
    Norwegian,
    Portuguese,
    Romanian,
    Russian,
    Spanish,
    Swedish,
    Tamil,
    Turkish,
}

impl From<LanguageName> for Language {
    fn from(value: LanguageName) -> Self {
        match value {
            LanguageName::Arabic => Language::Arabic,
            LanguageName::Danish => Language::Danish,
            LanguageName::Dutch => Language::Dutch,
            LanguageName::English => Language::English,
            LanguageName::Finnish => Language::Finnish,
            LanguageName::French => Language::French,
            LanguageName::German => Language::German,
            LanguageName::Greek => Language::Greek,
            LanguageName::Hungarian => Language::Hungarian,
            LanguageName::Italian => Language::Italian,
            LanguageName::Norwegian => Language::Norwegian,
            LanguageName::Portuguese => Language::Portuguese,
            LanguageName::Romanian => Language::Romanian,
            LanguageName::Russian => Language::Russian,
            LanguageName::Spanish => Language::Spanish,
            LanguageName::Swedish => Language::Swedish,
            LanguageName::Tamil => Language::Tamil,
            LanguageName::Turkish => Language::Turkish,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum FieldDescriptor {
    Text {
        flags: Option<Vec<TextOption>>,
        tokenizer: Option<String>,
        index: Option<IndexRecordOption>,
    },
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
        FieldDescriptor::Text { flags, tokenizer, index } => {
            let mut options = TextOptions::default() | tantivy::schema::TEXT;
            if let Some(flags) = flags {
                for flag in flags {
                    options = match flag {
                        TextOption::STORED => options | tantivy::schema::STORED,
                    };
                }
            }
            if let Some(tokenizer) = tokenizer {
                let index_option = match index {
                    Some(index) => (*index).into(),
                    None => tantivy::schema::IndexRecordOption::Basic,
                };
                let text_field_indexing = TextFieldIndexing::default()
                    .set_tokenizer(&tokenizer)
                    .set_index_option(index_option);
                options = options.set_indexing_options(text_field_indexing);
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
fn register_tokenizer(
    Boxed(index): Boxed<BoxArc<OpenIndex>>,
    name: String,
    Boxed(tokenizer): Boxed<BoxMutex<TextAnalyzer>>,
) {
    let manager = index.index.tokenizers();
    manager.register(&name, tokenizer.lock().unwrap().clone());
}

#[neon::export]
fn new_text_analyzer(
    Json(filters): Json<TextAnalyzerFilters>,
) -> Result<Boxed<BoxMutex<TextAnalyzer>>, Error> {
    // TODO: need a way to build off something other than a simple tokenizer
    let builder = TextAnalyzer::builder(SimpleTokenizer::default());
    let analyzer = filters.apply(builder);
    Ok(Boxed(BoxMutex::new(analyzer)))
}

fn count_chars_until_offset(i: &mut CharIndices, byte_offset: usize) -> usize {
    let mut chars = 0;
    while i.offset() < byte_offset {
        if i.next().is_none() {
            break;
        }
        chars += 1;
    }
    chars
}

#[neon::export]
fn text_analyzer_tokenize(
    Boxed(analyzer): Boxed<BoxMutex<TextAnalyzer>>,
    text: String,
) -> Json<Vec<Token>> {
    let mut analyzer = analyzer.lock().unwrap();
    let mut stream = analyzer.token_stream(&text);
    let mut result = vec![];
    let mut char_indices: CharIndices = text.char_indices();
    let mut char_offset = 0;
    stream.process(&mut |token| {
        char_offset += count_chars_until_offset(&mut char_indices, token.offset_from);
        let char_offset_from = char_offset;
        char_offset += count_chars_until_offset(&mut char_indices, token.offset_to);
        let char_offset_to = char_offset;
        result.push(Token::new(token, char_offset_from, char_offset_to));
    });
    Json(result)
}

#[neon::export]
fn new_term_query(
    term: String,
    field: f64,
    Json(options): Json<IndexRecordOption>,
) -> Result<Boxed<BoxArc<Box<dyn Query>>>, Error> {
    let field = Field::from_field_id(field as u32);
    let term = Term::from_field_text(field, &term);
    let query = TermQuery::new(term, options.into());
    Ok(Boxed(BoxArc::new(Box::new(query))))
}

#[neon::export]
fn new_phrase_query(
    Json(terms): Json<Vec<String>>,
    field: f64,
) -> Result<Boxed<BoxArc<Box<dyn Query>>>, Error> {
    let field = Field::from_field_id(field as u32);
    let terms = terms.into_iter().map(|term| {
        Term::from_field_text(field, &term)
    }).collect();
    let query = PhraseQuery::new(terms);
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

// TODO: expose TermDictionary as a first-class type
#[neon::export]
fn search_terms(
    Boxed(searcher): Boxed<BoxArc<Searcher>>,
    field: f64,
    pattern: String,
) -> Json<Vec<String>>
{
    let readers = searcher.segment_readers();
    let field = Field::from_field_id(field as u32);
    let mut result = vec![];
    for reader in readers {
        let inverted_index = reader.inverted_index(field).unwrap();
        let dict = inverted_index.terms();
        let mut stream = dict.search(Regex::new(&pattern).unwrap()).into_stream().unwrap();
        while let Some((term, _)) = stream.next() {
            let term = String::from_utf8_lossy(term);
            result.push(term.to_string());
        }
    }
    Json(result)
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
