const fs = require('fs/promises');
const { SchemaBuilder, Index, IndexWriter, Schema, Search, QueryParser, TopDocs } = require('../..');
const { getTestIndexPath } = require('../utils');

const INDEX_PATH = getTestIndexPath();
const DATA_PATH = `${__dirname}/data`;

const BOOKS = [
  'emma',
  'lady-susan',
  'mansfield-park',
  'northanger-abbey',
  'persuasion',
  'pride-and-prejudice',
  'sense-and-sensibility'
];

async function buildIndex() {
  const schema = new Schema({
    "_id": { type: "f64" },
    "title": { type: "text", flags: ["STORED"] },
    "author": { type: "text", flags: ["STORED"] },
    "url": { type: "text", flags: ["STORED"] },
    "year": { type: "f64", flags: ["STORED"] },
    "volume": { type: "f64", flags: ["STORED"] },
    "chapter": { type: "text", flags: ["STORED"] },
    "paragraph": { type: "f64", flags: ["STORED"] },
    "text": { type: "text", flags: ["STORED"] },
  });

  const index = new Index({
    path: INDEX_PATH,
    heapSize: 100_000_000,
    schema,
    reloadOn: 'COMMIT_WITH_DELAY',
  });

  // TODO: try this concurrently
  for (const book of BOOKS) {
    const paragraphs = JSON.parse(await fs.readFile(`${DATA_PATH}/${book}.json`, 'utf8'));
    for (const paragraph of paragraphs) {
      await index.addDocument(paragraph);
    }
  }

  await index.commit();
  await index.reload();

  return index;
}

async function test() {
  const index = await buildIndex();
  const searcher = index.searcher();
  return await searcher.search("love", {
    fields: ["text"],
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
