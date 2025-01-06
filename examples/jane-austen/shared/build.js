import * as fs from 'fs/promises';
import { Index, Schema } from 'tantivy';
import { getTestIndexPath } from 'utils';

const BOOKS = [
  'emma',
  'lady-susan',
  'mansfield-park',
  'northanger-abbey',
  'persuasion',
  'pride-and-prejudice',
  'sense-and-sensibility'
];

const PARAGRAPH_INDEX = {
  schema: {
    "_id": { type: "f64" },
    "title": { type: "text", flags: ["STORED"] },
    "author": { type: "text", flags: ["STORED"] },
    "url": { type: "text", flags: ["STORED"] },
    "year": { type: "f64", flags: ["STORED"] },
    "volume": { type: "f64", flags: ["STORED"] },
    "chapter": { type: "text", flags: ["STORED"] },
    "paragraph": { type: "f64", flags: ["STORED"] },
    "text": {
      type: "text",
      flags: ["STORED"],
      tokenizer: "jane_austen",
      // Needed for PhraseQuery
      index: "WITH_FREQS_AND_POSITIONS"
    },
  },
  heapSize: 20_000_000,
  cacheDir: "austen-paragraphs",
  sourceDir: "by-paragraph",
}

export default async function buildIndex(tokenizer) {
  const schema = new Schema(PARAGRAPH_INDEX.schema);
  const index = new Index({
    path: getTestIndexPath(PARAGRAPH_INDEX.cacheDir),
    heapSize: PARAGRAPH_INDEX.heapSize,
    schema,
    reloadOn: 'COMMIT_WITH_DELAY',
  });

  index.registerTokenizer('jane_austen', tokenizer);

  // TODO: try this concurrently
  for (const book of BOOKS) {
    // TODO: use a streaming JSON lines reader
    const docs = JSON.parse(await fs.readFile(`${import.meta.dirname}/${PARAGRAPH_INDEX.sourceDir}/${book}.json`, 'utf8'));
    for (const doc of docs) {
      doc.raw = doc.text;
      await index.addDocument(doc);
    }
  }

  await index.commit();
  await index.reload();

  return index;
}
