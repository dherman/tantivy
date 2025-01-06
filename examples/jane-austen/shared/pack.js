#!/usr/bin/env node

import * as fs from 'fs';
import * as path from 'path';

function die(error = null) {
  console.error(`
    pack.js - Pack Jane Austen book data into JSON for loading into an index
    
    Usage: pack.js <srcdir> <destdir>
`.trim());
  console.error();

  if (error) {
    console.error(error);
  }

  process.exit(1);
}

if (process.argv.length < 4) {
  die();
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

function packBooks(source, dest) {
  fs.mkdirSync(dest, { recursive: true });
  const books = subdirs(source);
  let id = 0;
  for (const book of books) {
    const metadata = JSON.parse(fs.readFileSync(path.join(source, book, "meta.json"), 'utf8'));
    const contents = JSON.parse(fs.readFileSync(path.join(source, book, "contents.json"), 'utf8'));
    const paragraphs = splitBookByParagraph(path.join(source, book), metadata, contents, id);
    id += paragraphs.length;
    // TODO: write as JSON lines
    fs.writeFileSync(path.join(dest, `${book}.json`), JSON.stringify(paragraphs, null, 2));
    console.error(`Wrote: ${dest}/${book}.json`);
  }
}

const source = process.argv[2];
const dest = process.argv[3];

packBooks(source, dest);
