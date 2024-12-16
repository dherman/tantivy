const { rmSync, mkdirSync, existsSync } = require('fs');

const INDEX_PATH = `${__dirname}/../.test-index`;

function getTestIndexPath() {
  if (existsSync(INDEX_PATH)) {
    rmSync(INDEX_PATH, { recursive: true });
  }
  mkdirSync(INDEX_PATH);
  return INDEX_PATH;
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
