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

// Build the index before starting the server
buildIndex()
  .then(index => {
    // Root route
    app.get('/', (req, res) => {
      const searcher = index.searcher();
      searcher.search(req.query.q, {
        fields: ["text"],
        top: 10
      }).then(results => {
        const items = results.map(([score, docSrc, _explanation]) => {
          const doc = JSON.parse(docSrc);
          console.error(docSrc);
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

/*
// Root route
app.get('/', (req, res) => {
  res.json({
    items: [{
      login: "dherman",
      avatar_url: "https://avatars.githubusercontent.com/u/307871?v=4",
    }],
  });
});


// Start server
app.listen(PORT, () => {
  console.log(`Server running on port ${PORT}`);
});
*/
