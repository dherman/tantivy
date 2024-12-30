const express = require('express');
const path = require('path');
const cors = require('cors');
const { buildParagraphIndex, buildPhraseIndex } = require('../shared/build.js');

const CORS_OPTIONS = {
  origin: ["http://localhost:5173"],
};

const app = express();
const PORT = process.env.PORT || 5174;

// Middleware
app.use(cors(CORS_OPTIONS));
app.use(express.json());

// // TODO: align this with tokenize.js and move any needed tokenization logic there
// const AUSTEN_HONORIFICS = ["Mr.", "Mrs.", "Ms.", "Dr."];
// const AUSTEN_PUNCTUATION = /“|”|\s+|!|\?|;|:|_|,|—|–|-|\(|\)/;

// function stripFullStop(token) {
//   return AUSTEN_HONORIFICS.includes(token) ? token : token.replace(/\.$/, "");
// }

// function tokenize(text) {
//   return text.split(AUSTEN_PUNCTUATION)
//     .filter(s => s.length)
//     .map(stripFullStop);
// }

async function startup() {
  return {
    paragraphIndex: await buildParagraphIndex(),
    phraseIndex: await buildPhraseIndex()
  };
}

function prefixSort(arr, query) {
  let prefixArray = [];
  let notPrefixArray = [];

  for (const elt of arr) {
    if (elt.startsWith(query)) {
      prefixArray.push(elt);
    } else {
      notPrefixArray.push(elt);
    }
  }

  return [...prefixArray.sort(), ...notPrefixArray.sort()];
}

function serve({ paragraphIndex, phraseIndex }) {
  // Typeahead route
  app.get('/typeahead/', (req, res) => {
    const searcher = phraseIndex.searcher();
    const terms = req.query.q.split(/\s+/);
    const query = terms.length > 1
      ? searcher.phrasePrefixQuery(terms.map(t => t.toLowerCase()), "text")
      : searcher.fuzzyTermQuery(req.query.q.toLowerCase(), "text", {
        maxDistance: 0,
        isPrefix: true
      });

    const startTime = performance.now();

    searcher.search(query, { top: 50 }).then(results => {
      const endTime = performance.now();

      // TODO: case insensitive
      const items = results.map(([_score, docSrc, _explanation]) => {
        const doc = JSON.parse(docSrc);
        return doc.text[0];
      });
      const sorted = prefixSort(items, terms.join(' '));
      res.json({
        time: endTime - startTime,
        items: sorted,
      });
    });
  });

  // Search route
  app.get('/search/', (req, res) => {
    const searcher = paragraphIndex.searcher();
    const terms = req.query.q.split(/\s+/).map(t => t.trim()).filter(t => t.length);
    if (terms.length === 0) {
      res.json([]);
      return;
    }

    const query = terms.length > 1
      ? searcher.phrasePrefixQuery(terms.map(t => t.toLowerCase()), "text")
      : searcher.fuzzyTermQuery(req.query.q.toLowerCase(), "text", {
        maxDistance: 0,
        isPrefix: true
      });

    const startTime = performance.now();

    searcher.search(query, { top: 10 }).then(results => {
      const endTime = performance.now();
      const items = results.map(([_score, docSrc, _explanation]) => {
        // Since a TantivyDocument may contain duplicate fields,
        // each field is represented in JSON as an array:
        //
        // https://docs.rs/tantivy/0.22.0/tantivy/schema/document/struct.TantivyDocument.html
        const doc = JSON.parse(docSrc);
        return {
          icon: `${doc.title[0].replaceAll(' ', '-').toLowerCase()}.jpg`,
          title: doc.title[0],
          text: doc.text[0],
          volume: (doc.volume && doc.volume.length) ? doc.volume[0] : null,
          chapter: doc.chapter[0],
          url: doc.url[0],
        };
      });
      res.json({
        time: endTime - startTime,
        items,
      });
    });
  });

  // Start server
  app.listen(PORT, () => {
    console.log(`Server running on port ${PORT}`);
  });
}

startup()
  .then((indices) => {
    serve(indices);
  });
