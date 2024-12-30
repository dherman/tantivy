export function splitSentences(text) {
  return text
    // Normalize newlines
    .replaceAll('\r\n', '\n')
    // Use temporary placeholders for honorifics to distinguish from full stops
    // 1. "Mr. B." -> "%Mr% %B%"
    .replaceAll(/(Rev|[DM]rs?)\.\s+([A-Z])\./g, "%$1% %$2%")
    // 2. "Mr. Knightley" -> "%Mr% Knightley"
    .replaceAll(/(Rev|[DM]rs?)\./g, "%$1%")
    // Split sentence boundaries
    .split(/(?:\.|\?|!)[’”"]?/)
    // Restore honorifics
    .map(sentence => sentence.replaceAll(/%([a-zA-Z]+)%/g, "$1.").trim())
    // Remove empty sentences
    .filter(sentence => sentence.length);
}

export function tokenizeSentence(sentence) {
  return sentence
    // Remove ASCII italics markup
    .replaceAll(/_/g, '')
    // Normalize whitespace
    .replaceAll(/\s+/g, ' ')
    // 1. "’Tis"
    // 2. "‘success,’"
    // 3. "call me ‘George’ now"
    // 4. "that is, no"
    .split(/(?:^’)|(?:[“,;]’”?)|(?:’ )|[—“”,‘;: ()]+/)
    .filter(word => word.length);
}

export function ngrams(tokens, minLength, maxLength) {
  const result = [];
  for (let i = minLength; i <= maxLength; i++) {
    for (let j = 0; j < tokens.length - i + 1; j++) {
      result.push(tokens.slice(j, j + i));
    }
  }
  return result;
}
