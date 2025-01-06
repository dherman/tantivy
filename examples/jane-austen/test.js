import { buildParagraphIndex, buildPhraseIndex } from './shared/build.js';
import { benchmark } from 'utils';
import { TextAnalyzer } from 'tantivy';

const analyzer = new TextAnalyzer({
  lowerCase: true,
  asciiFolding: true,
  stemmer: "English"
});

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
    terms: await benchmark(async () => {
      const tokens1 = analyzer.tokenize("Love is a thing of beauty.");
      const tokens2 = analyzer.tokenize("Mr. Knightley said, \"Emma, I love you.\"");
      console.error(`tokens1: ${JSON.stringify(tokens1)}`);
      console.error(`tokens2: ${JSON.stringify(tokens2)}`);
      console.error("Searching terms...");
      return paragraphs.searchTerms("text", "lov.*");
    }),
    tokenizeParagraph: await benchmark(async () => {
      return analyzer.tokenize("This is insufferable! My dearest friend, I was never so enraged before, and must relieve myself by writing to you, who I know will enter into all my feelings. Who should come on Tuesday but Sir James Martin! Guess my astonishment, and vexation—for, as you well know, I never wished him to be seen at Churchhill. What a pity that you should not have known his intentions! Not content with coming, he actually invited himself to remain here a few days. I could have poisoned him! I made the best of it, however, and told my story with great success to Mrs. Vernon, who, whatever might be her real sentiments, said nothing in opposition to mine. I made a point also of Frederica’s behaving civilly to Sir James, and gave her to understand that I was absolutely determined on her marrying him. She said something of her misery, but that was all. I have for some time been more particularly resolved on the match from seeing the rapid increase of her affection for Reginald, and from not feeling secure that a knowledge of such affection might not in the end awaken a return. Contemptible as a regard founded only on compassion must make them both in my eyes, I felt by no means assured that such might not be the consequence. It is true that Reginald had not in any degree grown cool towards me; but yet he has lately mentioned Frederica spontaneously and unnecessarily, and once said something in praise of her person. He was all astonishment at the appearance of my visitor, and at first observed Sir James with an attention which I was pleased to see not unmixed with jealousy; but unluckily it was impossible for me really to torment him, as Sir James, though extremely gallant to me, very soon made the whole party understand that his heart was devoted to my daughter. I had no great difficulty in convincing De Courcy, when we were alone, that I was perfectly justified, all things considered, in desiring the match; and the whole business seemed most comfortably arranged. They could none of them help perceiving that Sir James was no Solomon; but I had positively forbidden Frederica complaining to Charles Vernon or his wife, and they had therefore no pretence for interference; though my impertinent sister, I believe, wanted only opportunity for doing so. Everything, however, was going on calmly and quietly; and, though I counted the hours of Sir James’s stay, my mind was entirely satisfied with the posture of affairs. Guess, then, what I must feel at the sudden disturbance of all my schemes; and that, too, from a quarter where I had least reason to expect it. Reginald came this morning into my dressing-room with a very unusual solemnity of countenance, and after some preface informed me in so many words that he wished to reason with me on the impropriety and unkindness of allowing Sir James Martin to address my daughter contrary to her inclinations. I was all amazement. When I found that he was not to be laughed out of his design, I calmly begged an explanation, and desired to know by what he was impelled, and by whom commissioned, to reprimand me. He then told me, mixing in his speech a few insolent compliments and ill-timed expressions of tenderness, to which I listened with perfect indifference, that my daughter had acquainted him with some circumstances concerning herself, Sir James, and me which had given him great uneasiness. In short, I found that she had in the first place actually written to him to request his interference, and that, on receiving her letter, he had conversed with her on the subject of it, in order to understand the particulars, and to assure himself of her real wishes. I have not a doubt but that the girl took this opportunity of making downright love to him. I am convinced of it by the manner in which he spoke of her. Much good may such love do him! I shall ever despise the man who can be gratified by the passion which he never wished to inspire, nor solicited the avowal of. I shall always detest them both. He can have no true regard for me, or he would not have listened to her; and she, with her little rebellious heart and indelicate feelings, to throw herself into the protection of a young man with whom she has scarcely ever exchanged two words before! I am equally confounded at her impudence and his credulity. How dared he believe what she told him in my disfavour! Ought he not to have felt assured that I must have unanswerable motives for all that I had done? Where was his reliance on my sense and goodness then? Where the resentment which true love would have dictated against the person defaming me—that person, too, a chit, a child, without talent or education, whom he had been always taught to despise? I was calm for some time; but the greatest degree of forbearance may be overcome, and I hope I was afterwards sufficiently keen. He endeavoured, long endeavoured, to soften my resentment; but that woman is a fool indeed who, while insulted by accusation, can be worked on by compliments. At length he left me, as deeply provoked as myself; and he showed his anger more. I was quite cool, but he gave way to the most violent indignation; I may therefore expect it will the sooner subside, and perhaps his may be vanished for ever, while mine will be found still fresh and implacable. He is now shut up in his apartment, whither I heard him go on leaving mine. How unpleasant, one would think, must be his reflections! but some people’s feelings are incomprehensible. I have not yet tranquillised myself enough to see Frederica. She shall not soon forget the occurrences of this day; she shall find that she has poured forth her tender tale of love in vain, and exposed herself for ever to the contempt of the whole world, and the severest resentment of her injured mother.");
    }),
    paragraphs: await benchmark(async () => {
      console.error("Searching paragraphs dictionary...");
      console.error(paragraphs.searchTerms("text", "make.*"));
      console.error(paragraphs.searchTerms("text", "lov.*"));
      console.error(paragraphs.searchTerms("text", "woodhouse.*"));
      console.error(paragraphs.searchTerms("text", "knight.*"));
      console.error("Searching paragraphs...");
      return await paragraphs.search("love", {
        fields: ["text"],
        top: 10
      });
    }),
    phrases: await benchmark(async () => {
      console.error("Searching phrases...");
      const query = phrases.fuzzyTermQuery("makebelieve", "text", {
        maxDistance: 2,
        isPrefix: true
      });
      return await phrases.search(query, {
        fields: ["text"],
        top: 10
      });
    }),
  };
}

test()
  .then(result => {
    console.error("Term search:");
    console.log(result.terms.result);
    console.error(`Search time: ${result.terms.time}ms`);

    console.error("Tokenize paragraph:");
    console.error(`Token count: ${result.tokenizeParagraph.result.length}`);
    console.error(`Tokenize time: ${result.tokenizeParagraph.time}ms`);

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
