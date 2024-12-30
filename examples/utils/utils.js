import { rmSync, mkdirSync, existsSync } from 'fs';
import * as path from 'path';

const INDEX_PATH = path.join(import.meta.dirname, '..', '..', '.test-index');

export function getTestIndexPath(indexName = "test") {
  const indexPath = path.join(INDEX_PATH, indexName);
  if (existsSync(indexPath)) {
    rmSync(indexPath, { recursive: true });
  }
  mkdirSync(indexPath, { recursive: true });
  return indexPath;
}

export async function benchmark(thunk) {
  const start = performance.now();
  const result = await thunk();
  const end = performance.now();
  return {
    result,
    time: end - start
  };
}
