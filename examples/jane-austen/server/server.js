const express = require('express');
const path = require('path');
const cors = require('cors');
const buildIndex = require('../index.js');
const CORS_OPTIONS = {
  origin: ["http://localhost:5173"],
};

const app = express();
const PORT = process.env.PORT || 5174;

// Middleware
app.use(cors(CORS_OPTIONS));
app.use(express.json());

const AUSTEN_HONORIFICS = ["Mr.", "Mrs.", "Ms.", "Dr."];
const AUSTEN_PUNCTUATION = /“|”|\s+|!|\?|;|:|_|,|—|–|-|\(|\)/;

function stripFullStop(token) {
  return AUSTEN_HONORIFICS.includes(token) ? token : token.replace(/\.$/, "");
}

function tokenize(text) {
  return text.split(AUSTEN_PUNCTUATION)
    .filter(s => s.length)
    .map(stripFullStop);
}

// Build the index before starting the server
buildIndex()
  .then(index => {
    // Root route
    app.get('/', (req, res) => {
      const searcher = index.searcher();
      const terms = req.query.q.split(/\s+/);
      const query = terms.length > 1
        ? searcher.phrasePrefixQuery(terms, "text")
        : (console.error(`query: "${req.query.q}"`), searcher.fuzzyTermQuery(req.query.q, "text", {
          maxDistance: 0,
          isPrefix: true
        }));

      searcher.search(query, { top: 10 }).then(results => {
        const items = results.map(([score, docSrc, _explanation]) => {
          const doc = JSON.parse(docSrc);
          doc.query = req.query.q;
          doc.author = doc.author[0];
          doc.volume = (doc.volume && doc.volume.length) ? doc.volume[0] : null;
          doc.chapter = doc.chapter[0];
          doc.paragraph = doc.paragraph[0];
          doc.text = doc.text[0];
          doc.title = doc.title[0];
          doc.url = doc.url[0];
          doc.year = doc.year[0];
          doc.clip = doc.text.length > 80 ? `${doc.text.slice(0, 77)}...` : doc.text;
          doc.icon = `${doc.title.replaceAll(' ', '-').toLowerCase()}.jpg`;
          doc.score = score;
          return doc;
        });
        res.json({ items });
      });
    });

    // Start server
    app.listen(PORT, () => {
      console.log(`Server running on port ${PORT}`);
    });
  });
