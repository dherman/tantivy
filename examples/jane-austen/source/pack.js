#!/usr/bin/env node

const fs = require('fs');

if (process.argv.length < 3) {
  console.error('Usage: pack.js <title>');
  process.exit(1);
}

const title = process.argv[2];

function readContent(desc) {
  var result = {};
  for (var key in desc) {
    if (key === 'filename') {
      continue;
    }
    result[key] = desc[key];
  }
  result.text = fs
    .readFileSync(`./${title}/${desc.filename}`, 'utf8')
    .replace(/\r\n/g, '\n')
    .trim();
  return result;
}

const metadata = require(`./${title}/meta.json`);
const book = {
  title: metadata.title,
  author: metadata.author,
  url: metadata.url,
  year: metadata.year,
  contents: metadata.contents.map(readContent)
};

fs.writeFileSync(`${title}.json`, JSON.stringify(book, null, 2));

console.error(`Wrote: ${title}.json`);
