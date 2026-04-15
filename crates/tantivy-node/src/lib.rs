use std::cell::RefCell;
use std::path::PathBuf;
use std::str::CharIndices;
use std::sync::{Arc, Mutex};

use neon::{prelude::*, types::JsBigInt};
use neon::types::extract::{Error, Json};

use num::{u53, Project};
use ordermap::OrderMap;
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::{Explanation, FuzzyTermQuery, PhrasePrefixQuery, PhraseQuery, RegexQuery, TermQuery};
use tantivy::schema::{NumericOptions, SchemaBuilder, TextFieldIndexing};
use tantivy::tokenizer::{AlphaNumOnlyFilter, AsciiFoldingFilter, Language, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer, StopWordFilter, TextAnalyzerBuilder, TokenStream, Tokenizer};
use tantivy::{Document, IndexReader, ReloadPolicy, Score, Term};
use tantivy::{schema::{Field, TextOptions}, IndexSettings, IndexWriter, TantivyDocument};

pub mod num;

use tantivy_fst::Regex;

// Explicitly-qualified Tantivy types to distinguish from our JS wrapper types of the same names.
mod t {
    pub use tantivy::schema::Schema;
    pub use tantivy::query::Query;
    pub use tantivy::{Index, Searcher};
    pub use tantivy::tokenizer::TextAnalyzer;
}

#[derive(Serialize, Deserialize, Debug, neon::TypeScript)]
#[serde(default, rename_all = "camelCase")]
struct IndexOptions {
    heap_size: f64,
    reload_on: ReloadOnPolicy,
}

impl Default for IndexOptions {
    fn default() -> Self {
        Self {
            heap_size: 10_000_000.0,
            reload_on: ReloadOnPolicy::CommitWithDelay,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, neon::TypeScript)]
#[serde(default, rename_all = "camelCase")]
struct SearchOptions {
    top: f64,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            top: 10.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, neon::TypeScript)]
#[serde(default, rename_all = "camelCase")]
struct TextAnalyzerFilters {
    remove_long: Option<f64>,
    alpha_num_only: bool,
    ascii_folding: bool,
    lower_case: bool,
    // TODO: split_compound_words
    stemmer: Option<LanguageName>,
    filter_stop_words: Option<LanguageName>,
}

impl Default for TextAnalyzerFilters {
    fn default() -> Self {
        Self {
            remove_long: None,
            alpha_num_only: false,
            ascii_folding: false,
            lower_case: false,
            stemmer: None,
            filter_stop_words: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, neon::TypeScript)]
struct FuzzyTermQueryOptions {
    max_distance: u32,
    transposition_costs_one: bool,
    is_prefix: bool,
}

impl std::default::Default for FuzzyTermQueryOptions {
    fn default() -> Self {
        Self {
            max_distance: 0,
            transposition_costs_one: true,
            is_prefix: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, neon::TypeScript)]
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
    fn apply<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> t::TextAnalyzer {
        // Step through the filters one at a time. This tail recursive style
        // allows each method to take a generic base type since the specific
        // base type depends on which filters actually get applied, but the
        // result type always ends up as TextAnalyzer.
        self.apply_remove_long(builder)
    }

    fn apply_remove_long<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> t::TextAnalyzer {
        match self.remove_long {
            // TODO: better cast
            Some(n) => self.apply_alpha_num_only(builder.filter(RemoveLongFilter::limit(n as usize))),
            None => self.apply_alpha_num_only(builder),
        }
    }

    fn apply_alpha_num_only<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> t::TextAnalyzer {
        match self.alpha_num_only {
            true => self.apply_ascii_folding(builder.filter(AlphaNumOnlyFilter)),
            false => self.apply_ascii_folding(builder),
        }
    }

    fn apply_ascii_folding<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> t::TextAnalyzer {
        match self.ascii_folding {
            true => self.apply_lower_caser(builder.filter(AsciiFoldingFilter)),
            false => self.apply_lower_caser(builder),
        }
    }

    fn apply_lower_caser<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> t::TextAnalyzer {
        match self.lower_case {
            true => self.apply_stemmer(builder.filter(LowerCaser)),
            false => self.apply_stemmer(builder),
        }
    }

    fn apply_stemmer<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> t::TextAnalyzer {
        match self.stemmer {
            Some(language) => self.apply_stop_words(builder.filter(Stemmer::new(language.into()))),
            None => self.apply_stop_words(builder),
        }
    }

