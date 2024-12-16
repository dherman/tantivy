#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

if (process.argv.length < 4) {
  console.error('Usage: pack.js <srcdir> <destdir>');
  process.exit(1);
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
  const books = subdirs(source);
  let id = 0;
  for (const book of books) {
    const metadata = JSON.parse(fs.readFileSync(path.join(source, book, "meta.json"), 'utf8'));
    const contents = JSON.parse(fs.readFileSync(path.join(source, book, "contents.json"), 'utf8'));
    const paragraphs = splitBookByParagraph(path.join(source, book), metadata, contents, id);
    id += paragraphs.length;
    fs.writeFileSync(path.join(dest, `${book}.json`), JSON.stringify(paragraphs, null, 2));
    console.error(`Wrote: ${dest}/${book}.json`);
  }
}

const source = process.argv[2];
const dest = process.argv[3];

packBooks(source, dest);
