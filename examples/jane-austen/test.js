import { buildParagraphIndex, buildPhraseIndex } from './shared/build.js';
import { benchmark } from 'utils';

async function test() {
  console.error("Building paragraph index...");
  const { result: paragraphIndex, time: paragraphTime } = await benchmark(() => buildParagraphIndex());
  console.error(`Build time: ${paragraphTime}ms`);
  const paragraphs = paragraphIndex.searcher();
  console.error("Building phrase index...");
  const { result: phraseIndex, time: phraseTime } = await benchmark(() => buildPhraseIndex());
  console.error(`Build time: ${phraseTime}ms`);
  const phrases = phraseIndex.searcher();
  return {
    paragraphs: await benchmark(async () => {
      console.error("Searching paragraphs...");
      return await paragraphs.search("love", {
        fields: ["text"],
        top: 10
      });
    }),
    phrases: await benchmark(async () => {
      console.error("Searching phrases...");
      return await phrases.search("love", {
        fields: ["text"],
        top: 10
      });
    }),
  };
}

test()
  .then(result => {
    console.error("Paragraph search:");
    const paragraphsSummary = result.paragraphs.result.map(([score, doc, _explanation]) => {
      return { score, doc: JSON.parse(doc) };
    });
    console.log(JSON.stringify(paragraphsSummary, 0, 2));
    console.error(`Search time: ${result.paragraphs.time}ms`);

    console.error("Phrase search:");
    const phrasesSummary = result.phrases.result.map(([score, doc, _explanation]) => {
      return { score, doc: JSON.parse(doc) };
    });
    console.log(JSON.stringify(phrasesSummary, 0, 2));
    console.error(`Search time: ${result.phrases.time}ms`);
  })
  .catch(error => {
    console.error(error);
  });
