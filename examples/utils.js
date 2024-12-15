const { rmSync, mkdirSync, existsSync } = require('fs');

const INDEX_PATH = `${__dirname}/../.test-index`;

function getTestIndexPath() {
  if (existsSync(INDEX_PATH)) {
    rmSync(INDEX_PATH, { recursive: true });
  }
  mkdirSync(INDEX_PATH);
  return INDEX_PATH;
}

module.exports.getTestIndexPath = getTestIndexPath;
