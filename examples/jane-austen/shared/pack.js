#!/usr/bin/env node

import * as fs from 'fs';
import * as path from 'path';
import { splitSentences, tokenizeSentence, ngrams } from './tokenize.js';

function die(error = null) {
  console.error(`
    pack.js - Pack Jane Austen book data into JSON for loading into an index
    
    Usage: pack.js (paragraph|phrase) <srcdir> <destdir>
    
      paragraph: Split books into paragraphs
      phrase:    Split books into phrases
`.trim());
  console.error();

  if (error) {
    console.error(error);
  }

  process.exit(1);
}

if (process.argv.length < 5) {
  die();
}

function extractBookPhrases(output, srcdir, contents) {
  for (const chapter of contents) {
    const text = fs.readFileSync(path.join(srcdir, chapter.filename), 'utf8');
    const sentences = splitSentences(text).map(tokenizeSentence);

    for (const sentence of sentences) {
      const phrases = ngrams(sentence, 1, 3);
      for (const phrase of phrases) {
        output.add(phrase.join(' '));
      }
    }
  }
}

function packPhrases(source, dest) {
  const output = new Set();
  const books = subdirs(source);
  for (const book of books) {
    const contents = JSON.parse(fs.readFileSync(path.join(source, book, "contents.json"), 'utf8'));
    extractBookPhrases(output, path.join(source, book), contents);
  }
  fs.mkdirSync(dest, { recursive: true });
  fs.writeFileSync(path.join(dest, "phrases.txt"), [...output].join('\n'));
}

function splitBookByParagraph(srcdir, metadata, contents, id) {
  const result = [];
  for (const chapter of contents) {
    const paragraphs = fs.readFileSync(path.join(srcdir, chapter.filename), 'utf8')
      .replaceAll('\r\n', '\n')
      .split('\n\n')
      .map(p => p.trim())
      .filter(p => p.length);

    let paragraphID = 0;

    for (const paragraph of paragraphs) {
      const record = {
        _id: id
      };
      id++;
      Object.assign(record, metadata);
      if (chapter.volume) {
        record.volume = chapter.volume;
      }
      record.chapter = chapter.chapter;
      record.paragraph = paragraphID;
      paragraphID++;
      record.text = paragraph;
      result.push(record);
    }
  }
  return result;
}

function subdirs(dir) {
  return fs.readdirSync(dir)
    .filter(file => fs.statSync(path.join(dir, file)).isDirectory());
}

function packBooks(source, dest, splitBook) {
  fs.mkdirSync(dest, { recursive: true });
  const books = subdirs(source);
  let id = 0;
  for (const book of books) {
    const metadata = JSON.parse(fs.readFileSync(path.join(source, book, "meta.json"), 'utf8'));
    const contents = JSON.parse(fs.readFileSync(path.join(source, book, "contents.json"), 'utf8'));
    const paragraphs = splitBook(path.join(source, book), metadata, contents, id);
    id += paragraphs.length;
    // TODO: write as JSON lines
    fs.writeFileSync(path.join(dest, `${book}.json`), JSON.stringify(paragraphs, null, 2));
    console.error(`Wrote: ${dest}/${book}.json`);
  }
}

const indexType = process.argv[2];
const source = process.argv[3];
const dest = process.argv[4];

switch (indexType) {
  case 'paragraph':
    // TODO: undo the abstraction that takes the splitter function
    packBooks(source, dest, splitBookByParagraph);
    break;
  case 'phrase':
    packPhrases(source, dest);
    break;
  default:
    die("expected 'paragraph' or 'phrase'");
}
