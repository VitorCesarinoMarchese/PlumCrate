export function currentSectionIndex(tops, readingLine, atEnd) {
  if (atEnd) return tops.length - 1;
  let current = 0;
  for (let index = 0; index < tops.length; index += 1) {
    if (tops[index] > readingLine) break;
    current = index;
  }
  return current;
}
