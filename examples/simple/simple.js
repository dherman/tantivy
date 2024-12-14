const { SchemaBuilder, Index, IndexWriter, Schema, Search, QueryParser, TopDocs } = require('..');
const { rmSync, mkdirSync, existsSync } = require('fs');

const INDEX_PATH = `${__dirname}/../data`;

if (existsSync(INDEX_PATH)) {
  rmSync(INDEX_PATH, { recursive: true });
}
mkdirSync(INDEX_PATH);

async function test() {
  const schema = new Schema({
    "_id": ["STRING"],
    "title": ["TEXT", "STORED"],
    "year": ["TEXT", "STORED"],
    "authors": ["TEXT", "STORED"],
    "url": ["TEXT", "STORED"]
  });

  const index = new Index({
    path: INDEX_PATH,
    heapSize: 100_000_000,
    schema,
    reloadOn: 'COMMIT_WITH_DELAY',
  });

  index.addDocument({
    "_id": "1",
    "title": "The Economic History of the Fur Trade: 1670 to 1870",
    "year": "2008",
    "authors": ["Ann M. Carlos, University of Colorado", "Frank D. Lewis, Queen’s University"],
    "url": "https://www.goodreads.com/book/show/108.2-the_economic_history_of_the_fur_trade",
  });

  await index.commit();
  await index.reload();

  const searcher = index.searcher();

  return await searcher.search("fur", {
    fields: ["title", "url"],
    top: 10
  });
}

test()
  .then(result => {
    console.log(JSON.stringify(result, 0, 2));
  })
  .catch(error => {
    console.error(error);
  });
