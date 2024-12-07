const { SchemaBuilder, Index, IndexWriter, Search, QueryParser, TopDocs } = require('..');
const { rmSync, mkdirSync, existsSync } = require('fs');

const INDEX_PATH = `${__dirname}/../data`;

rmSync(INDEX_PATH, { recursive: true });
mkdirSync(INDEX_PATH);

const schemaBuilder = new SchemaBuilder();

const id = schemaBuilder.addTextField("_id", ["STRING"]);
const title = schemaBuilder.addTextField("title", ["TEXT", "STORED"]);
const year = schemaBuilder.addTextField("year", ["TEXT", "STORED"]);
const url = schemaBuilder.addTextField("url", ["TEXT", "STORED"]);

const schema = schemaBuilder.build();

const index = new Index(schema, INDEX_PATH);
const indexWriter = new IndexWriter(index, 100000000);

const search = new Search(index, schema, indexWriter, [id, title, year, url]);

search.addDoc({
    _id: "1",
    title: "The Economic History of the Fur Trade: 1670 to 1870",
    year: "2008",
    authors: ["Ann M. Carlos, University of Colorado", "Frank D. Lewis, Queen’s University"],
    url: "http://eh.net/encyclopedia/the-economic-history-of-the-fur-trade-1670-to-1870/"
});
search.commit();

const parser = new QueryParser(search);
const query = parser.parse("fur");
const collector = new TopDocs(10);

const results = search.topSearch(query, collector);

console.log(results);