    fn apply_stop_words<T: Tokenizer>(self, builder: TextAnalyzerBuilder<T>) -> t::TextAnalyzer {
        match self.filter_stop_words {
            Some(language) => builder.filter(StopWordFilter::new(language.into()).unwrap()).build(),
            None => builder.build(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, neon::TypeScript)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum IndexRecordOption {
    #[default]
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, neon::TypeScript)]
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

#[derive(Serialize, Deserialize, Debug, Clone, neon::TypeScript)]
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

#[derive(Serialize, Deserialize, Debug, Clone, neon::TypeScript)]
enum TextOption {
    STORED,
}

#[derive(Serialize, Deserialize, Debug, Clone, neon::TypeScript)]
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

#[derive(Clone)]
struct Schema {
    schema: RefCell<t::Schema>,
    fields: OrderMap<String, FieldDescriptor>,
}

#[neon::export(class)]
impl Schema {
    fn new(Json(fields): Json<OrderMap<String, FieldDescriptor>>) -> Self {
        let mut builder = t::Schema::builder();
        for (field_name, options) in fields.iter() {
            add_field(&mut builder, field_name, options);
        }
        Self {
            schema: RefCell::new(builder.build()),
            fields: fields,
        }
    }

    fn fields(&self) -> Json<OrderMap<String, FieldDescriptor>> {
        Json(self.fields.clone())
    }
}

#[derive(Clone)]
struct Searcher {
    searcher: Arc<t::Searcher>,
}

impl Searcher {
    fn interpret_field(&self, field: &str) -> Result<Field, Error> {
        Ok(self.searcher
            .index()
            .schema()
            .get_field(field)?)
    }
}

#[neon::export(class)]
impl Searcher {
    fn new(
        index: Arc<OpenIndex>,
    ) -> Result<Self, Error> {
        Ok(Self {
            searcher: Arc::new(index.reader.lock().map_err(|_| "mutex poisoned")?.searcher()),
        })
    }

    fn term_query(
        &self,
        term: String,
        field: String,
        options: Option<Json<IndexRecordOption>>,
    ) -> Result<Query, Error> {
        let field = self.interpret_field(&field)?;
        let term = Term::from_field_text(field, &term);
        let Json(options) = options.unwrap_or(Json(IndexRecordOption::default()));
        let query = TermQuery::new(term, options.into());
        Ok(Query { query: Arc::new(Box::new(query)) })
    }

    fn phrase_query(
        &self,
        Json(terms): Json<Vec<String>>,
        field: String,
    ) -> Result<Query, Error> {
        let field = self.interpret_field(&field)?;
        let terms = terms.into_iter().map(|term| {
            Term::from_field_text(field, &term)
        }).collect();
        let query = PhraseQuery::new(terms);
        Ok(Query { query: Arc::new(Box::new(query)) })
    }

    fn fuzzy_term_query(
        &self,
        term: String,
        field: String,
        options: Option<Json<FuzzyTermQueryOptions>>,
    ) -> Result<Query, Error> {
        let field = self.interpret_field(&field)?;
        let term = Term::from_field_text(field, &term);
        let Json(options) = options.unwrap_or(Json(FuzzyTermQueryOptions::default()));
        let query = if options.is_prefix {
            FuzzyTermQuery::new_prefix(term, options.max_distance as u8, options.transposition_costs_one)
        } else {
            FuzzyTermQuery::new(term, options.max_distance as u8, options.transposition_costs_one)
        };
        Ok(Query { query: Arc::new(Box::new(query)) })
    }

    fn regexp_query(
        &self,
        pattern: String,
        field: String,
    ) -> Result<Query, Error> {
        let field = self.interpret_field(&field)?;
        let query = RegexQuery::from_pattern(&pattern, field)?;
        Ok(Query { query: Arc::new(Box::new(query)) })
    }

    fn phrase_prefix_query(
        &self,
        Json(terms): Json<Vec<String>>,
        field: String,
    ) -> Result<Query, Error> {
        let field = self.interpret_field(&field)?;
        let terms = terms.into_iter().map(|term| {
            Term::from_field_text(field, &term)
        }).collect();
        let query = PhrasePrefixQuery::new(terms);
        Ok(Query { query: Arc::new(Box::new(query)) })
    }

    fn search_sync(
        &self,
        query: &Query,
        Json(options): Json<Option<SearchOptions>>,
    ) -> Json<Vec<(Score, String, Explanation)>>{
        let index = self.searcher.index();
        let schema = index.schema();
        let options = options.unwrap_or_default();
        let collector = TopDocs::with_limit(options.top as usize);
        Json(
            self.searcher
                .search(query.query.as_ref(), &collector)
                .unwrap()
                .iter()
                .map(|&(score, doc_address)| {
                    let retrieved_doc: TantivyDocument = self.searcher.doc(doc_address).unwrap();
                    (score, retrieved_doc.to_json(&schema), query.query.explain(&self.searcher, doc_address).unwrap())
                })
                .collect::<Vec<_>>()
        )
    }

    #[neon(task)]
    fn search(
        self,
        query: Query,
        options: Json<Option<SearchOptions>>,
    ) -> Json<Vec<(Score, String, Explanation)>>{
        self.search_sync(&query, options)
    }

    fn search_terms(
        &self,
        field: String,
        pattern: String,
    ) -> Result<Json<Vec<String>>, Error>
    {
        let readers = self.searcher.segment_readers();
        let field = self.interpret_field(&field)?;
        let mut result = vec![];
        for reader in readers {
            let inverted_index = reader.inverted_index(field)?;
            let dict = inverted_index.terms();
            let mut stream = dict.search(Regex::new(&pattern)?).into_stream()?;
            while let Some((term, _)) = stream.next() {
                let term = String::from_utf8_lossy(term);
                result.push(term.to_string());
            }
        }
        Ok(Json(result))
    }
}

#[derive(Clone)]
struct TextAnalyzer {
    analyzer: RefCell<t::TextAnalyzer>,
}

#[neon::export(class)]
impl TextAnalyzer {
    fn new(
        filters: Option<Json<TextAnalyzerFilters>>,
    ) -> Result<Self, Error> {
        // TODO: need a way to build off something other than a simple tokenizer
        let builder = t::TextAnalyzer::builder(SimpleTokenizer::default());
        let Json(filters) = filters.unwrap_or(Json(TextAnalyzerFilters::default()));
        let analyzer = filters.apply(builder);
        Ok(Self {
            analyzer: RefCell::new(analyzer),
        })
    }

    fn tokenize(&mut self, text: String) -> Json<Vec<Token>> {
        let mut analyzer = self.analyzer.borrow_mut();
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
}

#[derive(Clone)]
struct Index {
    index: Arc<OpenIndex>,
}

#[neon::export(class)]
impl Index {
    fn new(
        path: String,
        schema: Schema,
        Json(options): Json<Option<IndexOptions>>,
    ) -> Result<Self, Error> {
        let dir_path = PathBuf::from(path);
        let dir = tantivy::directory::MmapDirectory::open(dir_path)?;
        let index = t::Index::create(dir, schema.schema.borrow().clone(), IndexSettings::default())?;
        let options = options.unwrap_or_default();
        let reader = Mutex::new(
            index
                .reader_builder()
                .reload_policy(options.reload_on.into())
                .try_into()?
        );
        let heap_size: u53 = options.heap_size.project()?;
        let heap_size: u64 = heap_size.into();
        let heap_size: usize = heap_size.try_into()?;
        let writer = Mutex::new(index.writer(heap_size)?);
        Ok(Self {
            index: Arc::new(OpenIndex { index, writer, reader }),
        })
    }

    #[neon(task)]
    fn commit(self) -> Result<(), Error> {
        self.commit_sync()
    }

    fn commit_sync(&self) -> Result<(), Error> {
        self.index.writer.lock().map_err(|_| "mutex poisoned")?.commit()?;
        Ok(())
    }

    #[neon(task)]
    fn reload(self) -> Result<(), Error> {
        self.reload_sync()
    }

    fn reload_sync(&self) -> Result<(), Error> {
        self.index.reader.lock().map_err(|_| "mutex poisoned")?.reload()?;
        Ok(())
    }

    fn add_document<'cx>(
        &self,
        cx: &mut FunctionContext<'cx>,
        Json(document): Json<serde_json::Map<String, serde_json::Value>>,
    ) -> JsResult<'cx, JsBigInt> {
        let document = match TantivyDocument::from_json_object(&self.index.index.schema(), document) {
            Ok(doc) => doc,
            Err(err) => {
                return cx.throw_error(format!("failed to parse document: {}", err));
            }
        };
        let stamp = self.index.writer
            .lock()
            .map_err(|_| "mutex poisoned").unwrap()
            .add_document(document).unwrap();
        Ok(JsBigInt::from_u64(cx, stamp))
    }

    fn searcher(&self) -> Result<Searcher, Error> {
        Searcher::new(self.index.clone())
    }

    fn register_tokenizer(
        &self,
        name: String,
        tokenizer: TextAnalyzer,
    ) {
        let manager = self.index.index.tokenizers();
        manager.register(&name, tokenizer.analyzer.borrow().clone());
    }
}

#[derive(Clone)]
struct Query {
    query: Arc<Box<dyn t::Query>>,
}

#[neon::export(class)]
impl Query {
    fn new(
        query: Arc<Box<dyn t::Query>>,
    ) -> Self {
        Self { query }
    }
}

struct OpenIndex {
    index: t::Index,
    writer: Mutex<IndexWriter>,
    reader: Mutex<IndexReader>,
}

#[derive(Serialize, Deserialize, Debug, neon::TypeScript)]
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
fn generate_typescript_declarations() -> String {
    neon::typescript::generate()
}
