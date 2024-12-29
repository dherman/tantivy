const { rmSync, mkdirSync, existsSync } = require('fs');
const path = require('path');

const INDEX_PATH = path.join(__dirname, '..', '.test-index');

function getTestIndexPath(indexName = "test") {
  const indexPath = path.join(INDEX_PATH, indexName);
  if (existsSync(indexPath)) {
    rmSync(indexPath, { recursive: true });
  }
  mkdirSync(indexPath, { recursive: true });
  return indexPath;
}

async function benchmark(thunk) {
  const start = performance.now();
  const result = await thunk();
  const end = performance.now();
  return {
    result,
    time: end - start
  };
}

module.exports.getTestIndexPath = getTestIndexPath;
module.exports.benchmark = benchmark;
