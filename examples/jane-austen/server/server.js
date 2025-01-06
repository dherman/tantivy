import express from 'express';
import cors from 'cors';
import buildIndex from 'shared/build';
import { TextAnalyzer } from 'tantivy';

const CORS_OPTIONS = {
  origin: ["http://localhost:5173"],
};

const app = express();
const PORT = process.env.PORT || 5174;

// Middleware
app.use(cors(CORS_OPTIONS));
app.use(express.json());

const tokenizer = new TextAnalyzer({
  asciiFolding: true,
  lowerCase: true
});

function restoreCase(completed, typed) {
  const completedLC = completed.toLowerCase();
  const typedLC = typed.toLowerCase();
  let i = 0;
  while (i < completedLC.length && i < typedLC.length && completedLC[i] === typedLC[i]) {
    i++;
  }
  return typed.substring(0, i) + completed.substring(i);
}

function findMatches(text, queryTokens) {
  let result = [];
  const textTokens = tokenizer.tokenize(text);
  const queryLength = queryTokens.length;
  let i = 0;
  while ((i + queryLength - 1) < textTokens.length) {
    if (queryTokens.every((qt, j) => textTokens[i + j].text === qt.text)) {
      result.push({ charOffsetFrom: textTokens[i].charOffsetFrom, charOffsetTo: textTokens[i + queryLength - 1].charOffsetTo });
      i += queryLength;
    } else {
      i++;
    }
  }
  return result;
}

function serve(index) {
  // Typeahead route
  app.get('/typeahead/', (req, res) => {
    const startTime = performance.now();
    const paragraphs = index.searcher();
    const tokens = tokenizer.tokenize(req.query.q);
    // TODO: restore common punctuation removed by tokenization
    //       that will feel more natural to the user, e.g.:
    //       - "Mr.", "Mrs.", "Rev.", "Dr."
    //       - "Emma’s", "Emma's"
    //       - "well-looking", "good-bye"
    const terms = tokens.map(token => req.query.q.substring(token.charOffsetFrom, token.charOffsetTo));
    const lastTermTyped = terms[terms.length - 1];
    const lastTermOptions = paragraphs
      .searchTerms("text", `${lastTermTyped.toLowerCase()}.*`)
      .map(lastTermOption => restoreCase(lastTermOption, lastTermTyped));
    const start = terms.slice(0, terms.length - 1);
    const completions = lastTermOptions.map(lastTermOption => [...start, lastTermOption]);
    const endTime = performance.now();
    res.json({
      time: endTime - startTime,
      items: completions,
    });
  });

  // Search route
  app.get('/search/', (req, res) => {
    const startTime = performance.now();
    const query = req.query.q;
    const tokens = tokenizer.tokenize(query);
    const terms = tokens.map(token => query.substring(token.charOffsetFrom, token.charOffsetTo).toLowerCase());
    const searcher = index.searcher();
    const queryObject = terms.length === 1
      ? searcher.termQuery(terms[0], "text", "WITH_FREQS_AND_POSITIONS")
      : searcher.phraseQuery(terms, "text");
    searcher.search(queryObject, { top: 10 }).then(results => {
      const items = results.map(([_score, docSrc, _explanation]) => {
        // Since a TantivyDocument may contain duplicate fields,
        // each field is represented in JSON as an array:
        //
        // https://docs.rs/tantivy/0.22.0/tantivy/schema/document/struct.TantivyDocument.html
        const doc = JSON.parse(docSrc);
        const matches = findMatches(doc.text[0], tokens);
        return {
          icon: `${doc.title[0].replaceAll(' ', '-').toLowerCase()}.jpg`,
          title: doc.title[0],
          text: doc.text[0],
          volume: (doc.volume && doc.volume.length) ? doc.volume[0] : null,
          chapter: doc.chapter[0],
          url: doc.url[0],
          matches,
        };
      });
      const endTime = performance.now();
      res.json({
        time: endTime - startTime,
        items,
        queryTokens: tokens,
      });
    });
  });

  // Start server
  app.listen(PORT, () => {
    console.log(`Server running on port ${PORT}`);
  });
}

buildIndex(tokenizer).then(serve);
