const buildIndex = require('./index.js');
const { benchmark } = require('../utils.js');

async function test() {
  const index = await buildIndex();
  const searcher = index.searcher();
  return benchmark(async () => {
    return await searcher.search("love", {
      fields: ["text"],
      top: 10
    });
  });
}

test()
  .then(result => {
    const summary = result.result.map(([score, doc, _explanation]) => {
      return { score, doc: JSON.parse(doc) };
    });
    console.log(JSON.stringify(summary, 0, 2));
    console.error(`Search time: ${result.time}ms`);
  })
  .catch(error => {
    console.error(error);
  });
