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

const PHRASE_INDEX = {
  schema: {
    "_id": { type: "f64" },
    "text": { type: "text", flags: ["STORED"] },
  },
  heapSize: 50_000_000,
  cacheDir: "austen-phrases",
  sourceDir: "by-phrase",
};

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
    "text": { type: "text", flags: ["STORED"] },
  },
  heapSize: 20_000_000,
  cacheDir: "austen-paragraphs",
  sourceDir: "by-paragraph",
}

export async function buildPhraseIndex() {
  const schema = new Schema(PHRASE_INDEX.schema);
  const index = new Index({
    path: getTestIndexPath(PHRASE_INDEX.cacheDir),
    heapSize: PHRASE_INDEX.heapSize,
    schema,
    reloadOn: 'COMMIT_WITH_DELAY',
  });

  // TODO: use a streaming line reader
  const phrases = (await fs.readFile(`${import.meta.dirname}/${PHRASE_INDEX.sourceDir}/phrases.txt`, 'utf8')).split('\n');
  let _id = 0;

  for (const phrase of phrases) {
    await index.addDocument({
      "_id": _id,
      "text": phrase.trim(),
    });
    _id++;
  }

  await index.commit();
  await index.reload();

  return index;
}

export async function buildParagraphIndex() {
  const schema = new Schema(PARAGRAPH_INDEX.schema);
  const index = new Index({
    path: getTestIndexPath(PARAGRAPH_INDEX.cacheDir),
    heapSize: PARAGRAPH_INDEX.heapSize,
    schema,
    reloadOn: 'COMMIT_WITH_DELAY',
  });

  // TODO: try this concurrently
  for (const book of BOOKS) {
    // TODO: use a streaming JSON lines reader
    const docs = JSON.parse(await fs.readFile(`${import.meta.dirname}/${PARAGRAPH_INDEX.sourceDir}/${book}.json`, 'utf8'));
    for (const doc of docs) {
      await index.addDocument(doc);
    }
  }

  await index.commit();
  await index.reload();

  return index;
}
